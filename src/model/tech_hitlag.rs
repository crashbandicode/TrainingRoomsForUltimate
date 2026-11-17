//! Native-order hitlag arithmetic. Input is BASE attack damage, before the percent bonus.
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub maximum: f32,
    pub multiplier: f32,
    pub add: f32,
    pub threshold: i32,
    pub object_multiplier: f32,
    /// Live G.f0, validated here but used only by the separate percent conversion.
    pub percent_multiplier: f32,
}
fn positive_floor(value: f32) -> Option<i32> {
    (value.is_finite() && (1.0..2147483648.0).contains(&value))
        .then(|| value.floor() as i32)
}
/// Product AttackData has hitlag multiplier 1.0; the argument also permits owned replay probes.
pub fn frames(base_damage: f32, attack_multiplier: f32, p: Parameters) -> Option<i32> {
    if !base_damage.is_finite() || base_damage <= 0.0
        || !attack_multiplier.is_finite() || attack_multiplier <= 0.0
        || !p.maximum.is_finite() || p.maximum <= 0.0
        || !p.multiplier.is_finite() || !p.add.is_finite() || p.threshold <= 0
        || !p.object_multiplier.is_finite() || p.object_multiplier <= 0.0
        || !p.percent_multiplier.is_finite() || p.percent_multiplier <= 0.0
    { return None; }
    // Separate f32 operations match FMUL/FADD; no fused multiply-add or f64 intermediates.
    let scaled = p.multiplier * base_damage;
    let added = scaled + p.add;
    let raw = attack_multiplier * added;
    let capped = if raw > p.maximum { p.maximum } else { raw };
    let mut result = positive_floor(capped)?;
    if result >= p.threshold {
        let ratio = (p.maximum - result as f32) / (p.maximum - p.threshold as f32);
        // Native FMIN propagates NaN, then FMAXNM selects numeric zero.
        let t = if ratio.is_nan() || ratio <= 0.0 { 0.0 }
            else if ratio > 1.0 { 1.0 } else { ratio };
        let a = 1.0 - t;
        let b = 1.0 - a;
        let c = p.object_multiplier * a;
        let multiplier = b + c;
        result = positive_floor(multiplier * result as f32)?;
    }
    Some(result)
}

/// One off-hook provider binding attempt; a conflicting prebinding stays unavailable.
#[derive(Default)]
pub struct Binding(std::sync::OnceLock<Result<(), ()>>);
impl Binding {
    pub const fn new() -> Self { Self(std::sync::OnceLock::new()) }
    pub fn ready<E>(&self, bind: impl FnOnce() -> Result<(), E>) -> bool {
        self.0.get_or_init(|| bind().map_err(|_| ())).is_ok()
    }
}
