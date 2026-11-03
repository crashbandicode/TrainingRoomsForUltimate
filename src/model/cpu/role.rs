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
        u8::try_from(entry)
            .ok()
            .filter(|e| *e < 8)
            .map(|entry| Self { entry, partner })
    }

    pub const fn index(self) -> usize {
        self.entry as usize * 2 + self.partner as usize
    }
}

/// Receipt taken before advancing a live native fighter. Destruction or scene
/// retirement invalidates it even when the allocator reuses every address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameReceipt {
    entry: u8,
    scene: Scene,
    scene_generation: u64,
    entry_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Registration {
    accessor: usize,
    key: FighterKey,
    receipt: FrameReceipt,
}

/// Current CPU registrations, owned by native entry/scene lifetime callbacks.
/// Accessors are equality tokens until a consumer separately resolves a live body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleTable {
    slots: [Option<Registration>; FighterKey::COUNT],
    scene: Option<Scene>,
    scene_generation: Option<u64>,
    entry_generations: [Option<u64>; 8],
}

impl Default for RoleTable {
    fn default() -> Self {
        Self::new()
    }
}

impl RoleTable {
    pub const fn new() -> Self {
        Self {
            slots: [None; FighterKey::COUNT],
            scene: None,
            scene_generation: Some(0),
            entry_generations: [Some(0); 8],
        }
    }

    /// Reconcile BEFORE native callbacks as well as inside every role consumer.
    /// A busy/non-practice interval ends the prior scope; reentry needs new frames.
    pub fn observe_scene(&mut self, scene: Option<Scene>) -> bool {
        if self.scene == scene {
            return false;
        }
        self.clear();
        self.scene = scene;
        true
    }

    pub fn receipt(&self, entry: i32) -> Option<FrameReceipt> {
        let entry = u8::try_from(entry).ok().filter(|entry| *entry < 8)?;
        Some(FrameReceipt {
            entry,
            scene: self.scene?,
            scene_generation: self.scene_generation?,
            entry_generation: self.entry_generations[usize::from(entry)]?,
        })
    }

    pub fn current(&self, receipt: FrameReceipt) -> bool {
        self.scene == Some(receipt.scene)
            && self.scene_generation == Some(receipt.scene_generation)
            && self.entry_generations[usize::from(receipt.entry)] == Some(receipt.entry_generation)
    }

    /// Publish only a frame still owned after native callbacks have completed.
    pub fn set(&mut self, receipt: FrameReceipt, accessor: usize, key: FighterKey) -> bool {
        if accessor == 0
            || key.entry != receipt.entry
            || !self.current(receipt)
            || !is_cpu_role(receipt.scene, i32::from(key.entry), true)
        {
            return false;
        }
        self.forget_key(key);
        self.forget(accessor);
        self.slots[key.index()] = Some(Registration {
            accessor,
            key,
            receipt,
        });
        true
    }

    pub fn forget(&mut self, accessor: usize) {
        for slot in &mut self.slots {
            if slot.is_some_and(|role| role.accessor == accessor) {
                *slot = None;
            }
        }
    }

    pub fn forget_key(&mut self, key: FighterKey) {
        self.slots[key.index()] = None;
    }

    /// Owner id, not an entry pointer. Both paired bodies retire before free.
    pub fn retire_entry(&mut self, entry: u32) {
        if entry >= 8 {
            return;
        }
        let index = entry as usize;
        self.entry_generations[index] =
            self.entry_generations[index].and_then(|g| g.checked_add(1));
        self.slots[index * 2..index * 2 + 2].fill(None);
    }

    /// Never reset generation counters on session cleanup (that would permit ABA).
    pub fn clear(&mut self) {
        self.slots.fill(None);
        self.scene_generation = self.scene_generation.and_then(|g| g.checked_add(1));
    }

    /// Whether any body is recorded.
    pub fn any(&self) -> bool {
        self.slots
            .iter()
            .flatten()
            .any(|role| self.current(role.receipt))
    }

    /// The key of the body whose accessor is `accessor`.
    pub fn find(&self, scene: Option<Scene>, accessor: usize) -> Option<FighterKey> {
        if accessor == 0 {
            return None;
        }
        self.slots
            .iter()
            .flatten()
            .find(|role| {
                role.accessor == accessor
                    && scene == Some(role.receipt.scene)
                    && self.current(role.receipt)
            })
            .map(|role| role.key)
    }

    /// The accessor recorded for `key`, if any.
    pub fn accessor_of(&self, scene: Option<Scene>, key: FighterKey) -> Option<usize> {
        let role = self.slots[key.index()]?;
        (scene == Some(role.receipt.scene) && self.current(role.receipt)).then_some(role.accessor)
    }
}
