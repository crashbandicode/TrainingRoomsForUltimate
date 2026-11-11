//! Extra-capacity creation wait (lobby-preload D). Values only.
//!
//! A creation whose native preflight refuses is evaluated again on every helper loader callback.
//! That repetition is the designed wait, not a retry loop (`native_source_capacity::managed::preflight_for`).
//! Before D nothing said so: a waiting room whose visible CPU was an exact Mii reported
//! `[Preload] … planned=5 … Loading` for the whole visit and never created extras 7/3/2
//! (agent-online sp1 L12339 on M5f, l1b L13265 on M5e). This module names the check that refused,
//! decides when that is logged (once per scene, extra and reason) and how many plan targets the
//! readiness line counts while a refusal stands. It never admits, refuses, retries or plans.
//!
//! lobby-preload A (operator decision pending) adds the value rule that admits each layout owner
//! beside a creation, `owner_admission`, moved here from `native_source_capacity` so the host
//! gates test it. A changes one thing in that rule: a count-1 exact Mii is admitted in a visible
//! owner (index 0/1) as it already was in a helper (index 2/3), and only with
//! `ExactMiiProof::Proven`. For a visible owner that proof is the ordinary Mii adapter's committed
//! authority (`role_owner::Committed::permits`), rederived under the native pool lock.

use crate::engine::mii::inventory::MiiClass;
use core::fmt;

/// Restarts of the creation stability run, in one scene, after which `Refusal::Unstable` is noted:
/// about two seconds of loader callbacks whose preflight differed from the previous one.
pub const UNSTABLE_RESTARTS: u16 = 120;
/// Stable callbacks from one helper only (the waiting creation needs both) before `OneHelper` is noted.
pub const ONE_HELPER_TICKS: u16 = 300;

/// The first check that refused a creation preflight, in `preflight_for` order. `index` is the
/// layout owners-array index: 0/1 the visible owners, 2/3 the helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No qualified capacity layout (Training, LDN or Quickplay waiting) right now.
    Layout,
    /// The extra is not this layout's compiled extra.
    Extra,
    /// Another claimant holds the extra.
    Claimed,
    /// The callback is not one of the layout's helper owners.
    Callback,
    /// The earlier extras are not registered in creation order.
    Order,
    /// The native scene snapshot could not be read.
    Snapshot,
    /// The entry count or the layout's creation shape differs.
    Shape,
    /// A native backup root is set.
    Backup,
    /// The extra's own roots or selection byte are occupied.
    Occupied,
    /// A layout owner is none of the admitted shapes (ordinary, grouped, ready bank, proven exact
    /// Mii base helper). `proof` is the exact-Mii proof result for that owner.
    Owner { index: u8, count: u32, kind: u32, proof: &'static str },
    /// A layout owner's alias root differs from its resource root.
    Alias { index: u8 },
    /// A visible owner's forms are not ready in its admitted shape.
    Forms { index: u8 },
    /// A helper owner is not parked ready.
    Parked { index: u8 },
    /// A waiting helper owner is not ready.
    HelperReady { index: u8 },
    /// A layout owner's resource object is unreadable.
    Resources,
    /// A layout owner's resource kind word differs from its entry's first kind.
    ResourceKind { index: u8 },
    /// The extra's input rows are not the expected unclaimed rows.
    Input,
    /// A visible owner's creation capture failed; `check` names the failed check.
    Visible { index: u8, check: &'static str },
    /// A grouped helper owner's parked group could not be captured.
    ParkedGroups,
    /// The extra's input records are claimed.
    Unclaimed,
    /// The preflight differed between callbacks `UNSTABLE_RESTARTS` times in this scene.
    Unstable,
    /// Only one helper's callbacks saw the stable preflight for `ONE_HELPER_TICKS` callbacks.
    OneHelper,
}

impl Refusal {
    fn code(self) -> u8 {
        match self {
            Self::Layout => 0,
            Self::Extra => 1,
            Self::Claimed => 2,
            Self::Callback => 3,
            Self::Order => 4,
            Self::Snapshot => 5,
            Self::Shape => 6,
            Self::Backup => 7,
            Self::Occupied => 8,
            Self::Owner { .. } => 9,
            Self::Alias { .. } => 10,
            Self::Forms { .. } => 11,
            Self::Parked { .. } => 12,
            Self::HelperReady { .. } => 13,
            Self::Resources => 14,
            Self::ResourceKind { .. } => 15,
            Self::Input => 16,
            Self::Visible { .. } => 17,
            Self::ParkedGroups => 18,
            Self::Unclaimed => 19,
            Self::Unstable => 20,
            Self::OneHelper => 21,
        }
    }

    fn index(self) -> u8 {
        match self {
            Self::Owner { index, .. } | Self::Alias { index } | Self::Forms { index }
            | Self::Parked { index } | Self::HelperReady { index } | Self::ResourceKind { index }
            | Self::Visible { index, .. } => index.min(3),
            _ => 0,
        }
    }

    /// Log once key: the check and the owner it refused, not the details.
    pub fn key(self) -> u8 {
        self.code() * 4 + self.index()
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Layout => f.write_str("layout"),
            Self::Extra => f.write_str("extra"),
            Self::Claimed => f.write_str("claimed"),
            Self::Callback => f.write_str("callback"),
            Self::Order => f.write_str("order"),
            Self::Snapshot => f.write_str("snapshot"),
            Self::Shape => f.write_str("shape"),
            Self::Backup => f.write_str("backup"),
            Self::Occupied => f.write_str("occupied"),
            Self::Owner { index, count, kind, proof } => write!(f,
                "owner index={} count={} kind=0x{:x} proof={} (not ordinary, grouped, a ready bank or a proven exact Mii)",
                index, count, kind, proof),
            Self::Alias { index } => write!(f, "alias index={}", index),
            Self::Forms { index } => write!(f, "forms index={}", index),
            Self::Parked { index } => write!(f, "parked index={}", index),
            Self::HelperReady { index } => write!(f, "helper-ready index={}", index),
            Self::Resources => f.write_str("resources"),
            Self::ResourceKind { index } => write!(f, "resource-kind index={}", index),
            Self::Input => f.write_str("input"),
            Self::Visible { index, check } => write!(f, "visible index={} check=\"{}\"", index, check),
            Self::ParkedGroups => f.write_str("parked-groups"),
            Self::Unclaimed => f.write_str("unclaimed"),
            Self::Unstable => write!(f, "unstable (preflight changed {} times)", UNSTABLE_RESTARTS),
            Self::OneHelper => write!(f, "one-helper ({} callbacks)", ONE_HELPER_TICKS),
        }
    }
}

/// One extra's creation wait in one scene `E`. A new scene starts empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Waiting<E> {
    epoch: Option<E>,
    logged: u128,
    restarts: u16,
}

impl<E: Copy + Eq> Waiting<E> {
    pub const NEW: Self = Self { epoch: None, logged: 0, restarts: 0 };

    fn scene(&mut self, epoch: E) {
        if self.epoch != Some(epoch) {
            *self = Self { epoch: Some(epoch), ..Self::NEW };
        }
    }

    /// True the first time `refusal`'s key is noted in `epoch`: the caller logs it then.
    pub fn noted(&mut self, epoch: E, refusal: Refusal) -> bool {
        self.scene(epoch);
        let bit = 1u128 << refusal.key();
        let first = self.logged & bit == 0;
        self.logged |= bit;
        first
    }

    /// A stability run restarted because the preflight changed. True exactly once per scene,
    /// at `UNSTABLE_RESTARTS`.
    pub fn restarted(&mut self, epoch: E) -> bool {
        self.scene(epoch);
        self.restarts = self.restarts.saturating_add(1);
        self.restarts == UNSTABLE_RESTARTS
    }
}

/// The readiness line's capacity while a creation refusal stands for the extra at `position` in the
/// scene's creation order: the helpers plus the extras created before it. Planning, metrics and
/// creation keep `limit`; this is only what the line counts.
pub fn held_limit(limit: usize, helpers: usize, position: Option<usize>) -> usize {
    position.map_or(limit, |position| limit.min(helpers + position))
}

/// Result of the exact-Mii proof for one layout owner. A Mii kind alone is not a proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExactMiiProof {
    /// Not a count-1 installed Mii owner, or that owner's proof is not compiled.
    NotApplicable,
    /// Another helper load or capture owns the native state. Not admission and not a retry.
    Busy,
    /// Helper (index 2/3): `mii_managed_helper::ready_checked` matched the committed payload.
    /// Visible (index 0/1): the owner rederived under the pool lock is exactly the authority the
    /// ordinary Mii adapter committed for it, with its original row kind and committed Mii kind.
    Proven,
    /// Missing, stale or different authority or payload.
    Refused,
}

impl ExactMiiProof {
    /// The creation-waiting line's name for this result.
    pub fn name(self) -> &'static str {
        match self {
            Self::NotApplicable => "not-applicable",
            Self::Busy => "busy",
            Self::Proven => "proven",
            Self::Refused => "refused",
        }
    }
}

/// One already-present layout owner while an extra is created, as the preflight read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnerFacts {
    /// Owners-array index: 0/1 visible, 2/3 helpers.
    pub index: usize,
    pub count: u32,
    pub installed: u64,
    pub kind: u32,
    /// `native_source::ordinary_kind(kind)`: one non-Mii form and actor.
    pub ordinary: bool,
    /// `Forms::starting(kind).count`.
    pub starting_forms: u32,
    pub exact_mii: ExactMiiProof,
}

/// The admission shapes compiled into the plugin build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Compiled {
    /// `source-ldn-grouped-helper-capacity-probe` or `managed-grouped-source-alpha`.
    pub grouped_helper: bool,
    /// `managed-grouped-source-alpha` (grouped helpers in Training too).
    pub managed_grouped: bool,
    /// `managed-visible-grouped-source-alpha`.
    pub visible_grouped: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnerAdmission {
    pub admits: bool,
    pub grouped_visible: bool,
}

/// One already-present layout owner while an extra is created: ordinary; a grouped helper; a
/// grouped visible owner; or a count-1 installed exact Mii with a proven exact payload, visible or
/// helper (lobby-preload A; before A only helpers). The native body bank's visible override, the
/// alias/forms/parked/ready checks, the resource kind words, the input rows and the visible
/// creation capture still follow in `preflight_for`, and `create_reserved` compares them after
/// the native call.
pub fn owner_admission(training: bool, facts: OwnerFacts, compiled: Compiled) -> OwnerAdmission {
    let ordinary = facts.count == 1 && facts.installed == 1 && facts.ordinary;
    let grouped_helper = compiled.grouped_helper && (compiled.managed_grouped || !training)
        && facts.index >= 2 && (2..=3).contains(&facts.count);
    let grouped_visible = compiled.visible_grouped && facts.index < 2 && (2..=3).contains(&facts.starting_forms);
    let exact = facts.count == 1
        && facts.installed == 1
        && facts.exact_mii == ExactMiiProof::Proven
        && MiiClass::from_kind(facts.kind).is_some();
    OwnerAdmission { admits: ordinary || grouped_helper || grouped_visible || exact, grouped_visible }
}

/// One visible owner's input route row: (pointer, state, key, owner, alias).
pub type RouteRow = (usize, u8, i32, i32, i32);

/// lobby-preload B: a visible bank window as the extra-creation capture read it
/// (`native_source_actors`, under the current native callback). Addresses are equality values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BankWindow {
    /// Layout owner (0 Player; 1 Training CPU; 4 waiting CPU).
    pub owner: u32,
    pub entry: usize,
    /// The owner's resource object `R` and its kind word `R+0x58`.
    pub resource: (usize, u32),
    /// The window's member kinds (`u32::MAX` past the count) and member bodies.
    pub kinds: [u32; 3],
    pub bodies: [usize; 3],
    /// Each member's (actor, control, kind), read from the window's own actor cells.
    pub members: [Option<(usize, usize, u32)>; 3],
    /// The active cell (the member the window's selected context names).
    pub active: usize,
    /// The owner's own input record (`records + owner * 0x280`).
    pub record: usize,
    /// The active control's (input_index, binding, alias_index).
    pub route: (i32, usize, i32),
    /// The body bank's committed member kinds, re-read now under the bank lock by
    /// `body_bank::ready_members`: Ready, the committed descriptor and bodies, the active cell.
    /// None: no committed body bank owns this window (a native alternate form, or not Ready).
    pub committed: Option<[u32; 3]>,
}

/// lobby-preload B: the active input row a committed body-bank window must show. The analogue
/// of the native alternate-form rule (`source_visible_forms::binding`): instead of the Aegis/
/// Trainer family, the bank's committed window proves the members. The pointer and key are the
/// committed active member's control and kind, never copied from input; occupancy and
/// owner/alias are preserved exactly, as that rule preserves them.
pub fn body_bank_row(window: BankWindow, observed: RouteRow) -> Result<RouteRow, &'static str> {
    let committed = window.committed.ok_or("bank: no committed body bank owns the window")?;
    if committed != window.kinds { return Err("bank: window kinds differ from the committed bank"); }
    let count = window.kinds.iter().take_while(|kind| **kind != u32::MAX).count();
    if !matches!(window.owner, 0 | 1 | 4) || window.entry == 0 || window.resource.0 == 0
        || window.record == 0 || count == 0 || window.active >= count
    {
        return Err("bank: owner, entry, resource, record or active cell");
    }
    if window.resource.1 != window.kinds[0] { return Err("bank: resource kind word"); }
    if window.route != (window.owner as i32, window.record, window.owner as i32) {
        return Err("bank: active control is not bound to the owner's record");
    }
    for index in 0..3 {
        if index >= count {
            if window.members[index].is_some() || window.bodies[index] != 0 {
                return Err("bank: member past the committed count");
            }
            continue;
        }
        let (actor, control, kind) = window.members[index].ok_or("bank: committed member absent")?;
        if actor == 0 || control == 0 || window.bodies[index] == 0 || kind != window.kinds[index]
            || window.kinds[..index].contains(&kind)
            || window.bodies[..index].contains(&window.bodies[index])
            || window.members[..index].iter().flatten().any(|(a, c, _)| *a == actor || *c == control)
        {
            return Err("bank: member actor, control, kind or body");
        }
    }
    let (_, control, kind) = window.members[window.active].ok_or("bank: committed member absent")?;
    Ok((control, observed.1, kind as i32, observed.3, observed.4))
}

/// lobby-preload B: the creation capture's decision for a visible bank window's active input row.
/// `alternate` is the native alternate-form rule's expected row (`visible::expected_slot`; None
/// outside Aegis/Trainer). M5g stopped there, so a committed body bank always failed with
/// `bank: active input row` (g1 #1322). Ok(true): admitted by the committed body-bank rule.
pub fn creation_row(observed: RouteRow, alternate: Option<RouteRow>, window: Option<BankWindow>)
    -> Result<bool, &'static str> {
    if alternate == Some(observed) { return Ok(false); }
    let Some(window) = window else { return Err("bank: active input row"); };
    if body_bank_row(window, observed)? == observed { Ok(true) }
    else { Err("bank: active input row is not the committed bank member's") }
}

