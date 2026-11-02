//! Which fighter bodies the CPU behaviours act on, and the key of each one's own state.
//!
//! Decision B4 (SPEC-cpu-01 question 7): the CPU settings apply to the CPU role only, never to
//! entry 0, so a P1 driven by an AI (the standalone pilot, or the fallback) is never touched. The
//! role is the game's own operator flag (`FighterInformation::is_operation_cpu`) on an entry that is
//! not entry 0 and, in the waiting room, only the practice CPU's entry 4. Decision B6: one state per
//! fighter body, so Popo and Nana (one entry, two bodies) each have their own.

/// Where the behaviours are live (SPEC-cpu-01 "Where it is live").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    /// Native Training.
    Training,
    /// The waiting room's lobby practice: only entries 0 and 4 are serviced, and entry 0 is never a CPU.
    Lobby,
}

/// The practice CPU's entry in the waiting room.
pub const LOBBY_CPU_ENTRY: i32 = 4;

/// Whether a fighter on `entry`, operated by the CPU or not, is a CPU role for this scene.
pub const fn is_cpu_role(scene: Scene, entry: i32, operated_by_cpu: bool) -> bool {
    if !operated_by_cpu || entry <= 0 || entry >= 8 {
        return false;
    }
    match scene {
        Scene::Training => true,
        Scene::Lobby => entry == LOBBY_CPU_ENTRY,
    }
}

/// The key of one fighter body's state: its entry and whether it is the entry's second body (Nana).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FighterKey {
    pub entry: u8,
    pub partner: bool,
}

impl FighterKey {
    /// Bodies the state table holds: two per entry.
    pub const COUNT: usize = 16;

    pub fn new(entry: i32, partner: bool) -> Option<Self> {
        u8::try_from(entry).ok().filter(|e| *e < 8).map(|entry| Self { entry, partner })
    }

    pub const fn index(self) -> usize {
        self.entry as usize * 2 + self.partner as usize
    }
}

/// The module accessors of the CPU-role bodies the per-frame dispatch has seen, so a hook that gets
/// only an accessor decides "is this a CPU role, and whose state" by comparing pointers (no game call,
/// no allocation). The per-frame wrapper registers each CPU-role body on its own frame and forgets it
/// when it stops being one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleTable {
    slots: [(usize, Option<FighterKey>); FighterKey::COUNT],
}

impl Default for RoleTable {
    fn default() -> Self {
        Self::new()
    }
}

impl RoleTable {
    pub const fn new() -> Self {
        Self { slots: [(0, None); FighterKey::COUNT] }
    }

    /// Record that `accessor` is the body `key` (replacing an earlier record of either).
    pub fn set(&mut self, accessor: usize, key: FighterKey) {
        self.forget_key(key);
        self.forget(accessor);
        self.slots[key.index()] = (accessor, Some(key));
    }

    pub fn forget(&mut self, accessor: usize) {
        for slot in &mut self.slots {
            if slot.0 == accessor {
                *slot = (0, None);
            }
        }
    }

    pub fn forget_key(&mut self, key: FighterKey) {
        self.slots[key.index()] = (0, None);
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Whether any body is recorded.
    pub fn any(&self) -> bool {
        self.slots.iter().any(|slot| slot.0 != 0)
    }

    /// The key of the body whose accessor is `accessor`.
    pub fn find(&self, accessor: usize) -> Option<FighterKey> {
        if accessor == 0 {
            return None;
        }
        self.slots.iter().find(|slot| slot.0 == accessor).and_then(|slot| slot.1)
    }

    /// The accessor recorded for `key`, if any.
    pub fn accessor_of(&self, key: FighterKey) -> Option<usize> {
        Some(self.slots[key.index()].0).filter(|a| *a != 0)
    }
}
