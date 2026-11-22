//! Native fighter form groups.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Forms {
    pub count: u32,
    pub kinds: [u32; 3],
    pub actor_count: u32,
    pub flags: u8,
}

impl Forms {
    /// Matches the native descriptor expansion. The caller admits catalog kinds.
    pub fn starting(kind: u32) -> Self {
        let (count, kinds, flags) = match kind {
            0x24 => (3, [0x24, 0x25, 0x26], 0x05),
            0x25 => (3, [0x25, 0x26, 0x24], 0x05),
            0x26 => (3, [0x26, 0x24, 0x25], 0x05),
            0x5a => (2, [0x5a, 0x5b, u32::MAX], 0x64),
            0x5b => (2, [0x5b, 0x5a, u32::MAX], 0x64),
            0x4b => (2, [0x4b, 0x4c, u32::MAX], 0x02),
            _ => (1, [kind, u32::MAX, u32::MAX], 0),
        };
        Self {
            count,
            kinds,
            actor_count: if kind == 0x4b { 2 } else { 1 },
            flags,
        }
    }

    pub fn index_of(self, kind: u32) -> Option<u8> {
        self.kinds[..self.count as usize]
            .iter()
            .position(|k| *k == kind)
            .map(|i| i as u8)
    }

    pub fn same_group(self, kind: u32) -> bool {
        self.index_of(kind).is_some()
    }
}
