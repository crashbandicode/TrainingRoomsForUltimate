//! HUD readback facade (WP-P slice 1, WORKPLAN §2.12 "readback observations beside current
//! behaviour (M2, shadow)"): after every presentation publication and lifecycle event the
//! native card of the owner is read back, value-only, and compared with the expectation
//! derived from the transition (`crate::engine::swap::swap_model::presentation`). Nothing is
//! written; the publishers (`publish_mii_presentation`, `republish_after_rebuild`, the exact
//! portrait job) stay the writers until the M4 facade. A mismatch is a logged `[Hud] mismatch`
//! line, never silent (rule 5).
//!
//! FACTs: the owner's card slot is found through the HUD root `main+0x52c3420` → `+8` → `+0`,
//! records of `0x1d8` bytes with the owner at `+0xd68` and the slot index at `+0xd60`; slots
//! at `hud+0xb0 → +0x1790`, `0x200` bytes each; bust mode `slot+0x1a8`, the construction bind
//! `+0x1a9 == 1`, the texture table index `+0x1ac` (`src/mii_hud_portrait.rs:2068-2089`). No
//! established read offset exists for the card's kind/name text (`redesign/astra/HUD.md` §1),
//! so the kind is compared through the publisher's own `observed`/`presented` receipts and
//! the screenshot remains the acceptance oracle; the readback proves bust/bind state.
use crate::engine::seams::native_fields as fields;
use std::sync::atomic::{AtomicU8, Ordering};
use crate::engine::swap::swap_model::presentation::HudExpectation;

macro_rules! trace {
    ($($arg:tt)*) => { crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}

/// Value-only card observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CardObservation { pub slot: i32, pub bust_mode: u8, pub bound: u8, pub table_index: u32 }

pub(crate) unsafe fn observe(base: usize, owner: u32) -> Result<CardObservation, &'static str> {
    let (slot, slot_ptr) = crate::runtime::roster::character_roster::mii_hud_portrait::slot_of_owner(base, owner)?;
    let p = slot_ptr as usize;
    Ok(CardObservation {
        slot,
        bust_mode: fields::read::<u8>(p, 0x1a8).ok_or("slot +0x1a8 unreadable")?,
        bound: fields::read::<u8>(p, 0x1a9).ok_or("slot +0x1a9 unreadable")?,
        table_index: fields::read::<u32>(p, 0x1ac).ok_or("slot +0x1ac unreadable")?,
    })
}

/// Compare the observation with the expectation: an exact bust must be in bust mode and
/// bound; a kind/pair/default presentation must not show a stale bust.
pub(crate) fn matches(expected: &HudExpectation, card: &CardObservation) -> Result<(), &'static str> {
    let bust = card.bust_mode != 0 && card.bound == 1;
    match (expected.bust.is_some(), bust) {
        (true, false) => Err("exact bust not bound"),
        (false, true) => Err("stale bust bound"),
        _ => Ok(()),
    }
}

/// Shadow readback after a publication or a lifecycle event: logs `[Hud] readback … ok=`
/// and, on a mismatch, `[Hud] mismatch …`. `exact` is whether the transition's presentation
/// is an exact Mii bust (the `PortraitIntent::Exact` decision today). Returns the verdict.
///
/// F-HUD-c (diag builds): for the waiting CPU card (owner 4, not an exact bust) the readback also compares the
/// texture bound on the partner icon with the icon the body's own costume (`R+0x64`) names, and
/// checks the publisher's row colour against it: `icon=ok|differs|unavailable` (`n/a` otherwise),
/// `body_costume`, `row_costume`, `icon_bound`, `icon_expected`. `icon=differs` makes `ok=false`;
/// `icon=unavailable` (no waiting card, no `R`) is reported and leaves `ok` to the card check.
pub(crate) unsafe fn shadow_readback(base: usize, owner: u32, kind: u32, exact: bool, why: &str) -> Option<bool> {
    use crate::engine::swap::swap_model::presentation::{verify_waiting_icon, WAITING_CPU_OWNER};
    let expected = HudExpectation { kind, bust: exact.then_some(crate::engine::swap::swap_model::presentation::TextureKey(0)), pair: false, waiting_icon: None };
    // Diag only: shipped builds write no `[Hud]` line, so they skip the icon lookup as well.
    let icon = (cfg!(feature = "diag") && owner == WAITING_CPU_OWNER && !exact)
        .then(|| crate::runtime::roster::character_roster::waiting_icon_observation(base, kind));
    let observed = icon.and_then(Result::ok);
    let icon_verdict = observed.as_ref().map(verify_waiting_icon);
    let (icon_state, icon_reason) = match (icon, icon_verdict) {
        (Some(Err(reason)), _) => ("unavailable", reason),
        (_, Some(Err(reason))) => ("differs", reason),
        (Some(Ok(_)), _) => ("ok", ""),
        (None, _) => ("n/a", ""),
    };
    let (body_costume, row_costume, icon_bound, icon_expected) =
        observed.map_or((u32::MAX, u32::MAX, 0, 0), |o| (o.body_costume, o.row_costume, o.bound, o.expected));
    let icon_ok = !matches!(icon_verdict, Some(Err(_)));
    if let Some(Err(reason)) = icon_verdict {
        trace!("[Hud] mismatch owner={} kind={:#x} why={} reason={}", owner, kind, why, reason);
    }
    match observe(base, owner) {
        Ok(card) => {
            let verdict = matches(&expected, &card);
            trace!("[Hud] readback owner={} kind={:#x} exact={} why={} ok={} slot={} bust={} bound={} table={} icon={} body_costume={} row_costume={} icon_bound={:#x} icon_expected={:#x} icon_reason=\"{}\" {}",
                owner, kind, exact, why, verdict.is_ok() && icon_ok, card.slot, card.bust_mode, card.bound, card.table_index,
                icon_state, body_costume, row_costume, icon_bound, icon_expected, icon_reason,
                verdict.err().map_or(String::new(), |e| format!("mismatch={e}")));
            if let Err(reason) = verdict { trace!("[Hud] mismatch owner={} kind={:#x} why={} reason={}", owner, kind, why, reason); }
            Some(verdict.is_ok() && icon_ok)
        }
        Err(reason) => {
            trace!("[Hud] readback owner={} kind={:#x} why={} unavailable={} icon={} body_costume={} row_costume={} icon_reason=\"{}\"",
                owner, kind, why, reason, icon_state, body_costume, row_costume, icon_reason);
            None
        }
    }
}

/// State-load class (zero HUD code on that path today, WORKPLAN §2.12 FACT): the card is
/// observed when a load starts and compared when the role's restore completes; a load must
/// change no card. One slot per role (Player 0, CPU 1), pointer-free.
static LOAD_CARD: [AtomicU8; 2] = [AtomicU8::new(u8::MAX), AtomicU8::new(u8::MAX)];

pub(crate) unsafe fn state_load_started(base: usize, role: usize) {
    if role > 1 { return; }
    let word = observe(base, role as u32).map_or(u8::MAX, |c| (c.bust_mode != 0 && c.bound == 1) as u8);
    LOAD_CARD[role].store(word, Ordering::Release);
}

pub(crate) unsafe fn state_load_completed(base: usize, role: usize) {
    if role > 1 { return; }
    let before = LOAD_CARD[role].swap(u8::MAX, Ordering::AcqRel);
    if before == u8::MAX { return; }
    match observe(base, role as u32) {
        Ok(card) => {
            let now = (card.bust_mode != 0 && card.bound == 1) as u8;
            trace!("[Hud] state_load owner={} unchanged={} bust_before={} bust_now={} slot={} table={}",
                role, before == now, before, now, card.slot, card.table_index);
            if before != now { trace!("[Hud] mismatch owner={} why=state load reason=card changed across the load", role); }
        }
        Err(reason) => trace!("[Hud] state_load owner={} unavailable={}", role, reason),
    }
}
