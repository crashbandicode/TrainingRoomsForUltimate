//! Stage Builder stages in the stage lists (rc.14): the save's custom stages follow the 117 catalog stages as extra
//! entries, in the game's order, and the kept rotation order names them by their save file id (`stage_<id>.bin`), so a
//! stage deleted in Stage Builder leaves every list and every saved order. Pure: the runtime reads the save's list
//! and hands it in. Native facts: native stage kind 325 (`SP_Edit`), one form; `artifacts/rc14-custom-stages/DESIGN.md`.

use serde::{Deserialize, Serialize};

use crate::engine::stages::lobby_stages::{LobbyStageOrder, STAGES};

/// The native stage kind every custom stage loads as (`SP_Edit`, row 325 of the 13.0.5 stage table).
pub const CUSTOM_STAGE_KIND: u32 = 325;
/// The stage the lists fall back to when a custom stage is gone (the Waiting Room, the lists' default).
pub const FALLBACK_STAGE: u32 = 311;

/// One Stage Builder stage the save holds: its file id and its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomStage {
    pub id: u32,
    pub name: String,
}

/// A stage in a list: a catalog stage (its base id) or a custom stage (its save file id).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum StageRef {
    #[serde(rename = "stage")]
    Catalog(u32),
    Custom(u32),
}

/// What grid tile `tile` shows: tile 0 is All (`None`), tiles 1..=117 the catalog, then one tile per custom stage.
pub fn tile_ref(tile: usize, customs: &[CustomStage]) -> Option<StageRef> {
    let index = tile.checked_sub(1)?;
    if let Some((id, _)) = STAGES.get(index) {
        return Some(StageRef::Catalog(*id));
    }
    customs.get(index - STAGES.len()).map(|stage| StageRef::Custom(stage.id))
}

/// The grid tile of a stage (the inverse of `tile_ref`); `None` for a custom stage the save no longer holds.
pub fn tile_of(stage: StageRef, customs: &[CustomStage]) -> Option<usize> {
    match stage {
        StageRef::Catalog(id) => STAGES.iter().position(|(s, _)| *s == id).map(|i| i + 1),
        StageRef::Custom(id) => customs.iter().position(|s| s.id == id).map(|i| STAGES.len() + 1 + i),
    }
}

/// Tiles of the Lobby Stages grid (All, the catalog, the custom stages).
pub fn tile_count(customs: &[CustomStage]) -> usize {
    1 + STAGES.len() + customs.len()
}

/// The name a list shows for a custom stage the save holds.
pub fn name_of(id: u32, customs: &[CustomStage]) -> Option<&str> {
    customs.iter().find(|s| s.id == id).map(|s| s.name.as_str())
}

/// The ordered stage list the waiting rooms and Training Stage Rotation move through: All, or the stages in the order
/// they were added. Kept under `lobby_stage_order`; `lobby_stages` keeps the catalog projection for older builds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageOrder {
    pub all: bool,
    pub entries: Vec<StageRef>,
    /// Lobby Stages' Direct choice when it is a custom stage (it then wins over `stage_preferences.direct_stage`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct_custom: Option<u32>,
    /// The names of the custom stages the order or the Direct choice names, as the save last had them: a stage deleted in
    /// Stage Builder is gone from the save, and its notice still names it.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub names: std::collections::BTreeMap<u32, String>,
}

impl Default for StageOrder {
    fn default() -> Self {
        Self { all: false, entries: vec![StageRef::Catalog(FALLBACK_STAGE)], direct_custom: None, names: Default::default() }
    }
}

impl StageOrder {
    /// The order an older settings file holds (catalog stages only).
    pub fn from_legacy(order: LobbyStageOrder) -> Self {
        let stages = order.stages();
        if stages.len() == STAGES.len() && order == all_catalog() {
            return Self { all: true, entries: Vec::new(), ..Self::default() };
        }
        Self { entries: stages.into_iter().map(StageRef::Catalog).collect(), ..Self::default() }
    }

    /// The kept order: `lobby_stage_order` when it is valid and still projects to the legacy `lobby_stages` (a page reset, a
    /// diag reload or an older build may have changed `lobby_stages` alone), else the legacy order.
    pub fn read(order: Option<&serde_json::Value>, legacy: LobbyStageOrder) -> Self {
        order.and_then(|v| serde_json::from_value::<Self>(v.clone()).ok())
            .filter(|o| o.valid() && o.legacy() == legacy)
            .unwrap_or_else(|| Self::from_legacy(legacy))
    }

    /// The custom stages the cycles may go to, which the runtime reads ahead from the save: the order's (every one the
    /// save holds while All is on) and the Direct choice, each once, in that order.
    pub fn custom_ids(&self, customs: &[CustomStage]) -> Vec<u32> {
        let mut ids: Vec<u32> = self.stages(customs).into_iter()
            .filter_map(|s| match s { StageRef::Custom(id) => Some(id), StageRef::Catalog(_) => None }).collect();
        if let Some(id) = self.direct_custom.filter(|id| customs.iter().any(|s| s.id == *id) && !ids.contains(id)) {
            ids.push(id);
        }
        ids
    }

    fn valid(&self) -> bool {
        let mut seen = std::collections::HashSet::new();
        (self.all || !self.entries.is_empty())
            && self.entries.iter().all(|e| seen.insert(*e) && match e {
                StageRef::Catalog(id) => STAGES.iter().any(|(s, _)| s == id),
                StageRef::Custom(_) => true,
            })
    }

    /// Drop the custom stages the save no longer holds, closing the gaps. An order left empty is the default.
    /// Returns whether anything changed.
    pub fn prune(&mut self, customs: &[CustomStage]) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| match e {
            StageRef::Catalog(_) => true,
            StageRef::Custom(id) => customs.iter().any(|s| s.id == *id),
        });
        let changed = self.entries.len() != before;
        if !self.all && self.entries.is_empty() {
            self.entries = Self::default().entries;
        }
        changed
    }

    /// The Direct custom stage when the save no longer holds it: cleared, and its name returned for the notice
    /// (`missing_notice`). The Direct choice is then the catalog stage of `stage_preferences` again.
    pub fn take_missing_direct(&mut self, customs: &[CustomStage]) -> Option<String> {
        let id = self.direct_custom.filter(|id| !customs.iter().any(|s| s.id == *id))?;
        self.direct_custom = None;
        let name = self.names.get(&id).cloned().unwrap_or_else(|| format!("stage {id}"));
        self.names.remove(&id);
        Some(name)
    }

    /// Keep the names of the custom stages the order or the Direct choice names, from the save's current list; names of
    /// stages nothing names any more are dropped. Returns whether anything changed.
    pub fn remember(&mut self, customs: &[CustomStage]) -> bool {
        let before = self.names.clone();
        let named: Vec<u32> = self.entries.iter().filter_map(|e| match e { StageRef::Custom(id) => Some(*id), _ => None })
            .chain(self.direct_custom).collect();
        self.names.retain(|id, _| named.contains(id));
        for s in customs.iter().filter(|s| named.contains(&s.id)) {
            self.names.insert(s.id, s.name.clone());
        }
        self.names != before
    }

    /// The place (1-based) of a stage in the order; none while All is on.
    pub fn rank(&self, stage: StageRef) -> Option<usize> {
        if self.all { return None; }
        self.entries.iter().position(|e| *e == stage).map(|i| i + 1)
    }

    /// A on a stage: add it at the end, or take it out (the last one stays). From All, a pick starts a new order with
    /// just that stage. Returns whether the order changed.
    pub fn toggle(&mut self, stage: StageRef) -> bool {
        if self.all {
            self.all = false;
            self.entries = vec![stage];
            return true;
        }
        match self.entries.iter().position(|e| *e == stage) {
            Some(_) if self.entries.len() == 1 => false,
            Some(i) => { self.entries.remove(i); true }
            None => { self.entries.push(stage); true }
        }
    }

    /// A on All: every stage.
    pub fn select_all(&mut self) -> bool {
        if self.all { return false; }
        self.all = true;
        self.entries.clear();
        true
    }

    /// The stages in order: All is every catalog stage, then every custom stage the save holds, in the game's order.
    pub fn stages(&self, customs: &[CustomStage]) -> Vec<StageRef> {
        if self.all {
            return STAGES.iter().map(|(id, _)| StageRef::Catalog(*id))
                .chain(customs.iter().map(|s| StageRef::Custom(s.id))).collect();
        }
        self.entries.iter().copied().filter(|e| match e {
            StageRef::Catalog(_) => true,
            StageRef::Custom(id) => customs.iter().any(|s| s.id == *id),
        }).collect()
    }

    /// The legacy `lobby_stages` projection: the catalog entries in order (All stays All). An order of custom stages
    /// only projects to the default, so an older build reads a valid order.
    pub fn legacy(&self) -> LobbyStageOrder {
        if self.all { return all_catalog(); }
        let mut values = vec![0u8; STAGES.len() + 1];
        let mut next = 0u8;
        for e in &self.entries {
            if let StageRef::Catalog(id) = e {
                if let Some(i) = STAGES.iter().position(|(s, _)| s == id) {
                    next = next.saturating_add(1);
                    values[i + 1] = next;
                }
            }
        }
        if next == 0 { return LobbyStageOrder::NATIVE; }
        serde_json::from_value(serde_json::json!(values)).unwrap_or(LobbyStageOrder::NATIVE)
    }
}

fn all_catalog() -> LobbyStageOrder {
    let mut values = vec![0u8; STAGES.len() + 1];
    values[0] = 1;
    serde_json::from_value(serde_json::json!(values)).unwrap_or(LobbyStageOrder::NATIVE)
}

/// Where a Direct choice goes: the custom stage when the save still holds it; otherwise the fallback stage and the
/// name of the stage that is gone, for the notice ("Custom stage missing: ‹name›").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolved {
    Stage(StageRef),
    Missing { fallback: StageRef, name: String },
}

/// Resolve a stage the lists or a kept preference name against the save's current list. `known_name` is the name the
/// preference kept when it was chosen (the save no longer has it once the stage is deleted).
pub fn resolve(stage: StageRef, customs: &[CustomStage], known_name: Option<&str>) -> Resolved {
    match stage {
        StageRef::Custom(id) if !customs.iter().any(|s| s.id == id) => Resolved::Missing {
            fallback: StageRef::Catalog(FALLBACK_STAGE),
            name: known_name.filter(|n| !n.is_empty()).map_or_else(|| format!("stage {id}"), str::to_owned),
        },
        _ => Resolved::Stage(stage),
    }
}

/// The notice for a custom stage that is gone.
pub fn missing_notice(name: &str) -> String {
    // restriction: custom-stage-missing
    format!("Custom stage missing: {name}")
}

/// A custom stage in a `Selection` (the stage cycles and the practice requests carry `Selection`s): the save file id with
/// this bit set. `lobby_stages::native_stage` maps it to `CUSTOM_STAGE_KIND`; the runtime loads that save file first.
pub const CUSTOM_FLAG: u32 = 0x8000_0000;

/// Save file ids are below this (the game counts them up from 1; 13448 after one stage on the test save).
const ID_LIMIT: u32 = 0x0100_0000;

/// The `Selection::stage` that stands for the custom stage with save file id `id` (`id` below `ID_LIMIT`).
pub const fn encode(id: u32) -> u32 {
    CUSTOM_FLAG | (id % ID_LIMIT)
}

/// The save file id of an encoded custom `Selection::stage`; `None` for a catalog stage or anything else.
pub const fn decode(stage: u32) -> Option<u32> {
    if stage & 0xff00_0000 == CUSTOM_FLAG { Some(stage % ID_LIMIT) } else { None }
}

/// The size of a 13.0.5 Stage Builder save file (`save:/save_data/stage/stage_<id>.bin`, one fixed-size record).
pub const STAGE_FILE_LEN: usize = 0x32198;
/// Where the record keeps its id (u32), its name (UTF-16, NUL-terminated, up to the music field at 0x13c) and the
/// length (u32) of the layout data that starts at 0x198 (what the game's loader `0x17d0120` copies into the rule).
const ID_AT: usize = 0;
const NAME_AT: usize = 0x110;
const NAME_END: usize = 0x13c;
const DATA_LEN_AT: usize = 0x15c;
const DATA_AT: usize = 0x198;

/// The save file id in a file name (`stage_13448.bin` → 13448).
pub fn file_id(file_name: &str) -> Option<u32> {
    let id = file_name.strip_prefix("stage_")?.strip_suffix(".bin")?;
    (!id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())).then(|| id.parse().ok()).flatten()
        .filter(|id: &u32| *id < ID_LIMIT)
}

/// The id and name of a Stage Builder save file, when the bytes are a whole record whose id matches its file name and
/// whose layout data fits; `None` otherwise (the stage is left out of the lists, never loaded).
pub fn parse_stage_file(bytes: &[u8], file_id: u32) -> Option<CustomStage> {
    if bytes.len() != STAGE_FILE_LEN {
        return None;
    }
    parse_stage_header(&bytes[..HEADER_LEN], bytes.len() as u64, file_id).map(|(stage, _)| stage)
}

/// The record's header: everything before the layout data. The list reads only this much of each file.
pub const HEADER_LEN: usize = DATA_AT;

/// `parse_stage_file` from the header alone and the file's length; also gives the record's length up to the end of its
/// layout data (what the loader reads: `HEADER_LEN` + the data length).
pub fn parse_stage_header(header: &[u8], file_len: u64, file_id: u32) -> Option<(CustomStage, usize)> {
    if header.len() < HEADER_LEN || file_len != STAGE_FILE_LEN as u64 {
        return None;
    }
    let u32_at = |at: usize| u32::from_le_bytes([header[at], header[at + 1], header[at + 2], header[at + 3]]);
    let id = u32_at(ID_AT);
    let used = DATA_AT.checked_add(u32_at(DATA_LEN_AT) as usize)?;
    if id != file_id || used > STAGE_FILE_LEN {
        return None;
    }
    let units: Vec<u16> = header[NAME_AT..NAME_END].as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c))
        .take_while(|u| *u != 0).collect();
    let name = String::from_utf16(&units).ok().filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| format!("Custom stage {id}"));
    Some((CustomStage { id, name }, used))
}

/// The byte of a record the game's setter copies into a stage-325 descriptor at +0x98 (the loader puts record byte 0x138
/// into the rule at `0x530a819`; the setter `0x178a9c0` copies that rule byte for 325).
pub const SAVE_BYTE_AT: usize = 0x138;

impl StageOrder {
    /// The order as the stage cycles take it: catalog stages with their form (`forms` by catalog index), custom stages
    /// encoded (`encode`). Custom stages the save no longer holds are left out.
    pub fn selections(&self, customs: &[CustomStage], forms: &crate::engine::stages::stage_forms::Forms)
        -> Vec<crate::engine::stages::stage_forms::Selection> {
        use crate::engine::stages::stage_forms::{Form, Selection};
        self.stages(customs).into_iter().map(|s| match s {
            StageRef::Catalog(id) => {
                let form = STAGES.iter().position(|(s, _)| *s == id).and_then(|i| forms.get(i)).unwrap_or(Form::Normal);
                Selection { stage: id, form }
            }
            StageRef::Custom(id) => Selection { stage: encode(id), form: Form::Normal },
        }).collect()
    }
}

/// The save's custom stages and the kept order, as the menu worker last read them (`runtime::lobby::custom_stages`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Published {
    pub order: StageOrder,
    pub customs: Vec<CustomStage>,
}

/// What the stage cycles move through: the rotation in order and the Direct choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lists {
    pub rotation: Vec<crate::engine::stages::stage_forms::Selection>,
    pub direct: crate::engine::stages::stage_forms::Selection,
    pub direct_mode: bool,
    /// The Direct custom stage the save no longer holds (its name, for `missing_notice`): `direct` is then the catalog
    /// stage of `stage_preferences`.
    pub missing: Option<String>,
}

/// The cycles' lists from the typed menu's order and preferences, with the save's custom stages when the worker has
/// published them for this order (`published.order` projects to `legacy`); otherwise the catalog lists as before.
pub fn lists(legacy: LobbyStageOrder, prefs: &crate::engine::stages::stage_menu::Preferences, published: Option<&Published>)
    -> Result<Lists, &'static str> {
    use crate::engine::stages::stage_forms::{Form, Selection};
    crate::engine::stages::lobby_stages::validate_preferences(prefs)?;
    let catalog_direct = Selection { stage: prefs.direct_stage, form: prefs.direct_form };
    let direct_mode = prefs.mode == crate::engine::stages::stage_menu::Mode::Direct;
    let legacy_rotation = legacy.selections(&prefs.rotation_forms)?;
    let Some(published) = published.filter(|p| p.order.legacy() == legacy) else {
        return Ok(Lists { rotation: legacy_rotation, direct: catalog_direct, direct_mode, missing: None });
    };
    let mut rotation = published.order.selections(&published.customs, &prefs.rotation_forms);
    if rotation.is_empty() {
        // Every custom stage of the order is gone: the pruned order's default, which the legacy projection is.
        rotation = legacy_rotation;
    }
    let held = |id: &u32| published.customs.iter().any(|s| s.id == *id);
    let (direct, missing) = match published.order.direct_custom {
        Some(id) if held(&id) => (Selection { stage: encode(id), form: Form::Normal }, None),
        Some(id) => (catalog_direct, Some(published.order.names.get(&id).cloned().unwrap_or_else(|| format!("stage {id}")))),
        None => (catalog_direct, None),
    };
    Ok(Lists { rotation, direct, direct_mode, missing })
}

/// The rule slot fields the game's own load (`0x2317840` around the loader `0x17d0120`) writes: the u16 at +0, the u32
/// at +4 and the u16 at +8 of rule `r` (`0x530a818 + r*0x40`), its two layout vectors (+0x10 and +0x28: begin, end,
/// capacity end) and the custom stage id (`0x530a8fc + r*0x30`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuleSlot {
    pub head: u16,
    pub word: u32,
    pub tail: u16,
    pub data: [u64; 3],
    pub extra: [u64; 3],
    pub id: u32,
}

impl RuleSlot {
    /// The game allocations the slot holds (vector starts; 0 is none).
    const fn buffers(&self) -> [u64; 2] {
        [self.data[0], self.extra[0]]
    }

    fn holds(&self, p: u64) -> bool {
        self.data[0] == p || self.extra[0] == p
    }
}

/// One step for the runtime: the slot to write (rule index, value), then the game allocations to free (`free`, 0 is
/// none). Nothing the rule holds once the write is done is ever freed. Fixed size: a departure runs inside a hook
/// (no heap in hooks).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RulePlan {
    pub write: Option<(u32, RuleSlot)>,
    pub free: [u64; 4],
}

impl RulePlan {
    /// The allocations to free, in order.
    pub fn freed(&self) -> impl Iterator<Item = u64> + '_ {
        self.free.iter().copied().filter(|p| *p != 0)
    }
}

/// TR's hold on the rule while a custom stage it loaded is there. The game frees a slot's old vectors when it writes the
/// slot (`0x2317840` and the room writers), so TR frees only what it took out of the rule itself, never what the rule
/// holds, and only once the stage that could read it is gone (the next load or departure). No heap.
#[derive(Debug, Default)]
pub struct RuleOwner {
    /// The rule as it was before TR's first load, and its index: written back when the practice scene ends.
    original: Option<(u32, RuleSlot)>,
    /// What TR wrote last: the rule still holds it while nothing else wrote the slot.
    installed: Option<RuleSlot>,
    /// Buffers TR took out of the rule (the stage on screen may still read them): freed at the next step. 0 is none.
    retired: [u64; 2],
}

impl RuleOwner {
    pub const fn new() -> Self {
        Self { original: None, installed: None, retired: [0; 2] }
    }

    /// The rule index TR's load is in, while TR holds the rule.
    pub fn index(&self) -> Option<u32> {
        self.original.map(|(index, _)| index)
    }

    /// Whether rule `index` (now `current`) still holds TR's load of custom stage `id`: nothing to load again.
    pub fn holds(&self, index: u32, current: RuleSlot, id: u32) -> bool {
        self.index() == Some(index) && self.installed == Some(current) && current.id == id
    }

    /// TR loads `loaded` into rule `index`, which holds `current` now. A different rule index than the one TR holds is
    /// departed from first (`depart`, by the runtime).
    pub fn load(&mut self, index: u32, current: RuleSlot, loaded: RuleSlot) -> RulePlan {
        // Taken out of the rule at the last step: the stage that read them has been replaced since.
        let retired = std::mem::take(&mut self.retired);
        let mut stale = [0; 2];
        match self.original {
            Some((at, _)) if at == index && self.installed == Some(current) => {
                // TR's last load is still there: out of the rule now, kept until the next step.
                self.retired = current.buffers();
            }
            Some((_, original)) => {
                // The game wrote the slot since (it freed TR's load then): its data is the original now, and the one TR
                // kept is nobody's.
                stale = original.buffers();
                self.original = Some((index, current));
            }
            None => self.original = Some((index, current)),
        }
        self.installed = Some(loaded);
        Self::plan(Some((index, loaded)), retired, stale, [current, loaded])
    }

    /// The practice scene is gone: the rule gets its original back while it still holds TR's load; otherwise the game
    /// wrote the slot, and the original TR kept is freed.
    pub fn depart(&mut self, current: RuleSlot) -> RulePlan {
        let Some((index, original)) = self.original.take() else { return RulePlan::default() };
        let retired = std::mem::take(&mut self.retired);
        let mut stale = [0; 2];
        let write = if self.installed.take() == Some(current) {
            self.retired = current.buffers();
            Some((index, original))
        } else {
            stale = original.buffers();
            None
        };
        Self::plan(write, retired, stale, [current, write.map_or(current, |(_, slot)| slot)])
    }

    /// Never free what the rule holds or is about to hold, nor anything twice.
    fn plan(write: Option<(u32, RuleSlot)>, retired: [u64; 2], stale: [u64; 2], held: [RuleSlot; 2]) -> RulePlan {
        let mut free = [0u64; 4];
        let mut n = 0;
        for p in retired.into_iter().chain(stale) {
            if p != 0 && !held.iter().any(|slot| slot.holds(p)) && !free[..n].contains(&p) {
                free[n] = p;
                n += 1;
            }
        }
        RulePlan { write, free }
    }
}

/// A saved stage's name as the game keeps it (UTF-16, NUL-terminated, 22 units at most in the header's 0x2c bytes).
pub fn name_from_units(id: u32, units: &[u16]) -> String {
    let end = units.iter().position(|u| *u == 0).unwrap_or(units.len());
    String::from_utf16(&units[..end]).ok().filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| format!("Custom stage {id}"))
}

/// What one frame of the stage save object shows the read (MainThread, read before each step).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observed {
    /// The storage `H+0x68`: 0 while no save operation runs.
    pub storage: usize,
    /// The game's own idle condition: the storage null, or its state 0 and the busy word 0.
    pub idle: bool,
    /// Training or the waiting-room practice: the only scenes TR starts a read in.
    pub in_scope: bool,
}

/// What a step asks of the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadAction {
    None,
    /// Queue the id and begin the read (`0x17cfd40`, `0x17cfeb0`); report the storage it made with `started`.
    Start(u32),
    /// The storage is TR's and idle: clean it up (`0x17cfbc0(H+0x68)`).
    Cleanup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReadState {
    Idle,
    Reading { id: u32, storage: usize, frames: u32 },
    Ready { id: u32, storage: usize },
    /// Gave up waiting: cleaned up once the storage is idle, if it is still TR's.
    Abandoned { storage: usize },
}

/// One stage read through the game's own save object, at most one at a time (the approved route): start only in scope,
/// with no game operation running; a storage the game replaced is dropped without cleanup; a read that does not finish
/// in `READ_FRAMES` is abandoned (cleaned up only once idle and still TR's); a finished read waits, idle, for its load.
#[derive(Debug)]
pub struct ReadMachine {
    state: ReadState,
    wanted: Option<u32>,
    /// The id a read of this want already failed for: not tried again until the want changes.
    failed: Option<u32>,
    /// The stage whose loaded data TR already holds (built right after its read): not read again until it is used.
    have: Option<u32>,
    /// Want checks in a row that named another stage than `have` (or none).
    stale: u8,
}

/// Frames a read may take before TR gives up on it (10 s at 60 fps).
pub const READ_FRAMES: u32 = 600;

/// Want checks in a row that name another stage than the one TR holds data for (or none) before that data is freed. One
/// check can miss the next entry while a list is busy (`peek_next` gives none on a held lock); two cannot.
pub const STALE_CHECKS: u8 = 2;

impl Default for ReadMachine {
    fn default() -> Self { Self::new() }
}

impl ReadMachine {
    pub const fn new() -> Self {
        Self { state: ReadState::Idle, wanted: None, failed: None, have: None, stale: 0 }
    }

    /// The stage TR holds data for left the list (deleted) or is no longer next, on `STALE_CHECKS` want checks in a
    /// row: its id, once. The runtime frees that data; the stage is read again if it is wanted again.
    pub fn take_stale(&mut self) -> Option<u32> {
        if self.stale < STALE_CHECKS {
            return None;
        }
        self.stale = 0;
        self.have.take()
    }

    /// TR holds the loaded data of `id` now (or, `None`, it used it): a stage TR holds is not read again.
    pub fn have(&mut self, id: Option<u32>) {
        self.have = id;
        self.stale = 0;
    }

    /// The finished read held no record of its stage (deleted since): back to idle, not retried for this want.
    pub fn spoiled(&mut self) {
        if let ReadState::Ready { id, .. } = self.state {
            self.fail(id);
        }
        self.state = ReadState::Idle;
    }

    /// The stage TR will need next (a Change Stage pick, the next rotation or waiting-room entry), or none. Each call is
    /// one check for `take_stale`.
    pub fn want(&mut self, id: Option<u32>) {
        if self.wanted != id {
            self.wanted = id;
            self.failed = None;
        }
        self.stale = match self.have {
            Some(held) if id != Some(held) => self.stale.saturating_add(1),
            _ => 0,
        };
    }

    pub fn wanted(&self) -> Option<u32> {
        self.wanted
    }

    /// The read of `want` failed or was dropped (no read will start for it until the want changes).
    pub fn failed(&self) -> bool {
        self.failed.is_some() && self.failed == self.wanted
    }

    fn fail(&mut self, id: u32) {
        self.failed = Some(id);
    }

    /// One frame.
    pub fn step(&mut self, o: Observed) -> ReadAction {
        match self.state {
            ReadState::Reading { id, storage, frames } => {
                if o.storage != storage {
                    // The game replaced TR's storage with its own operation: nothing of TR's is left to clean up.
                    self.state = ReadState::Idle;
                    self.fail(id);
                } else if o.idle {
                    self.state = ReadState::Ready { id, storage };
                } else if frames + 1 >= READ_FRAMES {
                    self.state = ReadState::Abandoned { storage };
                    self.fail(id);
                } else {
                    self.state = ReadState::Reading { id, storage, frames: frames + 1 };
                }
                ReadAction::None
            }
            ReadState::Abandoned { storage } | ReadState::Ready { storage, .. } if o.storage != storage => {
                self.state = ReadState::Idle;
                ReadAction::None
            }
            ReadState::Abandoned { .. } if o.idle => {
                self.state = ReadState::Idle;
                ReadAction::Cleanup
            }
            ReadState::Abandoned { .. } => ReadAction::None,
            ReadState::Ready { id, .. } if self.wanted != Some(id) && o.idle => {
                self.state = ReadState::Idle;
                ReadAction::Cleanup
            }
            ReadState::Ready { .. } => ReadAction::None,
            ReadState::Idle => match self.wanted {
                Some(id) if o.in_scope && o.storage == 0 && o.idle && self.failed != Some(id) && self.have != Some(id) => {
                    self.state = ReadState::Reading { id, storage: 0, frames: 0 };
                    ReadAction::Start(id)
                }
                _ => ReadAction::None,
            },
        }
    }

    /// `Start` ran: the storage `H+0x68` now (0: the game made none, the read failed).
    pub fn started(&mut self, id: u32, storage: usize) {
        if storage == 0 {
            self.state = ReadState::Idle;
            self.fail(id);
        } else {
            self.state = ReadState::Reading { id, storage, frames: 0 };
        }
    }

    /// The record of `id` is in `H+0x70`, the storage is TR's and idle: the load may run.
    pub fn ready(&self, id: u32, o: Observed) -> bool {
        matches!(self.state, ReadState::Ready { id: ready, storage } if ready == id && o.storage == storage && o.idle)
    }

    /// The load ran and the runtime cleaned up (or the load found no record): back to idle.
    pub fn consumed(&mut self) {
        self.state = ReadState::Idle;
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn three() -> Vec<CustomStage> {
        vec![CustomStage { id: 7, name: "Ledge Lab".into() }, CustomStage { id: 12, name: "Flat".into() },
             CustomStage { id: 40, name: "Walls".into() }]
    }

    fn custom_stages_follow_the_catalog_as_extra_tiles() {
        assert_eq!(tile_count(&[]), 1 + STAGES.len());
        let customs = three();
        assert_eq!(tile_count(&customs), 1 + STAGES.len() + 3, "3 custom stages are 3 more tiles");
        assert_eq!(tile_ref(0, &customs), None, "tile 0 is All");
        assert_eq!(tile_ref(1, &customs), Some(StageRef::Catalog(STAGES[0].0)));
        assert_eq!(tile_ref(STAGES.len(), &customs), Some(StageRef::Catalog(STAGES[STAGES.len() - 1].0)));
        for (i, s) in customs.iter().enumerate() {
            let tile = STAGES.len() + 1 + i;
            assert_eq!(tile_ref(tile, &customs), Some(StageRef::Custom(s.id)), "custom {i} at the end, in the save's order");
            assert_eq!(tile_of(StageRef::Custom(s.id), &customs), Some(tile));
        }
        assert_eq!(tile_ref(STAGES.len() + 4, &customs), None, "past the last custom stage");
        assert_eq!(name_of(12, &customs), Some("Flat"));
    }

    fn deleting_a_custom_stage_prunes_the_order_and_closes_the_gaps() {
        let customs = three();
        let mut order = StageOrder::default();
        assert!(order.toggle(StageRef::Custom(12)));
        assert!(order.toggle(StageRef::Catalog(0)));
        assert!(order.toggle(StageRef::Custom(40)));
        assert_eq!(order.rank(StageRef::Custom(40)), Some(4));
        // Stage 12 deleted in Stage Builder: gone from the tiles and from the order; the places close up.
        let after: Vec<_> = customs.iter().filter(|s| s.id != 12).cloned().collect();
        assert_eq!(tile_count(&after), 1 + STAGES.len() + 2);
        assert_eq!(tile_of(StageRef::Custom(12), &after), None);
        assert!(order.prune(&after));
        assert_eq!(order.entries, vec![StageRef::Catalog(311), StageRef::Catalog(0), StageRef::Custom(40)]);
        assert_eq!(order.rank(StageRef::Custom(40)), Some(3));
        assert!(!order.prune(&after), "nothing more to prune");
        // An order of deleted custom stages only falls back to the default.
        let mut only = StageOrder { entries: vec![StageRef::Custom(7)], ..StageOrder::default() };
        assert!(only.prune(&[]));
        assert_eq!(only, StageOrder::default());
    }

    fn rotation_cycles_through_custom_entries_in_order() {
        let customs = three();
        let mut order = StageOrder::default();
        order.toggle(StageRef::Custom(40));
        order.toggle(StageRef::Catalog(3));
        assert_eq!(order.stages(&customs), vec![StageRef::Catalog(311), StageRef::Custom(40), StageRef::Catalog(3)]);
        // A stage deleted while the order is kept is skipped even before the order is pruned.
        let after: Vec<_> = customs.iter().filter(|s| s.id != 40).cloned().collect();
        assert_eq!(order.stages(&after), vec![StageRef::Catalog(311), StageRef::Catalog(3)]);
        // All: the catalog, then the custom stages, in the save's order.
        assert!(order.select_all());
        let all = order.stages(&customs);
        assert_eq!(all.len(), STAGES.len() + 3);
        assert_eq!(&all[STAGES.len()..], &[StageRef::Custom(7), StageRef::Custom(12), StageRef::Custom(40)]);
        // A pick from All starts a new order with just it; the last stage stays.
        assert!(order.toggle(StageRef::Custom(7)));
        assert_eq!(order.entries, vec![StageRef::Custom(7)]);
        assert!(!order.toggle(StageRef::Custom(7)), "the last one stays");
    }

    fn the_kept_order_round_trips_and_older_files_still_read() {
        let mut order = StageOrder::default();
        order.toggle(StageRef::Custom(12));
        order.toggle(StageRef::Catalog(0));
        let value = serde_json::to_value(&order).unwrap();
        assert_eq!(value, serde_json::json!({"all": false, "entries": [{"stage": 311}, {"custom": 12}, {"stage": 0}]}));
        assert_eq!(StageOrder::read(Some(&value), order.legacy()), order);
        // `lobby_stages` changed alone (Reset page, a diag reload, an older build): the legacy order wins.
        assert_eq!(StageOrder::read(Some(&value), LobbyStageOrder::NATIVE), StageOrder::default());
        // The menu's Reset page writes this form for the default (`Menu::kept_page_default`).
        assert_eq!(serde_json::to_value(StageOrder::default()).unwrap(),
            serde_json::json!({"all": false, "entries": [{"stage": 311}]}));
        // The legacy projection keeps the catalog stages in order, so an older build reads a valid order.
        assert_eq!(order.legacy().stages(), vec![311, 0]);
        // A file with only `lobby_stages` (no custom stages yet) reads as that order.
        let legacy: LobbyStageOrder = serde_json::from_str("[0,0,1,2]").unwrap();
        let read = StageOrder::read(None, legacy);
        assert_eq!(read.entries, legacy.stages().into_iter().map(StageRef::Catalog).collect::<Vec<_>>());
        // A malformed or duplicated kept order falls back to the legacy one.
        let bad = serde_json::json!({"all": false, "entries": [{"custom": 1}, {"custom": 1}]});
        assert_eq!(StageOrder::read(Some(&bad), legacy), read);
        let unknown = serde_json::json!({"all": false, "entries": [{"stage": 1}]});
        assert_eq!(StageOrder::read(Some(&unknown), legacy), read, "stage 1 is a form, not a catalog stage");
        // All round-trips both ways.
        let all = StageOrder { all: true, entries: vec![], ..StageOrder::default() };
        assert_eq!(StageOrder::from_legacy(all.legacy()), all);
    }

    fn a_missing_direct_custom_stage_falls_back_with_its_name() {
        let customs = three();
        assert_eq!(resolve(StageRef::Custom(7), &customs, Some("Ledge Lab")), Resolved::Stage(StageRef::Custom(7)));
        assert_eq!(resolve(StageRef::Catalog(0), &[], None), Resolved::Stage(StageRef::Catalog(0)));
        let gone = resolve(StageRef::Custom(99), &customs, Some("Old Walls"));
        assert_eq!(gone, Resolved::Missing { fallback: StageRef::Catalog(FALLBACK_STAGE), name: "Old Walls".into() });
        assert_eq!(missing_notice("Old Walls"), "Custom stage missing: Old Walls");
        // The Direct custom stage keeps its name while the save has it, and gives it back once the stage is deleted.
        let mut order = StageOrder { direct_custom: Some(40), ..StageOrder::default() };
        order.toggle(StageRef::Custom(12));
        assert!(order.remember(&customs));
        assert_eq!(order.names.get(&40).map(String::as_str), Some("Walls"));
        assert_eq!(order.names.get(&12).map(String::as_str), Some("Flat"));
        assert_eq!(order.take_missing_direct(&customs), None, "still in the save");
        let after: Vec<_> = customs.iter().filter(|s| s.id != 40).cloned().collect();
        assert_eq!(order.take_missing_direct(&after).as_deref(), Some("Walls"));
        assert_eq!(order.direct_custom, None, "the Direct choice is the catalog stage again");
        assert_eq!(order.take_missing_direct(&after), None, "one notice");
        let value = serde_json::to_value(&order).unwrap();
        assert_eq!(StageOrder::read(Some(&value), LobbyStageOrder::NATIVE), order, "the names round-trip");
        assert_eq!(resolve(StageRef::Custom(99), &customs, None),
            Resolved::Missing { fallback: StageRef::Catalog(FALLBACK_STAGE), name: "stage 99".into() });
    }

    fn the_save_files_give_their_id_and_name_and_bad_ones_are_left_out() {
        assert_eq!(file_id("stage_13448.bin"), Some(13448));
        for bad in ["stage_.bin", "stage_12a.bin", "mii_3.bin", "stage_3.bin.bak", "stage_4294967295.bin"] {
            assert_eq!(file_id(bad), None, "{bad}");
        }
        let mut record = vec![0u8; STAGE_FILE_LEN];
        record[..4].copy_from_slice(&13448u32.to_le_bytes());
        for (i, u) in "Ledge Lab".encode_utf16().enumerate() {
            record[NAME_AT + 2 * i..NAME_AT + 2 * i + 2].copy_from_slice(&u.to_le_bytes());
        }
        record[DATA_LEN_AT..DATA_LEN_AT + 4].copy_from_slice(&522u32.to_le_bytes());
        assert_eq!(parse_stage_file(&record, 13448), Some(CustomStage { id: 13448, name: "Ledge Lab".into() }));
        assert_eq!(parse_stage_file(&record, 7), None, "the id must match the file name");
        assert_eq!(parse_stage_file(&record[..1000], 13448), None, "a cut file");
        let mut too_long = record.clone();
        too_long[DATA_LEN_AT..DATA_LEN_AT + 4].copy_from_slice(&(STAGE_FILE_LEN as u32).to_le_bytes());
        assert_eq!(parse_stage_file(&too_long, 13448), None, "layout data past the record");
        let mut unnamed = record.clone();
        unnamed[NAME_AT..NAME_END].fill(0);
        assert_eq!(parse_stage_file(&unnamed, 13448).map(|s| s.name), Some("Custom stage 13448".into()));
    }

    fn custom_stages_travel_through_the_cycles_encoded() {
        use crate::engine::stages::stage_forms::{Form, Forms, Selection};
        assert_eq!(decode(encode(13448)), Some(13448));
        assert_eq!(decode(311), None);
        let mut order = StageOrder::default();
        order.toggle(StageRef::Custom(12));
        assert_eq!(order.selections(&three(), &Forms::NORMAL),
            vec![Selection { stage: 311, form: Form::Normal }, Selection { stage: encode(12), form: Form::Normal }]);
        assert_eq!(order.selections(&[], &Forms::NORMAL), vec![Selection { stage: 311, form: Form::Normal }],
            "a deleted stage is not in the cycle");
        assert_eq!(crate::engine::stages::lobby_stages::native_stage(Selection { stage: encode(12), form: Form::Normal }),
            Ok(CUSTOM_STAGE_KIND));
        assert!(crate::engine::stages::lobby_stages::native_stage(Selection { stage: encode(12), form: Form::Omega }).is_err());
        // The waiting rooms' cycle and Training Stage Rotation move through them as through catalog stages.
        use crate::engine::stages::lobby_stage_cycle::StageCycle;
        let rotation = vec![Selection::normal(311), Selection::normal(encode(12))];
        let l = Lists { rotation: rotation.clone(), direct: Selection::normal(encode(40)), direct_mode: false, missing: None };
        let mut cycle = StageCycle::<Selection>::new();
        // The read-ahead looks at the next stage without reserving it.
        assert_eq!(cycle.peek_lists(&l, true), Some(rotation[0]));
        assert_eq!(cycle.prepare_lists(true, || Ok(l.clone()), true), Ok(Some(rotation[0])));
        assert!(cycle.enter_resolved().is_ok());
        assert_eq!(cycle.peek_lists(&l, true), Some(rotation[1]), "after the Waiting Room, the custom stage");
        assert_eq!(cycle.peek_lists(&Lists { direct_mode: true, ..l.clone() }, true), Some(Selection::normal(encode(40))));
        assert_eq!(cycle.prepare_lists(true, || Ok(l.clone()), true), Ok(Some(rotation[1])));
        assert_eq!(cycle.enter_resolved(), Ok(Some((rotation[1], CUSTOM_STAGE_KIND))));
        let direct = Lists { direct_mode: true, ..l.clone() };
        assert_eq!(cycle.prepare_lists(true, || Ok(direct.clone()), true), Ok(Some(Selection::normal(encode(40)))));
        assert_eq!(cycle.enter_resolved(), Ok(Some((Selection::normal(encode(40)), CUSTOM_STAGE_KIND))));
        assert_eq!(cycle.prepare_lists(false, || Ok(direct.clone()), true), Ok(None), "practice off");
        use crate::engine::stages::training_stage_rotation_policy::{Entry, TrainingRotation};
        let mut training = TrainingRotation::new();
        let customs_only = Lists { rotation: vec![rotation[1]], ..l.clone() };
        assert_eq!(training.entry_lists(0x2a00_1000, true, || Ok(customs_only.clone())),
            Entry::Prepared(rotation[1], CUSTOM_STAGE_KIND));
        assert_eq!(training.entry_lists(0x2a00_1000, false, || Ok(customs_only.clone())), Entry::Disabled);
    }

    fn the_cycles_take_the_custom_stages_the_worker_published() {
        use crate::engine::stages::stage_forms::{Form, Selection};
        use crate::engine::stages::stage_menu::{Mode, Preferences};
        let customs = three();
        let mut order = StageOrder::default();
        order.toggle(StageRef::Custom(40));
        order.toggle(StageRef::Catalog(3));
        let published = Published { order: order.clone(), customs: customs.clone() };
        let mut prefs = Preferences::default();
        let custom = |id| Selection { stage: encode(id), form: Form::Normal };
        let got = lists(order.legacy(), &prefs, Some(&published)).unwrap();
        assert_eq!(got.rotation, vec![Selection::normal(311), custom(40), Selection::normal(3)]);
        assert_eq!((got.direct, got.direct_mode, got.missing), (Selection::normal(311), false, None));
        // Before the worker has read the save, and when `lobby_stages` changed alone, the catalog order as before.
        assert_eq!(lists(order.legacy(), &prefs, None).unwrap().rotation, vec![Selection::normal(311), Selection::normal(3)]);
        assert_eq!(lists(LobbyStageOrder::NATIVE, &prefs, Some(&published)).unwrap().rotation, vec![Selection::normal(311)]);
        // Direct on a custom stage the save holds; deleted, the catalog Direct choice and the stage's name for the notice.
        prefs.mode = Mode::Direct;
        let mut direct = published.clone();
        direct.order.direct_custom = Some(12);
        direct.order.remember(&customs);
        let got = lists(order.legacy(), &prefs, Some(&direct)).unwrap();
        assert_eq!((got.direct, got.direct_mode, got.missing.as_deref()), (custom(12), true, None));
        direct.customs.retain(|s| s.id != 12);
        let got = lists(order.legacy(), &prefs, Some(&direct)).unwrap();
        assert_eq!((got.direct, got.missing.as_deref()), (Selection::normal(311), Some("Flat")));
        // An order whose custom stages are all deleted moves through the fallback stage, as the pruned order will.
        let only = StageOrder { entries: vec![StageRef::Custom(7)], ..StageOrder::default() };
        let gone = Published { order: only.clone(), customs: vec![] };
        assert_eq!(lists(only.legacy(), &prefs, Some(&gone)).unwrap().rotation, vec![Selection::normal(311)]);
        // What the runtime reads ahead from the save: the order's custom stages, then the Direct one.
        let mut ahead = order.clone();
        ahead.direct_custom = Some(12);
        assert_eq!(ahead.custom_ids(&customs), vec![40, 12]);
        ahead.direct_custom = Some(40);
        assert_eq!(ahead.custom_ids(&customs), vec![40]);
        assert!(ahead.select_all());
        assert_eq!(ahead.custom_ids(&customs), vec![7, 12, 40]);
        assert_eq!(ahead.custom_ids(&[]), Vec::<u32>::new());
        // The list reads each file's header only; the loader needs the header and the layout data.
        let mut record = vec![0u8; STAGE_FILE_LEN];
        record[..4].copy_from_slice(&13448u32.to_le_bytes());
        record[DATA_LEN_AT..DATA_LEN_AT + 4].copy_from_slice(&522u32.to_le_bytes());
        let (stage, used) = parse_stage_header(&record[..HEADER_LEN], STAGE_FILE_LEN as u64, 13448).unwrap();
        assert_eq!((stage.id, used), (13448, HEADER_LEN + 522));
        assert_eq!(parse_stage_header(&record[..HEADER_LEN], 1000, 13448), None, "a cut file");
    }

    fn a_stage_is_read_through_the_game_one_at_a_time_and_never_over_a_game_operation() {
        let scope = |storage: usize, idle: bool| Observed { storage, idle, in_scope: true };
        let mut m = ReadMachine::new();
        // Nothing wanted: nothing starts.
        assert_eq!(m.step(scope(0, true)), ReadAction::None);
        m.want(Some(13448));
        // Not in Training or the waiting-room practice, or a game operation running: no start.
        assert_eq!(m.step(Observed { storage: 0, idle: true, in_scope: false }), ReadAction::None);
        assert_eq!(m.step(scope(0x5000, true)), ReadAction::None, "a storage exists: the game's own operation");
        assert_eq!(m.step(scope(0, false)), ReadAction::None, "the busy word is set");
        // In scope and idle: queue and begin; the storage the game made is TR's from now on.
        assert_eq!(m.step(scope(0, true)), ReadAction::Start(13448));
        m.started(13448, 0x7000);
        assert!(!m.ready(13448, scope(0x7000, false)), "still reading");
        assert_eq!(m.step(scope(0x7000, false)), ReadAction::None);
        // The game finished the read: ready for the load, only while the storage is still TR's and idle.
        assert_eq!(m.step(scope(0x7000, true)), ReadAction::None);
        assert!(m.ready(13448, scope(0x7000, true)));
        assert!(!m.ready(12, scope(0x7000, true)), "another stage");
        assert!(!m.ready(13448, scope(0x9000, true)), "the game replaced the storage");
        // TR builds the stage's data right away and cleans up: held, it is not read again until it is used.
        m.consumed();
        m.have(Some(13448));
        assert_eq!(m.step(scope(0, true)), ReadAction::None, "held: no second read");
        m.have(None);
        assert_eq!(m.step(scope(0, true)), ReadAction::Start(13448), "used and still wanted: read again for the next use");
        m.started(13448, 0x7100);
        // The game replaced TR's storage mid-read (its own operation): dropped, no cleanup, not retried for this want.
        assert_eq!(m.step(scope(0x8000, false)), ReadAction::None);
        assert!(m.failed());
        assert_eq!(m.step(scope(0, true)), ReadAction::None, "not retried");
        // A new want clears it.
        m.want(Some(40));
        assert!(!m.failed());
        assert_eq!(m.step(scope(0, true)), ReadAction::Start(40));
        m.started(40, 0x7200);
        // A read that never finishes is abandoned after READ_FRAMES, and cleaned up only once idle and still TR's.
        for _ in 0..READ_FRAMES {
            assert_eq!(m.step(scope(0x7200, false)), ReadAction::None);
        }
        assert!(m.failed());
        assert!(!m.ready(40, scope(0x7200, true)), "abandoned: not loaded");
        assert_eq!(m.step(scope(0x7200, false)), ReadAction::None);
        assert_eq!(m.step(scope(0x7200, true)), ReadAction::Cleanup);
        assert_eq!(m.step(scope(0, true)), ReadAction::None, "not retried for this want");
        // A ready read nobody wants any more is cleaned up; a begin that made no storage fails.
        m.want(Some(7));
        assert_eq!(m.step(scope(0, true)), ReadAction::Start(7));
        m.started(7, 0x7300);
        assert_eq!(m.step(scope(0x7300, true)), ReadAction::None);
        m.want(None);
        assert_eq!(m.step(scope(0x7300, true)), ReadAction::Cleanup);
        assert_eq!(m.step(scope(0, true)), ReadAction::None);
        m.want(Some(9));
        assert_eq!(m.step(scope(0, true)), ReadAction::Start(9));
        m.started(9, 0);
        assert!(m.failed());
        // A finished read without the record (the stage was deleted meanwhile) is not retried for this want.
        m.want(Some(11));
        assert_eq!(m.step(scope(0, true)), ReadAction::Start(11));
        m.started(11, 0x7400);
        assert_eq!(m.step(scope(0x7400, true)), ReadAction::None);
        assert!(m.ready(11, scope(0x7400, true)));
        m.spoiled();
        assert!(m.failed());
        assert_eq!(m.step(scope(0, true)), ReadAction::None);
        // Names as the game keeps them.
        let units: Vec<u16> = "Ledge Lab".encode_utf16().chain([0, 65, 66]).collect();
        assert_eq!(name_from_units(7, &units), "Ledge Lab");
        assert_eq!(name_from_units(7, &[0; 22]), "Custom stage 7");
    }

    fn held_data_is_freed_once_its_stage_is_no_longer_next() {
        let scope = Observed { storage: 0, idle: true, in_scope: true };
        let mut m = ReadMachine::new();
        // Nothing held: nothing to free, whatever the want.
        m.want(None);
        m.want(Some(7));
        assert_eq!(m.take_stale(), None);
        // TR holds 7's data and 7 is still next: kept.
        m.have(Some(7));
        m.want(Some(7));
        m.want(Some(7));
        assert_eq!(m.take_stale(), None);
        // One check that names nothing (a busy list) keeps it; 7 again starts the count over.
        m.want(None);
        assert_eq!(m.take_stale(), None, "one check can miss the next entry");
        m.want(Some(7));
        m.want(None);
        assert_eq!(m.take_stale(), None, "the count starts over");
        // Deleted: the lists name no custom stage any more, two checks in a row. Freed once.
        m.want(None);
        assert_eq!(m.take_stale(), Some(7));
        assert_eq!(m.take_stale(), None, "once");
        // No longer held: wanted again, it is read again.
        m.want(Some(7));
        assert_eq!(m.step(scope), ReadAction::Start(7));
        m.started(7, 0x7000);
        assert_eq!(m.step(Observed { storage: 0x7000, ..scope }), ReadAction::None);
        m.consumed();
        m.have(Some(7));
        // The list moved on: another stage is next on two checks in a row.
        m.want(Some(12));
        assert_eq!(m.take_stale(), None);
        m.want(Some(12));
        assert_eq!(m.take_stale(), Some(7));
        // A load used the held data: nothing is left to free.
        m.have(Some(12));
        m.have(None);
        m.want(None);
        m.want(None);
        assert_eq!(m.take_stale(), None);
    }

    fn tr_frees_only_what_it_took_out_of_the_rule_and_gives_the_rule_back() {
        let slot = |p: u64, id: u32| RuleSlot { head: 0x0101, word: 2, tail: 3, data: [p, p + 0x10, p + 0x10], extra: [0; 3], id };
        let original = RuleSlot { data: [0x100, 0x110, 0x110], extra: [0x200, 0x210, 0x210], id: 5, ..RuleSlot::default() };
        let plan = |write: Option<(u32, RuleSlot)>, free: &[u64]| {
            let mut all = [0u64; 4];
            all[..free.len()].copy_from_slice(free);
            RulePlan { write, free: all }
        };
        let mut owner = RuleOwner::new();
        // The first load keeps the rule's own data for later.
        let a = slot(0x1000, 7);
        assert_eq!(owner.load(1, original, a), plan(Some((1, a)), &[]));
        assert!(owner.holds(1, a, 7));
        assert!(!owner.holds(1, a, 8) && !owner.holds(0, a, 7) && !owner.holds(1, original, 5));
        // The next load takes A out but keeps it: the stage on screen may still read it.
        let b = slot(0x2000, 12);
        assert_eq!(owner.load(1, a, b), plan(Some((1, b)), &[]));
        // One more: A goes now, B waits.
        let c = slot(0x3000, 40);
        assert_eq!(owner.load(1, b, c), plan(Some((1, c)), &[0x1000]));
        // The practice scene ends while the rule holds C: the original goes back, B goes, C waits for the next step.
        assert_eq!(owner.depart(c), plan(Some((1, original)), &[0x2000]));
        assert_eq!(owner.index(), None);
        assert_eq!(owner.depart(original), RulePlan::default(), "once");
        // A new scene's first load frees C; the rule's data is the original again and stays.
        let d = slot(0x4000, 7);
        assert_eq!(owner.load(1, original, d), plan(Some((1, d)), &[0x3000]));
        // The game wrote the slot meanwhile (a room's match data, a replay): it freed D itself. TR frees only the
        // original it took out, and writes nothing back.
        let game = RuleSlot { data: [0x5000, 0x5010, 0x5010], id: 99, ..RuleSlot::default() };
        assert_eq!(owner.depart(game), plan(None, &[0x100, 0x200]));
        // A load after the game wrote the slot between two loads: the game's data is the original now, the one TR
        // kept is freed.
        let e = slot(0x6000, 12);
        assert_eq!(owner.load(1, game, e), plan(Some((1, e)), &[]));
        let game2 = RuleSlot { data: [0x8000, 0x8010, 0x8010], id: 98, ..RuleSlot::default() };
        let f = slot(0x7000, 40);
        assert_eq!(owner.load(1, game2, f), plan(Some((1, f)), &[0x5000]));
        // A game write that reuses TR's buffer (same start, other length): never freed while the rule holds it.
        let reused = RuleSlot { data: [0x7000, 0x7004, 0x7010], id: 97, ..RuleSlot::default() };
        let g = slot(0x9000, 7);
        assert_eq!(owner.load(1, reused, g).freed().collect::<Vec<_>>(), vec![0x8000]);
        // The game wrote the slot again after G (and freed G): nothing to write back, and the rule holds 0x7000.
        assert_eq!(owner.depart(reused), RulePlan::default());
    }
    /// This module's cases in one table. Each case runs on its own thread, as the test harness runs them.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("custom_stages_follow_the_catalog_as_extra_tiles", custom_stages_follow_the_catalog_as_extra_tiles as fn()),
            ("deleting_a_custom_stage_prunes_the_order_and_closes_the_gaps", deleting_a_custom_stage_prunes_the_order_and_closes_the_gaps as fn()),
            ("rotation_cycles_through_custom_entries_in_order", rotation_cycles_through_custom_entries_in_order as fn()),
            ("the_kept_order_round_trips_and_older_files_still_read", the_kept_order_round_trips_and_older_files_still_read as fn()),
            ("a_missing_direct_custom_stage_falls_back_with_its_name", a_missing_direct_custom_stage_falls_back_with_its_name as fn()),
            ("the_save_files_give_their_id_and_name_and_bad_ones_are_left_out", the_save_files_give_their_id_and_name_and_bad_ones_are_left_out as fn()),
            ("custom_stages_travel_through_the_cycles_encoded", custom_stages_travel_through_the_cycles_encoded as fn()),
            ("the_cycles_take_the_custom_stages_the_worker_published", the_cycles_take_the_custom_stages_the_worker_published as fn()),
            ("tr_frees_only_what_it_took_out_of_the_rule_and_gives_the_rule_back", tr_frees_only_what_it_took_out_of_the_rule_and_gives_the_rule_back as fn()),
            ("a_stage_is_read_through_the_game_one_at_a_time_and_never_over_a_game_operation", a_stage_is_read_through_the_game_one_at_a_time_and_never_over_a_game_operation as fn()),
            ("held_data_is_freed_once_its_stage_is_no_longer_next", held_data_is_freed_once_its_stage_is_no_longer_next as fn()),
        ]);
    }
    }
}
