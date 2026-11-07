//! `recording-index.json`: last use, the hot five, loop counts, the bound save, active macros, the drill.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{check_ids, Drill, Error, Kind, LoopCount, Role, Use};
use crate::model::store::{default_id, write_replace};

const MAX_INDEX_BYTES: usize = 1024 * 1024;

/// One id's last-use time, and the five hottest ids, for each kind.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct ByKind<T> {
    #[serde(rename = "macro", default)]
    r#macro: T,
    #[serde(default)]
    save: T,
    #[serde(default)]
    sequence: T,
    #[serde(default)]
    combo: T,
}

impl<T> ByKind<T> {
    fn get(&self, kind: Kind) -> &T {
        match kind {
            Kind::Macro => &self.r#macro,
            Kind::Save => &self.save,
            Kind::Sequence => &self.sequence,
            Kind::Combo => &self.combo,
        }
    }

    fn get_mut(&mut self, kind: Kind) -> &mut T {
        match kind {
            Kind::Macro => &mut self.r#macro,
            Kind::Save => &mut self.save,
            Kind::Sequence => &mut self.sequence,
            Kind::Combo => &mut self.combo,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Loops {
    #[serde(default)]
    player: LoopCount,
    #[serde(default)]
    cpu: LoopCount,
}

impl Default for Loops {
    fn default() -> Self {
        Self { player: LoopCount::Finite(1), cpu: LoopCount::Finite(1) }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Actives {
    #[serde(default)]
    player: Option<String>,
    #[serde(default)]
    cpu: Option<String>,
}

/// Ids migration wrote, so a second call can return them without touching the card.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationReceipt {
    pub recordings: Vec<String>,
    pub saves: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_save: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_player: Option<String>,
    pub loop_player: LoopCount,
    pub loop_cpu: LoopCount,
}

fn schema_one() -> u32 {
    1
}

/// The index beside the content directories. Missing file: both loops are 1, nothing is active.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingIndex {
    #[serde(default = "schema_one")]
    schema_version: u32,
    #[serde(default)]
    migrated: bool,
    #[serde(default)]
    last_used: ByKind<BTreeMap<String, u64>>,
    #[serde(default)]
    hot: ByKind<Vec<String>>,
    #[serde(default, rename = "loop")]
    loops: Loops,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bound_save: Option<String>,
    #[serde(default)]
    active: Actives,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    drill: Option<Drill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    migration: Option<MigrationReceipt>,
    /// The one-time chord notice. Omitted when it has not been shown, so older indexes still load.
    #[serde(default, skip_serializing_if = "bool_is_false")]
    chord_notice_seen: bool,
}

impl RecordingIndex {
    pub fn fresh() -> Self {
        Self {
            schema_version: 1,
            migrated: false,
            last_used: ByKind::default(),
            hot: ByKind::default(),
            loops: Loops::default(),
            bound_save: None,
            active: Actives::default(),
            drill: None,
            migration: None,
            chord_notice_seen: false,
        }
    }

    /// Reads the index. A missing file is a fresh index and is not created here.
    pub fn load(root: &Path) -> Result<Self, Error> {
        let path = root.join(INDEX_FILE);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Self::fresh()),
            Err(err) => return Err(err.into()),
        };
        if bytes.len() > MAX_INDEX_BYTES {
            return Err(Error::Invalid("index is too large"));
        }
        let index: Self = serde_json::from_slice(&bytes)?;
        if index.schema_version != 1 {
            return Err(Error::Invalid("bad schema"));
        }
        Ok(index)
    }

    pub fn save(&self, root: &Path) -> Result<(), Error> {
        fs::create_dir_all(root)?;
        let mut bytes = serde_json::to_vec_pretty(self)?;
        bytes.push(b'\n');
        write_replace(&root.join(INDEX_FILE), &bytes)?;
        Ok(())
    }

    pub fn migrated(&self) -> bool {
        self.migrated
    }

    pub fn chord_notice_seen(&self) -> bool {
        self.chord_notice_seen
    }

    pub fn mark_chord_notice_seen(&mut self) {
        self.chord_notice_seen = true;
    }

    pub fn receipt(&self) -> Option<&MigrationReceipt> {
        self.migration.as_ref()
    }

    pub fn hot(&self, kind: Kind) -> &[String] {
        self.hot.get(kind)
    }

    pub fn active(&self, role: Role) -> Option<&str> {
        match role {
            Role::Player => self.active.player.as_deref(),
            Role::Cpu => self.active.cpu.as_deref(),
        }
    }

    pub fn loop_count(&self, role: Role) -> LoopCount {
        match role {
            Role::Player => self.loops.player,
            Role::Cpu => self.loops.cpu,
        }
    }

    pub fn bound(&self) -> Option<&str> {
        self.bound_save.as_deref()
    }

    pub fn drill(&self) -> Option<&Drill> {
        self.drill.as_ref()
    }

    /// Records a use and rebuilds that kind's five most recent ids. The sixth id leaves the list.
    pub fn note_use(&mut self, kind: Kind, id: &str, reason: Use, at: u64) -> Result<(), Error> {
        if !default_id(id) {
            return Err(Error::Invalid("bad id"));
        }
        match reason {
            Use::Bound | Use::SetActive | Use::Played | Use::Drilled | Use::Recorded => {
                self.touch(kind, id, at);
            }
        }
        Ok(())
    }

    /// Replaces this role's active macro. The other role stays. `None` clears it.
    pub fn set_active(&mut self, role: Role, id: Option<&str>, at: u64) -> Result<(), Error> {
        if let Some(id) = id {
            check_ids(&[id.to_owned()])?;
        }
        let stored = id.map(str::to_owned);
        match role {
            Role::Player => self.active.player = stored.clone(),
            Role::Cpu => self.active.cpu = stored.clone(),
        }
        if let Some(id) = stored {
            self.note_use(Kind::Macro, &id, Use::SetActive, at)?;
        }
        Ok(())
    }

    pub fn set_loop(&mut self, role: Role, count: LoopCount) {
        match role {
            Role::Player => self.loops.player = count,
            Role::Cpu => self.loops.cpu = count,
        }
    }

    pub fn bind_save(&mut self, id: Option<&str>, at: u64) -> Result<(), Error> {
        if let Some(id) = id {
            check_ids(&[id.to_owned()])?;
        }
        self.bound_save = id.map(str::to_owned);
        if let Some(id) = id {
            self.note_use(Kind::Save, id, Use::Bound, at)?;
        }
        Ok(())
    }

    pub fn set_drill(&mut self, drill: Option<Drill>, at: u64) -> Result<(), Error> {
        if let Some(drill) = &drill {
            check_ids(std::slice::from_ref(&drill.id))?;
        }
        self.drill = drill.clone();
        if let Some(drill) = drill {
            let kind = match drill.kind {
                super::DrillKind::Combo => Kind::Combo,
                super::DrillKind::Sequence => Kind::Sequence,
            };
            self.note_use(kind, &drill.id, Use::Drilled, at)?;
        }
        Ok(())
    }

    pub(crate) fn apply_migration(&mut self, receipt: &MigrationReceipt) {
        self.loops.player = receipt.loop_player;
        self.loops.cpu = receipt.loop_cpu;
        self.active.player = receipt.active_player.clone();
        self.active.cpu = None;
        self.bound_save = receipt.bound_save.clone();
        self.migration = Some(receipt.clone());
        self.migrated = true;
    }

    fn touch(&mut self, kind: Kind, id: &str, at: u64) {
        self.last_used.get_mut(kind).insert(id.to_owned(), at);
        let mut rows: Vec<(String, u64)> =
            self.last_used.get(kind).iter().map(|(id, at)| (id.clone(), *at)).collect();
        rows.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
        *self.hot.get_mut(kind) = rows.into_iter().take(5).map(|(id, _)| id).collect();
    }
}

fn bool_is_false(value: &bool) -> bool {
    !*value
}

pub const INDEX_FILE: &str = "recording-index.json";
