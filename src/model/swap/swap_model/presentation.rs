//! One presentation intent per transition (WP-P, WORKPLAN §2.12; `redesign/fable/HUD.md`
//! §2 shape, `redesign/astra/HUD.md` §2 semantics). What the HUD must show is derived
//! **only** from the validated transition (kind, forms, active member, exact configuration,
//! context) and the form's capability row — never from a receipt or a proof outcome.
//!
//! FACTs this module encodes:
//! - The native HUD reads the card slot of the owner's row (`hud+0xb0`, `+0xd68` owner,
//!   `+0xd60` slot; `src/mii_hud_portrait.rs:2070`), bust mode `slot+0x1a8`, bind `+0x1a9`,
//!   table index `+0x1ac`; a static portrait is drawn by kind otherwise (`:2084-2089`).
//! - A full Reload emits body event `0x1e`, not the portrait event `0x1d`; the mod
//!   synthesises `0x1d` (`src/character_roster.rs:1717-1724`).
//! - Today's intent is the adapter's `PortraitIntent {Exact, Original, NotMii}`
//!   (`character_roster.rs:173-177`), published only when a proof passed (C117 D3, M3).
//! - A refused non-presentation clause must not starve the HUD (Fable rule 1); a structural
//!   Mii resource failure stays a body-transition failure (Astra): `presentation_after_settle`.

#[cfg(test)]
use crate::engine::swap::character_forms::Forms;
#[cfg(test)]
use crate::engine::mii::inventory::MiiClass;
#[cfg(test)]
use crate::engine::mii::lifetime::Configuration;

/// Scene context; part of the target. A context the mechanism does not support is a
/// planner refusal before the transition, never a `HUD skipped` line after a body swap.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context { Training, QpWaiting, LdnWaiting }

#[cfg(test)]
impl Context {
    /// Physical owners whose card this context presents: Training P1/CPU1; waiting rooms
    /// present P1 and the waiting CPU4 card (`refresh_hud`'s two admitted shapes).
    pub fn presents(self, physical_owner: u32) -> bool {
        match self {
            Context::Training => physical_owner <= 1,
            Context::QpWaiting | Context::LdnWaiting => physical_owner == 0 || physical_owner == 4,
        }
    }
}

/// Identity of a rendered exact-Mii texture: the configuration's CharInfo and tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureKey(pub u64);

impl TextureKey {
    #[cfg(test)]
    pub fn of(configuration: &Configuration) -> Self {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let t = configuration.tokens();
        for b in configuration.charinfo().iter().copied()
            .chain(t.outfit.to_le_bytes()).chain(t.hat.to_le_bytes()).chain(t.colour.to_le_bytes()).chain(t.voice.to_le_bytes()) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self(h)
    }
}

/// What the card shows for one owner. The exact-Mii variant names the texture
/// (`TextureKey::of(&configuration)`), not the configuration itself: the HUD compares
/// textures, and the render job receives the configuration from the transition that
/// owns it, so this enum stays a few words wide and `Copy` (no lint allowance).
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presentation {
    /// Ordinary fighter, a PT/Aegis form, Solo Popo, Kirby (the hat is body-side).
    Kind { kind: u32, active_index: u8 },
    /// Ice Climbers pair plate (two simultaneous actors).
    Pair { lead: u32 },
    /// Armored/costumed bust rendered from the configuration this key identifies.
    ExactMii { class: u32, texture: TextureKey },
    /// Preset or native-constructed Mii: the static class portrait.
    DefaultMii { class: u32 },
}

#[cfg(test)]
impl Presentation {
    pub fn kind(&self) -> u32 {
        match *self {
            Presentation::Kind { kind, .. } => kind,
            Presentation::Pair { lead } => lead,
            Presentation::ExactMii { class, .. } | Presentation::DefaultMii { class } => class,
        }
    }
    /// The live-kind fallback that keeps the card truthful (never an exact claim).
    pub fn live_kind(kind: u32, active_index: u8) -> Self {
        match MiiClass::from_kind(kind) {
            Some(_) => Presentation::DefaultMii { class: kind },
            None => Presentation::Kind { kind, active_index },
        }
    }
}

/// Where the publisher runs relative to the native transition (one answer per kind).
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ordering { AfterReady, AtAcknowledge, AfterBuildVerified, OnNativeEvent }

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionKind { CanonicalReload, WarmSwap, PhaseBuild, NativeEvent }

#[cfg(test)]
pub fn ordering_for(kind: TransitionKind) -> Ordering {
    match kind {
        TransitionKind::CanonicalReload => Ordering::AfterReady,
        TransitionKind::WarmSwap => Ordering::AtAcknowledge,
        TransitionKind::PhaseBuild => Ordering::AfterBuildVerified,
        TransitionKind::NativeEvent => Ordering::OnNativeEvent,
    }
}

/// What the card must show: static kind key, or bust mode with a registered texture,
/// or the pair plate; the waiting-card icon in a waiting room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudExpectation { pub kind: u32, pub bust: Option<TextureKey>, pub pair: bool, pub waiting_icon: Option<u32> }

/// Scene epoch, entry incarnation, logical and physical owner, transition revision.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp { pub scene_epoch: u64, pub incarnation: usize, pub logical_owner: u32, pub physical_owner: u32, pub revision: u64 }

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BustRule { Never, ExactWhenConfigured }

/// The capability-table `presentation` row (WP1 table gains it; defined here until then).
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationRule { pub kind_event: bool, pub bust: BustRule, pub pair_plate: bool, pub waiting_icon: bool, pub native_form_follow: bool }

#[cfg(test)]
impl PresentationRule {
    /// One row per form: Mii classes may show a bust; a two-actor form is the pair plate;
    /// a multi-kind single-actor form (PT, Aegis) follows native form changes.
    pub fn for_form(kind: u32, forms: Forms) -> Self {
        let mii = MiiClass::from_kind(kind).is_some();
        PresentationRule {
            kind_event: true,
            bust: if mii { BustRule::ExactWhenConfigured } else { BustRule::Never },
            pair_plate: forms.actor_count == 2,
            waiting_icon: true,
            native_form_follow: forms.count > 1 && forms.actor_count == 1,
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported { Context { context: Context, physical_owner: u32 }, Form(&'static str) }

/// Derived once at preflight; immutable for the transition.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationIntent {
    pub stamp: Stamp,
    pub presentation: Presentation,
    pub forms: Forms,
    pub active: u8,
    pub context: Context,
    pub rule: PresentationRule,
}

#[cfg(test)]
impl PresentationIntent {
    /// The derivation from the validated transition's parts (kind, forms, active member,
    /// exact configuration, context, stamp). `from_target(&Target, …)` is this function
    /// applied to WP1's `Target` once it lands; nothing here reads a receipt.
    pub fn from_parts(kind: u32, forms: Forms, active: u8, configuration: Option<Configuration>, context: Context, stamp: Stamp)
        -> Result<Self, Unsupported> {
        if !context.presents(stamp.physical_owner) {
            return Err(Unsupported::Context { context, physical_owner: stamp.physical_owner });
        }
        if usize::from(active) >= forms.count.max(1) as usize { return Err(Unsupported::Form("active member outside the form")); }
        let rule = PresentationRule::for_form(kind, forms);
        let presentation = match (MiiClass::from_kind(kind), configuration, rule.pair_plate) {
            (_, _, true) => Presentation::Pair { lead: kind },
            (Some(_), Some(configuration), _) if rule.bust == BustRule::ExactWhenConfigured
                && configuration.canonical().class as u32 == kind =>
                Presentation::ExactMii { class: kind, texture: TextureKey::of(&configuration) },
            (Some(_), _, _) => Presentation::DefaultMii { class: kind },
            (None, _, _) => Presentation::Kind { kind, active_index: active },
        };
        Ok(Self { stamp, presentation, forms, active, context, rule })
    }

    /// Golden bridge from today's adapter decision (`PortraitIntent`): `Exact(c)` is the
    /// exact bust, `Original` of a Mii class is native's own portrait (default), `NotMii`
    /// is the kind. The pair plate and context admission come from the form and scene.
    pub fn from_portrait_intent(kind: u32, forms: Forms, active: u8, exact: Option<Configuration>, context: Context, stamp: Stamp)
        -> Result<Self, Unsupported> {
        Self::from_parts(kind, forms, active, exact, context, stamp)
    }

    pub fn expectation(&self) -> HudExpectation {
        let waiting = !matches!(self.context, Context::Training) && self.stamp.physical_owner == 4;
        let kind = self.presentation.kind();
        HudExpectation {
            kind,
            bust: match self.presentation { Presentation::ExactMii { texture, .. } => Some(texture), _ => None },
            pair: matches!(self.presentation, Presentation::Pair { .. }),
            waiting_icon: waiting.then_some(kind),
        }
    }

    /// A validated native form change of the same group (PT/Aegis Down-B): same stamp,
    /// the new active member, the kind presentation (never an exact Mii).
    pub fn after_native_form(&self, kind: u32, active: u8) -> Self {
        Self { presentation: Presentation::Kind { kind, active_index: active }, active, ..*self }
    }
}

/// How a settle outcome changes what is published (rule 3 of WORKPLAN §2.12): a body
/// proven live with the target kind is presented even when a non-presentation clause
/// degraded — as the live kind, never as an exact claim; a diverged body publishes the
/// live kind read back so the card never lies.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settlement { Adopted, Degraded, Diverged { live_kind: u32, active_index: u8 } }

#[cfg(test)]
pub fn presentation_after_settle(intent: &PresentationIntent, settlement: Settlement) -> Presentation {
    match settlement {
        Settlement::Adopted => intent.presentation,
        Settlement::Degraded => Presentation::live_kind(intent.presentation.kind(), intent.active),
        Settlement::Diverged { live_kind, active_index } => Presentation::live_kind(live_kind, active_index),
    }
}

/// Compare a value-only card observation with the expectation. `bust_key` is the key the
/// readback resolved for the bound texture (`None` when the slot is not in bust mode).
#[cfg(test)]
pub fn verify(expected: &HudExpectation, observed_kind: Option<u32>, bust_key: Option<TextureKey>, pair: bool, waiting_icon: Option<u32>)
    -> Result<(), &'static str> {
    if let Some(k) = observed_kind { if k != expected.kind { return Err("card kind differs from the expectation"); } }
    if expected.bust != bust_key { return Err(if expected.bust.is_some() { "exact bust not bound" } else { "stale bust bound" }); }
    if expected.pair != pair { return Err("pair plate differs"); }
    if expected.waiting_icon != waiting_icon { return Err("waiting icon differs"); }
    Ok(())
}

// F-HUD-c: the waiting-room CPU card's icon names the costume its body wears
// (`refactor/hud-costume-lobby/PROGRESS.md`; listings `hud-costume-lobby/re/`, 13.0.5 ELF `0789f0a4…`).
//
// FACTs:
// - One byte is the entry's costume everywhere. The waiting entry allocation copies the presentation row's colour
//   `E+0x19` (`E = main+0x52c5730 + 0x260*id`) into the creation descriptor's `+0x41` (`0x14ec1f8–0x14ec204`) and creates
//   the entry with it (`0x678150` at `0x14ec2b8`); configure writes the resource owner's `R+0x64` from `desc+0x41`, and
//   every body of the entry keys its files on `R+0x64` through the selector (`identity_view::native_costume`). A TM swap
//   keeps `desc+0x41` (`mii_selector_native.rs` "native O header"), so the row, `R+0x64` and the body agree.
// - Training's card reads that byte: the portrait setter `0x13639d0` (slot `+0x70` of the Training inner vtables
//   `0x5062780`/`0x5062ba0` and of the waiting inner vtable `0x5062f60`) loads `E+0x19` of the owner's presentation row
//   (`0x1363b08`) and builds the portrait with it (`0x138a3a0` at `0x1363b28`).
// - The waiting card's partner icon is built natively once, from the member record's costume (`0x13ce298
//   ldr w2,[rec+0x70]`, `0x323f330(cache, ui_chara, costume, 0)` at `0x13ce2a4`, `0x3778b80` at `0x13ce2b4`); a TM swap
//   repaints it with the same two calls (`character_roster::waiting_icon`). `0x323f330` forwards `(ui_chara, costume,
//   type)` to the key builder `0x3238a70` (`chara_<type>_<name>` + `_%02d` + `.bntx`), which swaps costumes 0 and 1 itself
//   for `ui_chara_robot` (`0xe18857219`) in the regions the body selector swaps them (`0x3238ac4–0x3238b68`).
// - M5g passed the literal costume 0 there and preloaded only the costume-0 icons: the defect.

/// The presentation rows `E` (`main+0x52c5730`, eight of `0x260` bytes) and the colour byte in each.
pub const PRESENTATION_ROWS_OFFSET: usize = 0x52c5730;
pub const PRESENTATION_ROW_STRIDE: usize = 0x260;
pub const PRESENTATION_ROW_COLOUR: usize = 0x19;
/// The waiting room's CPU entry: owner 4, whose row is `E[4]` (`refresh_hud`'s `waiting_cpu` shape).
pub const WAITING_CPU_OWNER: u32 = 4;

/// The costume argument the waiting-card icon lookup `0x323f330(cache, ui_chara(kind), costume, 0)` gets for a body of
/// `kind` on an entry whose costume is `entry_costume` (`E+0x19` = `desc+0x41` = `R+0x64`): the raw entry costume, as
/// native passes it, except for a Mii class, whose body wears costume 0 (the selector), so its icon names 0 as before.
pub fn waiting_icon_costume(kind: u32, entry_costume: u32) -> u32 {
    if (crate::engine::swap::identity_view::MII_FIRST..=crate::engine::swap::identity_view::MII_LAST).contains(&kind) {
        0
    } else {
        entry_costume
    }
}

/// The costume the key builder `0x3238a70` writes into the icon's file name for `kind`'s ui_chara and a costume argument.
#[cfg(test)]
pub const fn icon_file_costume(kind: u32, costume: u32, region: u32) -> u32 {
    if kind == crate::engine::swap::identity_view::ROB && crate::engine::swap::identity_view::region_swaps(region) {
        match costume {
            0 => 1,
            1 => 0,
            other => other,
        }
    } else {
        costume
    }
}

/// Value-only readback of the waiting card's icon (`[Hud] readback … icon=`): the texture word bound on the partner
/// picture (`picture+0x130`, written by both branches of `0x379e580`, `0x379e674` and `0x379e854`) against the cached
/// icon the body's own costume names (`**(0x323f330(…, waiting_icon_costume(kind, R+0x64), 0) + 8)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitingIconObservation {
    /// `R+0x64` of the CPU entry: what the body's files are keyed on.
    pub body_costume: u32,
    /// `E[4]+0x19`: what the publisher read.
    pub row_costume: u32,
    pub bound: usize,
    pub expected: usize,
}

pub fn verify_waiting_icon(o: &WaitingIconObservation) -> Result<(), &'static str> {
    if o.expected == 0 {
        return Err("no cached icon for the body's kind and costume");
    }
    if o.row_costume != o.body_costume {
        return Err("presentation row colour differs from the body's costume");
    }
    if o.bound != o.expected {
        return Err("bound icon is not the body's kind and costume");
    }
    Ok(())
}

#[cfg(test)]
mod waiting_icon_tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::swap::identity_view::native_costume;

    const REGIONS: [u32; 20] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 31, 32, 255, u32::MAX];

    /// The icon names exactly the costume the body's files are keyed with, for every kind, entry costume and region.
    fn the_icon_names_the_costume_the_body_wears_for_every_kind_region_and_costume() {
        for kind in 0..0x60u32 {
            for costume in 0..8u32 {
                for &region in &REGIONS {
                    assert_eq!(
                        icon_file_costume(kind, waiting_icon_costume(kind, costume), region),
                        native_costume(kind, costume, region),
                        "kind {:#x} costume {} region {}", kind, costume, region);
                }
            }
        }
    }

    fn the_readback_passes_only_the_body_costume_icon() {
        let ok = WaitingIconObservation { body_costume: 4, row_costume: 4, bound: 0x1000, expected: 0x1000 };
        assert_eq!(verify_waiting_icon(&ok), Ok(()));
        // M5g: the costume-0 texture bound while the body wears costume 4.
        let m5g = WaitingIconObservation { bound: 0x2000, ..ok };
        assert_eq!(verify_waiting_icon(&m5g), Err("bound icon is not the body's kind and costume"));
        let unloaded = WaitingIconObservation { expected: 0, ..ok };
        assert_eq!(verify_waiting_icon(&unloaded), Err("no cached icon for the body's kind and costume"));
        let diverged = WaitingIconObservation { row_costume: 3, ..ok };
        assert_eq!(verify_waiting_icon(&diverged), Err("presentation row colour differs from the body's costume"));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_icon_names_the_costume_the_body_wears_for_every_kind_region_and_costume", the_icon_names_the_costume_the_body_wears_for_every_kind_region_and_costume as fn()),
            ("the_readback_passes_only_the_body_costume_icon", the_readback_passes_only_the_body_costume_icon as fn()),
        ]);
    }
    }
}
