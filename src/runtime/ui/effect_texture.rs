//! The game's own DI / SDI effect textures on picture panes (stream u4b; SPEC-core-07 section 4.6, question 17).
//!
//! The effect textures sit inside `effect/system/common/ef_common.eff`, which no UI path hash names, so the UI texture loader of
//! `texture::load` cannot ask for them. They do not have to be loaded: the game already holds the file. Its effect manager loaded the
//! file once, kept the bytes, and its particle system built every texture of the file's `BNTX` in place (memory pool, texture, texture
//! view). This module reads the manager's and the system's structures (offsets and instructions: `sites_u4b`, `model::effect_texture`),
//! finds the emitters of the `SYS_VECTOR` (DI) and `SYS_HITSTOP_SPARK` (SDI) effects in the resident data, and hands the texture view
//! the game built for each emitter texture to the UI texture code (`texture::load_resident`), which registers it for a picture pane.
//!
//! Nothing is read from the card, decoded or uploaded by the plugin and nothing is baked; an effect mod that replaces the file shows,
//! because the resident data is what the game draws. Until the game has loaded the file (or if any guard or shape check fails) the
//! walk answers `Err`, and the chip keeps the clean room's arrow.
//!
//! Threads: every call runs on the UI job thread inside the layout tick; the manager's tables are read, never written. Nothing here
//! allocates on the plugin's heap.

use core::slice;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::model::effect_texture::{self as facts, Absent, Common, EmitterColours, Memory, TextureInfo, Visual, EMTR_BYTES};
use crate::model::guard::GuardOutcome;
use crate::model::sites_u4b::{self, SITES};
use crate::model::sites_ui::decode_adrp_ldr;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::game;
use crate::runtime::ui::texture::{self, Texture};

/// Every row passed and the manager's slot decoded; set by `install`.
static USABLE: AtomicBool = AtomicBool::new(false);
/// Absolute address of the slot that names the effect manager's instance variable (decoded from the game's own `adrp`/`ldr` words).
static MANAGER_SLOT_ADDRESS: AtomicUsize = AtomicUsize::new(0);

fn at(offset: usize) -> usize {
    game::text_base() + offset
}

/// The live memory of the game, as the pure walk reads it. The walk checks every pointer it follows for a plausible address and every
/// structure it enters for its own signature; this impl adds no check of its own.
pub struct Live;

impl Memory for Live {
    fn bytes(&self, address: usize, len: usize) -> Option<&[u8]> {
        if !facts::in_range(address) || len == 0 {
            return None;
        }
        // SAFETY: the address is inside user space (`in_range`; pointers the walk follows are also word aligned, `plausible`); every address the walk passes was read from a structure
        // of the game the guards identified, and the memory of the effect manager and particle system stays mapped while the game runs.
        Some(unsafe { slice::from_raw_parts(address as *const u8, len) })
    }
}

/// Compare every row of `sites_u4b::SITES` with the bytes now in the game and decode the manager's slot from the instance words. The
/// helper is usable only when all pass (and the texture helper of `sites_u4` is usable too).
pub fn install() -> [GuardOutcome; SITES.len()] {
    let mut found = [GuardOutcome::Stock; SITES.len()];
    for (slot, site) in found.iter_mut().zip(SITES.iter()) {
        *slot = site.outcome(&game::read_window(site.offset));
    }
    let all = found.iter().all(|o| o.usable());
    let mut slot = 0usize;
    if all {
        let window = game::read_window(sites_u4b::EFFMGR_INSTANCE_WORDS_OFFSET);
        let word = |i: usize| u32::from_le_bytes([window[4 * i], window[4 * i + 1], window[4 * i + 2], window[4 * i + 3]]);
        let pc = at(sites_u4b::EFFMGR_INSTANCE_WORDS_OFFSET) as u64;
        // `adrp x8, page; ldr x8, [x8, #off]; ldr x0, [x8]`: the slot is the one the constant names, and the instance is read through it.
        slot = match decode_adrp_ldr(pc, word(0), word(1)) {
            Some(address) if address == at(facts::MANAGER_SLOT) as u64 && word(2) == LDR_X0_X8 => address as usize,
            _ => 0,
        };
    }
    MANAGER_SLOT_ADDRESS.store(slot, Ordering::Release);
    USABLE.store(all && slot != 0, Ordering::Release);
    diagnostic(format_args!("[EffTex] install usable={} guards={found:?} manager_slot={slot:#x}", all && slot != 0));
    found
}

/// `ldr x0, [x8]`.
pub const LDR_X0_X8: u32 = 0xf940_0100;

/// Whether the walk may run: its rows passed and were granted, and the texture helper it hands views to is usable.
pub fn usable() -> bool {
    USABLE.load(Ordering::Acquire) && SITES.iter().all(|site| game::granted(site.offset)) && texture::usable()
}

/// The effect manager object, or `None` before the game created it.
pub fn manager() -> Option<usize> {
    let slot = MANAGER_SLOT_ADDRESS.load(Ordering::Acquire);
    if slot == 0 {
        return None;
    }
    // The slot holds the address of `instance_`, which holds the manager.
    let variable = Live.ptr(slot)?;
    Live.ptr(variable)
}

/// The resident `ef_common.eff`: its manager entry, bytes and particle-system resource.
pub fn locate() -> Result<Common, Absent> {
    if !usable() {
        return Err(Absent::Manager);
    }
    facts::find_common(&Live, manager().ok_or(Absent::Manager)?)
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
    pub colours: Option<EmitterColours>,
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
pub fn pictures(visual: Visual) -> Result<Pictures, Absent> {
    let common = locate()?;
    let set = facts::effect_set(&Live, common.effn, visual.effect_name())?;
    let emitters = facts::emitters(&Live, common.resource, set)?;
    let mut out = Pictures { set, emitters: emitters.count, count: 0, pictures: [None; MAX_PICTURES] };
    for (e, emitter) in emitters.emitters.iter().take(emitters.count).enumerate() {
        let colours = Live.bytes(emitter.emtr, EMTR_BYTES).and_then(EmitterColours::parse);
        for (k, slot) in emitter.slots.iter().enumerate() {
            if *slot == facts::SLOT_NONE || out.pictures.iter().flatten().any(|p| p.texture.slot == *slot) {
                continue;
            }
            // A slot this resource does not hold belongs to the parent resource the particle system falls back to (its slot resolver
            // recurses into it): the chip does not draw that sampler, the others stay.
            let texture = match facts::texture_of_slot(&Live, common.resource, *slot) {
                Ok(texture) => texture,
                Err(Absent::NoTexture) => continue,
                Err(other) => return Err(other),
            };
            if out.count < MAX_PICTURES {
                out.pictures[out.count] = Some(Picture { texture, emitter: e, sampler: k, tint: colours.as_ref().map_or([255; 4], facts::tint_rgba), colours });
                out.count += 1;
            }
        }
    }
    if out.count == 0 {
        return Err(Absent::NoTexture);
    }
    Ok(out)
}

/// A UI texture for `picture`: the game's own registration of the resident texture view (`texture::load_resident`). The texture is released
/// like any other (unbind its pane, then `texture::retire`).
///
/// # Safety
/// On the UI job thread, inside the layout tick; the picture came from `pictures` in this effect's life (the resident resource still exists).
pub unsafe fn texture_of(picture: &Picture) -> Option<Texture> {
    let (w, h) = (u16::try_from(picture.texture.width).ok()?, u16::try_from(picture.texture.height).ok()?);
    texture::load_resident(picture.texture.view, w, h)
}

/// Upper bound the survey reports of the manager's entries (diagnostics).
#[cfg(feature = "diag")]
pub const SURVEY_ENTRIES: usize = facts::ENTRY_COUNT;

/// Diag: one line per used manager entry (key, resource index, hash40, bytes address, first bytes of the file), so a run shows which files the
/// game keeps resident.
#[cfg(feature = "diag")]
pub fn survey(mut line: impl FnMut(core::fmt::Arguments<'_>)) {
    let Some(manager) = manager() else {
        line(format_args!("survey manager=none"));
        return;
    };
    line(format_args!("survey manager={manager:#x} system={:?} resources={:?}", Live.ptr(manager + facts::MANAGER_SYSTEM), Live.ptr(manager + facts::MANAGER_SYSTEM).and_then(|s| Live.ptr(s + facts::SYSTEM_RESOURCES))));
    for i in 0..SURVEY_ENTRIES {
        let base = manager + facts::ENTRY_BASE + i * facts::ENTRY_STRIDE;
        let (Some(key), Some(hash)) = (Live.i32(base + facts::ENTRY_KEY), Live.u64(base + facts::ENTRY_HASH)) else { continue };
        if key == -1 {
            continue;
        }
        let data = Live.u64(base + facts::ENTRY_DATA).unwrap_or(0);
        let magic = Live.ptr(base + facts::ENTRY_DATA).and_then(|d| Live.bytes(d, 4)).map(|b| [b[0], b[1], b[2], b[3]]);
        let effn = Live.ptr(base + facts::ENTRY_DATA);
        let sets = effn.map(|d| (facts::effect_set(&Live, d, Visual::Di.effect_name()), facts::effect_set(&Live, d, Visual::Sdi.effect_name())));
        line(format_args!(
            "survey entry={i} key={key:#x} resource={:?} hash40={:#x} data={data:#x} magic={magic:?} refs={:?} di_sdi_sets={sets:?}",
            Live.i32(base + facts::ENTRY_RESOURCE),
            hash & facts::HASH_MASK,
            Live.i32(base + 0x98)
        ));
    }
}

/// Diag: the structures of one visual, one line per emitter and texture (slots, sizes, formats, colour fields).
#[cfg(feature = "diag")]
pub fn describe(visual: Visual, mut line: impl FnMut(core::fmt::Arguments<'_>)) {
    let common = match locate() {
        Ok(c) => c,
        Err(why) => {
            line(format_args!("describe {visual:?} locate={why:?}"));
            return;
        }
    };
    line(format_args!("describe {visual:?} common={common:?}"));
    match facts::effect_set(&Live, common.effn, visual.effect_name()) {
        Ok(set) => {
            line(format_args!(
                "describe {visual:?} set={set} resource sets={:?} textures={:?} file={:?}",
                Live.i32(common.resource + facts::RESOURCE_SET_COUNT),
                Live.i32(common.resource + facts::RESOURCE_TEXTURE_COUNT),
                Live.ptr(common.resource + facts::RESOURCE_TEXTURES)
            ));
            match facts::emitters(&Live, common.resource, set) {
                Ok(e) => {
                    for (i, em) in e.emitters.iter().take(e.count).enumerate() {
                        line(format_args!("describe {visual:?} raw emitter={i} emtr={:#x} slots={:#x?} magic={:?}", em.emtr, em.slots, Live.bytes(em.emtr, 4).map(|b| [b[0], b[1], b[2], b[3]])));
                    }
                }
                Err(why) => line(format_args!("describe {visual:?} emitters={why:?}")),
            }
        }
        Err(why) => line(format_args!("describe {visual:?} effect_set={why:?}")),
    }
    match pictures(visual) {
        Ok(p) => {
            line(format_args!("describe {visual:?} set={} emitters={} pictures={}", p.set, p.emitters, p.count));
            for picture in p.pictures.iter().flatten() {
                let t = picture.texture;
                line(format_args!(
                    "describe {visual:?} emitter={} sampler={} slot={:#x} view={:#x} index={} {}x{} format={:#x} tint={:?} colours={:?}",
                    picture.emitter, picture.sampler, t.slot, t.view, t.index, t.width, t.height, t.format, picture.tint, picture.colours.map(|c| (c.scale, c.sources, c.fixed0, c.fixed1))
                ));
            }
        }
        Err(why) => line(format_args!("describe {visual:?} pictures={why:?}")),
    }
}
