//! Local build shim for the BTT-owned shared camera seam. No legacy apply(bool)
//! fallback: it would restore/reacquire every P1 pass. Replace with direct
//! btt_camera::request delegation when the primary forwards the BTT commit.
use std::sync::OnceLock;
type Request = unsafe fn(u8, bool);
static SHARED: OnceLock<Request> = OnceLock::new();
pub fn ready() -> bool {
    SHARED.get().is_some()
}
pub unsafe fn request(owner_bit: u8, want: bool) {
    if let Some(request) = SHARED.get() {
        request(owner_bit, want);
    }
}
