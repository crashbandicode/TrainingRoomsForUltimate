//! Diagnostic admission only. Addresses are 13.0.5 native return PCs, never pointers to dereference.

pub const CONSTRUCTOR: usize = 0x377c0d0;
pub const RETURNS: [usize; 2] = [0x1811408, 0x19d1480];
pub const LIMIT_PER_CALLER: usize = 8;

pub fn caller(base: usize, return_pc: usize) -> Option<usize> {
    if base == 0 { return None; }
    let offset = return_pc.checked_sub(base)?;
    RETURNS.iter().position(|&known| known == offset)
}

/// All windows must match before installing the observer. A missing row is a refusal.
pub fn admitted(matches: [bool; 3]) -> bool { matches.into_iter().all(|same| same) }

pub fn within_limit(previous: usize) -> bool { previous < LIMIT_PER_CALLER }
