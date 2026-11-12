//! Mod-owned drafts and presets. No native-save IDs, game pointers or file writes.
//! Preview adapters still need current native ownership/row validation before loading a value.
use crate::engine::mii::inventory::{Canonical, MiiConfig, Snapshot, TaggedId, CHARINFO_LEN};
use crate::engine::mii::lifetime::Configuration;
use serde::{Deserialize, Serialize};

pub const MAX_NAME_CHARS: usize = 48;
pub const MAX_PRESETS: usize = 256;
const SCHEMA: u32 = 1;

#[derive(Clone)]
pub struct Draft {
    template: MiiConfig,
    specials: [u8; 4],
}

impl Draft {
    /// An existing qualified appearance is copied; editing never changes its native save.
    pub fn from_template(template: &MiiConfig) -> Result<Self, &'static str> {
        Configuration::from_saved(template).map_err(|_| "Invalid Mii appearance template")?;
        Ok(Self { template: template.clone(), specials: template.canonical.appearance.specials })
    }

    pub fn specials(&self) -> [u8; 4] { self.specials }

    pub fn set_specials(&mut self, specials: [u8; 4]) -> Result<(), &'static str> {
        if specials.iter().any(|value| *value > 2) {
            return Err("Choose one of the three moves for each special");
        }
        self.specials = specials;
        Ok(())
    }

    fn value(&self) -> MiiConfig {
        let mut value = self.template.clone();
        value.canonical.appearance.specials = self.specials;
        value
    }

    /// Owned configuration only. This does not submit a swap or authorize native writes.
    pub fn configuration(&self) -> Result<Configuration, &'static str> {
        Configuration::from_saved(&self.value()).map_err(|_| "Invalid Mii draft")
    }

    fn preset(&self, name: &str) -> Result<Preset, &'static str> {
        Preset::new(name, self.configuration()?, self.template.raw_tags)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Preset {
    name: String,
    configuration: Configuration,
    tags: [TaggedId; 3],
}

impl core::fmt::Debug for Preset {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Preset").field("name", &self.name)
            .field("configuration", &self.configuration).finish_non_exhaustive()
    }
}

impl Preset {
    fn new(name: &str, configuration: Configuration, tags: [TaggedId; 3]) -> Result<Self, &'static str> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > MAX_NAME_CHARS || name.chars().any(char::is_control) {
            return Err("Use a nonempty name of at most 48 characters without control characters");
        }
        Ok(Self { name: name.into(), configuration, tags })
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn configuration(&self) -> Configuration { self.configuration }
    pub fn canonical(&self) -> Canonical { self.configuration.canonical() }

    fn stored(&self) -> StoredPreset {
        StoredPreset { name: self.name.clone(), canonical: self.canonical(),
            charinfo: self.configuration.charinfo().to_vec(), tags: self.tags.map(|tag| tag.0) }
    }
}

/// No native slot number is assigned to a mod preset. Integrating catalogs must preserve this
/// provenance instead of pretending a preset is a newly created native save file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Saved {
    ExistingNative(Canonical),
    ExistingPreset(Canonical),
    Created(Canonical),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredLibrary", into = "StoredLibrary")]
pub struct Library {
    presets: Vec<Preset>,
}

impl Library {
    pub fn presets(&self) -> &[Preset] { &self.presets }

    /// First explicit save commits a draft. Native configurations win duplicate lookup and
    /// existing names stay unchanged. Dropping/cancelling a Draft performs no library mutation.
    pub fn save(&mut self, draft: &Draft, name: &str, native: &Snapshot) -> Result<Saved, &'static str> {
        let preset = draft.preset(name)?;
        self.insert(preset, native)
    }

    /// Import a resolved owned replay value, preserving native-first canonical deduplication.
    pub fn import(&mut self, configuration: Configuration, name: &str, native: &Snapshot) -> Result<Saved, &'static str> {
        use crate::engine::mii::inventory::{TAG_CHARA, TAG_HAT, TAG_OUTFIT};
        let appearance = configuration.canonical().appearance;
        let tokens = configuration.tokens();
        let tag = |kind, row, hash| TaggedId((u64::from(kind) << 56) | (u64::from(row) << 40) | hash);
        let tags = [tag(TAG_OUTFIT, tokens.outfit, appearance.outfit),
            tag(TAG_HAT, tokens.hat, appearance.hat), tag(TAG_CHARA, 0u16, appearance.chara)];
        self.insert(Preset::new(name, configuration, tags)?, native)
    }

    fn insert(&mut self, preset: Preset, native: &Snapshot) -> Result<Saved, &'static str> {
        if !native.is_complete() { return Err("Wait for a complete native Mii catalog"); }
        let canonical = preset.canonical();
        let equal = |candidate: Canonical| candidate.hash() == canonical.hash() && candidate == canonical;
        if native.configs.iter().any(|config| equal(config.canonical)) {
            return Ok(Saved::ExistingNative(canonical));
        }
        if self.presets.iter().any(|entry| equal(entry.canonical())) {
            return Ok(Saved::ExistingPreset(canonical));
        }
        if self.presets.len() == MAX_PRESETS { return Err("Mii preset library is full"); }
        self.presets.push(preset);
        Ok(Saved::Created(canonical))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredLibrary {
    schema: u32,
    presets: Vec<StoredPreset>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPreset {
    name: String,
    canonical: Canonical,
    charinfo: Vec<u8>,
    tags: [u64; 3],
}

impl TryFrom<StoredPreset> for Preset {
    type Error = &'static str;
    fn try_from(stored: StoredPreset) -> Result<Self, Self::Error> {
        let charinfo: [u8; CHARINFO_LEN] = stored.charinfo.try_into().map_err(|_| "Invalid Mii appearance length")?;
        let tags = stored.tags.map(TaggedId);
        // Reuse the value validator, not the saved-file reader. These zero placeholders never
        // leave this function or become a native save selector.
        let value = MiiConfig { slot_id: 0, slot_index: 0, name: stored.name.clone(), charinfo,
            raw_tags: tags, canonical: stored.canonical, copies: Vec::new() };
        let configuration = Configuration::from_saved(&value).map_err(|_| "Mii preset appearance or resource tags differ")?;
        Self::new(&stored.name, configuration, tags)
    }
}

impl TryFrom<StoredLibrary> for Library {
    type Error = &'static str;
    fn try_from(stored: StoredLibrary) -> Result<Self, Self::Error> {
        if stored.schema != SCHEMA { return Err("Unsupported Mii preset library version"); }
        if stored.presets.len() > MAX_PRESETS { return Err("Mii preset library exceeds its capacity"); }
        let mut presets: Vec<Preset> = Vec::with_capacity(stored.presets.len());
        for record in stored.presets {
            let preset = Preset::try_from(record)?;
            let id = preset.canonical();
            if !presets.iter().any(|p| p.canonical().hash() == id.hash() && p.canonical() == id) {
                presets.push(preset);
            }
        }
        Ok(Self { presets })
    }
}

impl From<Library> for StoredLibrary {
    fn from(library: Library) -> Self {
        Self { schema: SCHEMA, presets: library.presets.iter().map(Preset::stored).collect() }
    }
}

