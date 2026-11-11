//! The pure half of the resident effect-texture helper (`runtime/ui/effect_texture.rs`, stream u4b; SPEC-core-07 section 4.6,
//! question 17): where the game keeps the textures of its own DI and SDI effects, and what the plugin reads of them.
//!
//! The game loads `effect/system/common/ef_common.eff` (arc hash40 `0x222d35b012`; its manager entry is named by the folder hash) through its `lib::EffectManager`, keeps the file's
//! bytes resident and hands the `VFXB` block to its particle system (`nn::vfx` resource), which builds every texture of the file's
//! `BNTX` in place (memory pool, texture, texture view) and registers each view in the same descriptor pool the UI draws from
//! (`0x9ab70` called with `0x35b9360`, the function the UI texture builder also registers its views with). The plugin therefore
//! neither reads, decodes nor uploads the file: it finds the resident resource, reads the emitter data from the resident bytes and copies the
//! texture view the game already built: the UI texture code registers that view once more for a picture pane (`runtime::ui::texture::load_resident`).
//!
//! Every offset below is decoded from the owned 13.0.5 `main` (`docs/provenance` rows of `sites_u4b.rs` name the instructions that
//! use them); the `EMTR` field offsets (colour, alpha, scale, sampler) are stream W9's inferred layout, checked against the game's
//! own reads at `+0x9f0`, `+0xa10`, `+0xa30` (the three sampler ids the resource's slot resolver reads).

/// How the manager's entry names the file: the hash40 of its folder, `effect/system/common` (the manager loads a folder's effect file; Eden
/// u4b-01 survey: entry hash `0x1482d0fc5a`, which is that string's CRC-32 `0x82d0fc5a` with length 0x14; the file inside is
/// `ef_common.eff`, arc hash40 `0x222d35b012`). The file is confirmed by its `EFFN` names, not by this hash alone.
pub const EF_COMMON_HASH40: u64 = 0x14_82d0_fc5a;
pub const HASH_MASK: u64 = 0xff_ffff_ffff;

/// `main`-relative slot that holds the address of `lib::Singleton<lib::EffectManager>::instance_` (the instance pointer is read
/// through it twice: `adrp`/`ldr` of the slot, then `ldr x0, [x8]`).
pub const MANAGER_SLOT: usize = 0x533_4920;

/// The effect manager's file entries.
pub const ENTRY_BASE: usize = 0x18000;
pub const ENTRY_STRIDE: usize = 0xa8;
pub const ENTRY_COUNT: usize = 30;
/// `i32`: the caller's resource key, -1 while the entry is free.
pub const ENTRY_KEY: usize = 0x00;
/// `i32`: the index of the particle-system resource that was made from this entry's file.
pub const ENTRY_RESOURCE: usize = 0x04;
/// `u64`: the hash40 of the file's folder (low 40 bits).
pub const ENTRY_HASH: usize = 0x08;
/// `u64`: the loaded file's bytes (the `EFFN` header).
pub const ENTRY_DATA: usize = 0x10;
/// The manager's pointer to the particle system (`nn::vfx::System`), and the system's array of resources.
pub const MANAGER_SYSTEM: usize = 0x194d0;
pub const SYSTEM_RESOURCES: usize = 0x98;
pub const SYSTEM_RESOURCE_MAX: i32 = 64;

/// The particle-system resource.
pub const RESOURCE_VALID: usize = 0x09;
pub const RESOURCE_SET_COUNT: usize = 0x4c;
pub const RESOURCE_SETS: usize = 0x58;
pub const RESOURCE_TEXTURE_COUNT: usize = 0xa0;
pub const RESOURCE_TEXTURES: usize = 0xa8;
/// The texture file object (`ResTextureFile`): the array of texture pointers, and the table whose `+4` is their count.
pub const FILE_TEXTURES: usize = 0x28;
/// One texture (`BRTI`): size, the texture view the game built, and the 8-byte descriptor slot it registered.
pub const BRTI_FORMAT: usize = 0x1c;
pub const BRTI_WIDTH: usize = 0x24;
pub const BRTI_HEIGHT: usize = 0x28;
pub const BRTI_VIEW: usize = 0x88;
pub const BRTI_SLOT: usize = 0x90;
/// The slot value the resource writes before it registers a view (and the value an emitter keeps for a texture it does not have).
pub const SLOT_NONE: u64 = u64::MAX;

/// One emitter set (`0x38` bytes): emitter count, the emitters' array.
pub const SET_STRIDE: usize = 0x38;
pub const SET_EMITTER_COUNT: usize = 0x04;
pub const SET_EMITTERS: usize = 0x18;
/// One emitter (`0x3e0` bytes): its binary (`EMTR`) and the three descriptor slots the resource resolved for its three samplers.
pub const EMITTER_STRIDE: usize = 0x3e0;
pub const EMITTER_EMTR: usize = 0x10;
pub const EMITTER_SLOTS: [usize; 3] = [0x48, 0x50, 0x58];

/// The `EFFN` header: count (`u16` at 8), the index of the 4 KiB block that holds the `VFXB` (`i16` at 0xe), 16-byte entries from 0x10
/// whose second word is the 1-based index of the emitter set, then a name table in entry order.
pub const EFFN_MAGIC: [u8; 4] = *b"EFFN";
pub const EFFN_COUNT: usize = 8;
pub const EFFN_BLOCK: usize = 0xe;
pub const EFFN_ENTRIES: usize = 0x10;
pub const EFFN_ENTRY_SIZE: usize = 16;
pub const EFFN_BLOCK_SIZE: usize = 0x1000;
/// The longest `EFFN` header the helper reads (the 13.0.5 file's is `0x2000`).
pub const EFFN_MAX_BYTES: usize = 0x8000;
pub const BRTI_MAGIC: [u8; 4] = *b"BRTI";
pub const EMTR_BYTES: usize = 0xac0;

/// The two visuals the DI and SDI chips draw (question 17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visual {
    /// `SYS_VECTOR`, emitter set `P_CmnVector`.
    Di,
    /// `SYS_HITSTOP_SPARK`, emitter set `P_CmnHitStopSpark`.
    Sdi,
}

impl Visual {
    /// The effect's name in the `EFFN` table.
    pub const fn effect_name(self) -> &'static [u8] {
        match self {
            Visual::Di => b"SYS_VECTOR",
            Visual::Sdi => b"SYS_HITSTOP_SPARK",
        }
    }
}

/// Reads of the running game (or of a test image). A `None` answer means "not mapped / not this size": the walk stops and the chip
/// keeps the clean room's arrow.
pub trait Memory {
    fn bytes(&self, address: usize, len: usize) -> Option<&[u8]>;

    fn u16(&self, address: usize) -> Option<u16> {
        self.bytes(address, 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&self, address: usize) -> Option<u32> {
        self.bytes(address, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn i32(&self, address: usize) -> Option<i32> {
        self.u32(address).map(|v| v as i32)
    }
    fn u64(&self, address: usize) -> Option<u64> {
        self.bytes(address, 8).map(|b| u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }
    fn ptr(&self, address: usize) -> Option<usize> {
        self.u64(address).filter(|v| plausible(*v as usize)).map(|v| v as usize)
    }
}

/// A pointer value the walk may follow: in user space and aligned to 4 (every structure the walk follows is at least word aligned).
pub fn plausible(address: usize) -> bool {
    in_range(address) && address.is_multiple_of(4)
}

/// An address a read may start at (a field of a structure the walk entered may sit at any byte, e.g. the resource's valid byte at `+9`).
pub fn in_range(address: usize) -> bool {
    (0x1_0000..(1usize << 40)).contains(&address)
}

/// Where the resident `ef_common.eff` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Common {
    /// The file's bytes (the `EFFN` header).
    pub effn: usize,
    /// The particle-system resource made from the file.
    pub resource: usize,
    /// The manager entry's index (diagnostics).
    pub entry: usize,
}

/// A descriptor slot the game registered for a texture view: the 8 bytes the UI texture-info keeps at `+0x40`.
pub type Slot = u64;

/// One texture of the resident file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureInfo {
    pub index: usize,
    /// The descriptor slot the particle system registered (how the emitter names the texture).
    pub slot: Slot,
    /// The texture view the game built (`BRTI + 0x88`): what the UI texture code registers again for a picture pane.
    pub view: usize,
    pub width: u32,
    pub height: u32,
    pub format: u32,
}

/// Only the emitter fields printed by the picture diagnostic. The four key
/// arrays are used transiently to calculate tint; keeping them in every picture
/// inflated the UI fiber path beyond its 16 KiB budget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmitterColourSummary {
    pub scale: f32,
    pub sources: [Source; 4],
    pub fixed0: [f32; 4],
    pub fixed1: [f32; 4],
}

impl From<&EmitterColours> for EmitterColourSummary {
    fn from(colours: &EmitterColours) -> Self {
        Self { scale: colours.scale, sources: colours.sources, fixed0: colours.fixed0, fixed1: colours.fixed1 }
    }
}

/// How many pictures one visual keeps (each emitter's distinct textures; the 13.0.5 file has 4 per visual).
pub const MAX_PICTURES: usize = 8;

/// One texture of an effect: what to draw, and the emitter colour to tint it with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Picture {
    pub texture: TextureInfo,
    /// The emitter (index in its set) and sampler (0-2) that name the texture first.
    pub emitter: usize,
    pub sampler: usize,
    /// Colour 0 at life 0.3 times the colour scale (`model::effect_texture::tint_rgba`), white when the emitter's colour fields are not readable.
    pub tint: [u8; 4],
    pub colours: Option<EmitterColourSummary>,
}

/// The textures of one visual.
#[derive(Clone, Copy, Debug)]
pub struct Pictures {
    pub set: usize,
    pub emitters: usize,
    pub count: usize,
    pub pictures: [Option<Picture>; MAX_PICTURES],
}

/// The distinct textures of the emitters of `visual`, in emitter then sampler order.
pub fn pictures<M: Memory>(mem: &M, common: Common, visual: Visual) -> Result<Pictures, Absent> {
    let set = effect_set(mem, common.effn, visual.effect_name())?;
    let emitters = emitters(mem, common.resource, set)?;
    let mut out = Pictures { set, emitters: emitters.count, count: 0, pictures: [None; MAX_PICTURES] };
    for (e, emitter) in emitters.emitters.iter().take(emitters.count).enumerate() {
        let colours = mem.bytes(emitter.emtr, EMTR_BYTES).and_then(EmitterColours::parse);
        for (k, slot) in emitter.slots.iter().enumerate() {
            if *slot == SLOT_NONE || out.pictures.iter().flatten().any(|p| p.texture.slot == *slot) {
                continue;
            }
            // A slot this resource does not hold belongs to the parent resource the particle system falls back to (its slot resolver
            // recurses into it): the chip does not draw that sampler, the others stay.
            let texture = match texture_of_slot(mem, common.resource, *slot) {
                Ok(texture) => texture,
                Err(Absent::NoTexture) => continue,
                Err(other) => return Err(other),
            };
            if out.count < MAX_PICTURES {
                out.pictures[out.count] = Some(Picture { texture, emitter: e, sampler: k, tint: colours.as_ref().map_or([255; 4], tint_rgba), colours: colours.as_ref().map(EmitterColourSummary::from) });
                out.count += 1;
            }
        }
    }
    if out.count == 0 {
        return Err(Absent::NoTexture);
    }
    Ok(out)
}

/// What the plugin keeps of an emitter: its binary's address, the textures its three samplers use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Emitter {
    pub emtr: usize,
    pub slots: [Slot; 3],
}

pub const MAX_EMITTERS: usize = 8;

/// The emitters of one visual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisualEmitters {
    pub set: usize,
    pub count: usize,
    pub emitters: [Emitter; MAX_EMITTERS],
}

/// Why a walk gave up (diagnostics; every one of them leaves the chip on the clean room's arrow).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Absent {
    /// The manager, its entries or the particle system could not be read.
    Manager,
    /// No entry holds `ef_common.eff` (the file is not loaded now).
    NotLoaded,
    /// The entry's bytes or the resource are not what the walk expects.
    Shape,
    /// The `EFFN` table does not name the effect (an effect mod that renamed it).
    NoEffect,
    /// The emitter set is empty or larger than the helper keeps.
    NoEmitters,
    /// A texture the emitters name is not in the resource.
    NoTexture,
}

/// Find the resident `ef_common.eff`: the manager entry that names it, the file's `EFFN` header and the particle-system resource.
pub fn find_common<M: Memory>(mem: &M, manager: usize) -> Result<Common, Absent> {
    let system = mem.ptr(manager + MANAGER_SYSTEM).ok_or(Absent::Manager)?;
    let resources = mem.ptr(system + SYSTEM_RESOURCES).ok_or(Absent::Manager)?;
    for entry in 0..ENTRY_COUNT {
        let base = manager + ENTRY_BASE + entry * ENTRY_STRIDE;
        let (Some(key), Some(hash)) = (mem.i32(base + ENTRY_KEY), mem.u64(base + ENTRY_HASH)) else { return Err(Absent::Manager) };
        if key == -1 || hash & HASH_MASK != EF_COMMON_HASH40 {
            continue;
        }
        let effn = mem.ptr(base + ENTRY_DATA).ok_or(Absent::Shape)?;
        if mem.bytes(effn, 4) != Some(&EFFN_MAGIC[..]) {
            return Err(Absent::Shape);
        }
        let id = mem.i32(base + ENTRY_RESOURCE).filter(|id| (0..SYSTEM_RESOURCE_MAX).contains(id)).ok_or(Absent::NotLoaded)?;
        let resource = mem.ptr(resources + 8 * id as usize).ok_or(Absent::NotLoaded)?;
        // The loader takes the resource only when this byte is non-zero (`ldrb w9, [x8, #9]; cmp w9, #0; csel`).
        if mem.bytes(resource + RESOURCE_VALID, 1).is_none_or(|b| b[0] == 0) {
            return Err(Absent::Shape);
        }
        return Ok(Common { effn, resource, entry });
    }
    Err(Absent::NotLoaded)
}

/// The 0-based emitter-set index the `EFFN` table gives the effect called `name`.
///
/// The names follow the entries as NUL-terminated upper-case words in entry order (W7's reading of the 13.0.5 file; a run of fewer
/// than three characters is table padding), and entry `i` is `{flags, 1-based set index, 0, 0}`.
pub fn effect_set<M: Memory>(mem: &M, effn: usize, name: &[u8]) -> Result<usize, Absent> {
    let head = mem.bytes(effn, EFFN_ENTRIES).ok_or(Absent::Shape)?;
    if head[..4] != EFFN_MAGIC {
        return Err(Absent::Shape);
    }
    let count = u16::from_le_bytes([head[EFFN_COUNT], head[EFFN_COUNT + 1]]) as usize;
    let block = i16::from_le_bytes([head[EFFN_BLOCK], head[EFFN_BLOCK + 1]]);
    let names_at = EFFN_ENTRIES + count * EFFN_ENTRY_SIZE;
    let end = usize::try_from(block).ok().map(|b| b * EFFN_BLOCK_SIZE).filter(|e| *e > names_at && *e <= EFFN_MAX_BYTES).ok_or(Absent::Shape)?;
    let all = mem.bytes(effn, end).ok_or(Absent::Shape)?;
    let index = name_index(&all[names_at..], name, count).ok_or(Absent::NoEffect)?;
    let word = EFFN_ENTRIES + index * EFFN_ENTRY_SIZE + 4;
    let set = u32::from_le_bytes([all[word], all[word + 1], all[word + 2], all[word + 3]]);
    usize::try_from(set).ok().and_then(|s| s.checked_sub(1)).ok_or(Absent::NoEffect)
}

/// The index of `name` among the table's names (runs of `[A-Z0-9_]` of three or more characters before a NUL), or `None` when the table
/// does not hold exactly `count` names (the walk then does not trust the order).
fn name_index(table: &[u8], name: &[u8], count: usize) -> Option<usize> {
    let mut found = None;
    let mut index = 0usize;
    let mut start = 0usize;
    while start < table.len() {
        let rest = &table[start..];
        let run = rest.iter().take_while(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || **b == b'_').count();
        if run >= 3 && rest.get(run) == Some(&0) {
            if &rest[..run] == name {
                found = Some(index);
            }
            index += 1;
            start += run + 1;
        } else {
            start += run.max(1);
        }
    }
    (index == count).then_some(found).flatten()
}

/// The emitters of emitter set `set` of the resource.
pub fn emitters<M: Memory>(mem: &M, resource: usize, set: usize) -> Result<VisualEmitters, Absent> {
    let sets = mem.i32(resource + RESOURCE_SET_COUNT).filter(|n| (1..=4096).contains(n)).ok_or(Absent::Shape)? as usize;
    if set >= sets {
        return Err(Absent::NoEmitters);
    }
    let table = mem.ptr(resource + RESOURCE_SETS).ok_or(Absent::Shape)?;
    let at = table + set * SET_STRIDE;
    let count = mem.i32(at + SET_EMITTER_COUNT).filter(|n| (1..=MAX_EMITTERS as i32).contains(n)).ok_or(Absent::NoEmitters)? as usize;
    let first = mem.ptr(at + SET_EMITTERS).ok_or(Absent::Shape)?;
    let mut out = VisualEmitters { set, count, emitters: [Emitter { emtr: 0, slots: [SLOT_NONE; 3] }; MAX_EMITTERS] };
    for (i, slot) in out.emitters.iter_mut().take(count).enumerate() {
        let node = first + i * EMITTER_STRIDE;
        slot.emtr = mem.ptr(node + EMITTER_EMTR).ok_or(Absent::Shape)?;
        for (k, offset) in EMITTER_SLOTS.iter().enumerate() {
            slot.slots[k] = mem.u64(node + offset).ok_or(Absent::Shape)?;
        }
    }
    Ok(out)
}

/// The texture of the resource that holds descriptor slot `slot`.
pub fn texture_of_slot<M: Memory>(mem: &M, resource: usize, slot: Slot) -> Result<TextureInfo, Absent> {
    if slot == SLOT_NONE {
        return Err(Absent::NoTexture);
    }
    let file = mem.ptr(resource + RESOURCE_TEXTURES).ok_or(Absent::Shape)?;
    let count = mem.i32(resource + RESOURCE_TEXTURE_COUNT).filter(|n| (1..=4096).contains(n)).ok_or(Absent::Shape)? as usize;
    let array = mem.ptr(file + FILE_TEXTURES).ok_or(Absent::Shape)?;
    for index in 0..count {
        let brti = mem.ptr(array + 8 * index).ok_or(Absent::Shape)?;
        if mem.bytes(brti, 4) != Some(&BRTI_MAGIC[..]) {
            return Err(Absent::Shape);
        }
        if mem.u64(brti + BRTI_SLOT) == Some(slot) {
            let (Some(width), Some(height), Some(format)) = (mem.u32(brti + BRTI_WIDTH), mem.u32(brti + BRTI_HEIGHT), mem.u32(brti + BRTI_FORMAT)) else {
                return Err(Absent::Shape);
            };
            if width == 0 || height == 0 || width > 4096 || height > 4096 {
                return Err(Absent::Shape);
            }
            let view = mem.ptr(brti + BRTI_VIEW).ok_or(Absent::Shape)?;
            return Ok(TextureInfo { index, slot, view, width, height, format });
        }
    }
    Err(Absent::NoTexture)
}

/// The emitter binary's fields the chip reads (stream W9's layout; the field meanings are its inference).
pub mod emtr {
    pub const KEY_COUNTS: usize = 0x60;
    pub const COLOUR_SCALE: usize = 0x3b0;
    pub const COLOUR0_KEYS: usize = 0x3c0;
    pub const ALPHA0_KEYS: usize = 0x440;
    pub const COLOUR1_KEYS: usize = 0x4c0;
    pub const ALPHA1_KEYS: usize = 0x540;
    pub const SCALE_KEYS: usize = 0x600;
    pub const KEY_ROWS: usize = 8;
    pub const BLEND: usize = 0x89e;
    pub const SOURCES: usize = 0x99c;
    pub const FIXED0: usize = 0x9a0;
    pub const FIXED1: usize = 0x9b0;
    pub const BASE_SCALE: usize = 0x9c0;
    pub const SAMPLERS: usize = 0x9f0;
    pub const SAMPLER_STRIDE: usize = 0x20;
}

/// Where a colour or alpha comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Fixed,
    Keyed,
    Random,
    Other(u8),
}

impl Source {
    fn of(byte: u8) -> Source {
        match byte {
            0 => Source::Fixed,
            2 => Source::Keyed,
            3 => Source::Random,
            other => Source::Other(other),
        }
    }
}

/// One emitter's colour data, copied out of its binary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmitterColours {
    pub scale: f32,
    /// `[colour0, colour1, alpha0, alpha1]`.
    pub sources: [Source; 4],
    pub fixed0: [f32; 4],
    pub fixed1: [f32; 4],
    /// Key counts of `[colour0, alpha0, colour1, alpha1, scale]`.
    pub counts: [u32; 5],
    pub colour0: [[f32; 4]; emtr::KEY_ROWS],
    pub alpha0: [[f32; 4]; emtr::KEY_ROWS],
    pub colour1: [[f32; 4]; emtr::KEY_ROWS],
    pub alpha1: [[f32; 4]; emtr::KEY_ROWS],
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn row(b: &[u8], at: usize) -> [f32; 4] {
    [f32_at(b, at), f32_at(b, at + 4), f32_at(b, at + 8), f32_at(b, at + 12)]
}

fn rows(b: &[u8], at: usize) -> [[f32; 4]; emtr::KEY_ROWS] {
    let mut out = [[0.0; 4]; emtr::KEY_ROWS];
    for (i, r) in out.iter_mut().enumerate() {
        *r = row(b, at + 16 * i);
    }
    out
}

impl EmitterColours {
    /// Read the colour fields of an emitter binary (`EMTR_BYTES` bytes). `None` when a value is not finite (the layout is not the one the
    /// walk expects).
    pub fn parse(b: &[u8]) -> Option<EmitterColours> {
        if b.len() < EMTR_BYTES {
            return None;
        }
        let mut counts = [0u32; 5];
        for (i, c) in counts.iter_mut().enumerate() {
            *c = u32::from_le_bytes([b[emtr::KEY_COUNTS + 4 * i], b[emtr::KEY_COUNTS + 4 * i + 1], b[emtr::KEY_COUNTS + 4 * i + 2], b[emtr::KEY_COUNTS + 4 * i + 3]]);
        }
        let colours = EmitterColours {
            scale: f32_at(b, emtr::COLOUR_SCALE),
            sources: [
                Source::of(b[emtr::SOURCES]),
                Source::of(b[emtr::SOURCES + 1]),
                Source::of(b[emtr::SOURCES + 2]),
                Source::of(b[emtr::SOURCES + 3]),
            ],
            fixed0: row(b, emtr::FIXED0),
            fixed1: row(b, emtr::FIXED1),
            counts,
            colour0: rows(b, emtr::COLOUR0_KEYS),
            alpha0: rows(b, emtr::ALPHA0_KEYS),
            colour1: rows(b, emtr::COLOUR1_KEYS),
            alpha1: rows(b, emtr::ALPHA1_KEYS),
        };
        let finite = colours.scale.is_finite()
            && colours.fixed0.iter().chain(colours.fixed1.iter()).all(|v| v.is_finite())
            && colours.counts.iter().all(|c| *c <= emtr::KEY_ROWS as u32);
        finite.then_some(colours)
    }

    /// Colour 0's RGB at normalised particle life `t`, by W9's rule: the fixed value, the keyed value at `t`, or the mean of the keys.
    pub fn colour0_at(&self, t: f32) -> [f32; 3] {
        pick(self.sources[0], [self.fixed0[0], self.fixed0[1], self.fixed0[2]], &self.colour0, self.counts[0], t)
    }

    /// Colour 1's RGB at `t`.
    pub fn colour1_at(&self, t: f32) -> [f32; 3] {
        pick(self.sources[1], [self.fixed1[0], self.fixed1[1], self.fixed1[2]], &self.colour1, self.counts[2], t)
    }

    /// Alpha 0 at `t`.
    pub fn alpha0_at(&self, t: f32) -> f32 {
        pick(self.sources[2], [self.fixed0[3]; 3], &self.alpha0, self.counts[1], t)[0]
    }

    /// Alpha 1 at `t`.
    pub fn alpha1_at(&self, t: f32) -> f32 {
        pick(self.sources[3], [self.fixed1[3]; 3], &self.alpha1, self.counts[3], t)[0]
    }
}

fn pick(source: Source, fixed: [f32; 3], keys: &[[f32; 4]; emtr::KEY_ROWS], count: u32, t: f32) -> [f32; 3] {
    let n = (count as usize).min(emtr::KEY_ROWS);
    match source {
        Source::Keyed if n > 0 => keyed(&keys[..n], t),
        Source::Random if n > 0 => {
            let mut sum = [0.0f32; 3];
            for k in &keys[..n] {
                for (s, v) in sum.iter_mut().zip(k.iter()) {
                    *s += v;
                }
            }
            [sum[0] / n as f32, sum[1] / n as f32, sum[2] / n as f32]
        }
        _ => fixed,
    }
}

/// Linear interpolation of `(r, g, b, time)` keys at `t`, clamped to the first and last key.
fn keyed(keys: &[[f32; 4]], t: f32) -> [f32; 3] {
    let first = keys[0];
    if t <= first[3] {
        return [first[0], first[1], first[2]];
    }
    for pair in keys.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if t <= b[3] {
            let span = b[3] - a[3];
            let f = if span > 0.0 { (t - a[3]) / span } else { 1.0 };
            return [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f];
        }
    }
    let last = keys[keys.len() - 1];
    [last[0], last[1], last[2]]
}

/// The pane tint the diagnostic chip uses for an emitter: colour 0 at life 0.3 times the colour scale, clamped (W9 rule 7 without its display gain).
pub fn tint_rgba(c: &EmitterColours) -> [u8; 4] {
    let rgb = c.colour0_at(0.3);
    let scale = if c.scale > 0.0 { c.scale.min(2.0) } else { 1.0 };
    let byte = |v: f32| (v * scale).clamp(0.0, 1.0).mul_add(255.0, 0.5) as u8;
    [byte(rgb[0]), byte(rgb[1]), byte(rgb[2]), 255]
}

/// A byte image of the game's memory for tests and for the walk's own checks: regions of bytes at fixed addresses.
#[cfg(test)]
#[derive(Default)]
pub struct Image {
    regions: Vec<(usize, Vec<u8>)>,
}

#[cfg(test)]
impl Image {
    pub fn new() -> Image {
        Image::default()
    }
    pub fn region(&mut self, base: usize) -> &mut Vec<u8> {
        self.regions.push((base, Vec::new()));
        &mut self.regions.last_mut().unwrap().1
    }
    pub fn put(&mut self, address: usize, data: &[u8]) {
        for (base, bytes) in &mut self.regions {
            if address >= *base && address + data.len() <= *base + bytes.len() {
                bytes[address - *base..address - *base + data.len()].copy_from_slice(data);
                return;
            }
        }
        panic!("address {address:#x} not in a region");
    }
    pub fn put_u64(&mut self, address: usize, v: u64) {
        self.put(address, &v.to_le_bytes());
    }
    pub fn put_u32(&mut self, address: usize, v: u32) {
        self.put(address, &v.to_le_bytes());
    }
}

#[cfg(test)]
impl Memory for Image {
    fn bytes(&self, address: usize, len: usize) -> Option<&[u8]> {
        self.regions.iter().find(|(base, bytes)| address >= *base && address + len <= *base + bytes.len()).map(|(base, bytes)| &bytes[address - base..address - base + len])
    }
}

