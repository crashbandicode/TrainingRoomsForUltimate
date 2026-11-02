//! The one clock every log line uses: milliseconds since the console booted, from the system tick
//! (`std::time::SystemTime` reads 0 on the Switch build, so it cannot be used).

#[cfg(target_os = "switch")]
pub fn uptime_ms() -> u64 {
    // SAFETY: `GetSystemTick` has no preconditions.
    let tick = unsafe { skyline::nn::os::GetSystemTick() };
    crate::model::install_line::tick_to_ms(tick as u64)
}

/// Host fallback (tests): milliseconds since the first call.
#[cfg(not(target_os = "switch"))]
pub fn uptime_ms() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64
}
