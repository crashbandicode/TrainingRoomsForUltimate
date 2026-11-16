//! 13.0.5 owned articles, including fired weapons absent from the current ring.
//! Native facts: `artifacts/rc15-btt-projectile-hw2/NATIVE-OWNER.md` (ctor33b98b0, builder3a6438).
use super::native_fields as fields;

/// # Safety
/// The fighter accessor and its owned pool remain live during the game-thread callback.
pub(crate) unsafe fn read(owner: usize, base: usize) -> [Option<usize>; 64] {
    let empty = [None; 64];
    let Some(module) = fields::chain(owner, &[0x98]) else { return empty };
    if fields::read::<usize>(module, 0) != Some(base + 0x4f6a230) { return empty; }
    let Some(pool) = fields::child(module, 0x240) else { return empty };
    if fields::read::<usize>(pool, 0) != Some(base + 0x4f626e8) || fields::read::<u64>(pool, 8) != Some(64) { return empty; }
    let mut articles = fields::inline_elements::<64>(pool, 0x10);
    for slot in &mut articles {
        if slot.is_some_and(|at| ![0x5071a40, 0x5186a58].iter().any(|vt| fields::read::<usize>(at, 0) == Some(base + vt))) {
            *slot = None;
        }
    }
    articles
}
