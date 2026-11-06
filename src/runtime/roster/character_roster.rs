//! Reload a practice player or CPU through the game's asynchronous entry/resource loader.
//! Requests contain values and entry identities, never retained game pointers.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use hos_lock::Mutex as OwnerMutex;

use skyline::hooks::{getRegionAddress, InlineCtx, Region};
use {crate::engine::compat::menu::OnOff, crate::engine::compat::menu::TrainingModpackMenu, crate::engine::swap::trio::TrioFighter, crate::runtime::compat::menu::MENU};
use {crate::runtime::compat::sync::lock_write, crate::runtime::compat::sync::read as setting};

use crate::engine::swap::character_memory::WaitingSession;
use crate::runtime::compat::common::{
    button_config, game_mode,
    input::{Buttons, MappedInputs},
    menu,
};
use crate::runtime::lobby::training::{self, lobby};
use crate::engine::swap::character_forms::{
    configured_forms, cycle_position, next_cycle, next_from_original, EntryState, Forms,
    PresentedReceipt, Rebuild,
};

#[path = "native_service.rs"]
mod native_service;
use native_service::ServiceEntryContext;

#[cfg(feature = "native-body-bank-alpha")]
#[path = "native_body_bank.rs"]
pub(crate) mod body_bank;

#[path = "native_install.rs"]
mod native_install;
use native_install::{InstallGate, InstallToken};

#[path = "native_fighter_install.rs"]
mod native_fighter_install;

#[path = "native_reload_pose.rs"]
mod native_reload_pose;

use crate::engine::swap::native_hud_refresh as native_hud_refresh;

#[path = "native_source.rs"]
mod native_source;
#[cfg(feature = "managed-third-source-alpha")]
#[path = "native_source_capacity.rs"]
mod native_source_capacity;
#[cfg(feature = "managed-third-source-alpha")]
pub(super) use native_source_capacity::{prepare_load_contract, VerifiedLoad};
#[path = "character_sources.rs"]
pub mod sources;
/// GS-PANIC: the process-memory admission every fighter load the mod starts goes through.
#[path = "process_memory.rs"]
pub(crate) mod process_memory;

macro_rules! trace {
    ($($arg:tt)*) => { crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}

#[cfg(feature="mii-saved-alpha")]
#[path="../mii/mii_saved_alpha.rs"]
pub(crate) mod mii_saved_alpha;
#[cfg(feature="mii-saved-alpha")]
#[path="../mii/mii_catalog_request.rs"]
pub(crate) mod mii_catalog_request;
#[cfg(feature="mii-saved-alpha")]
#[path="../mii/mii_backend.rs"]
mod mii_backend;
#[cfg(feature="mii-saved-alpha")]
#[path="../mii/mii_saved_fingerprints.rs"]
mod mii_saved_fingerprints;

#[cfg(feature="mii-ordinary-player-alpha")]
#[path="../mii/mii_ordinary_player.rs"]
mod mii_ordinary_player;
#[cfg(feature="mii-ordinary-cpu-alpha")]
#[path="../mii/mii_ordinary_cpu.rs"]
mod mii_ordinary_cpu;
#[cfg(feature="mii-ordinary-cpu-alpha")]
#[path="../mii/mii_selector_native.rs"]
mod mii_selector_native;
#[cfg(feature="mii-ldn-qualification-probe")]
#[path="../mii/mii_ldn_qualification.rs"]
mod mii_ldn_qualification;
#[cfg(feature="mii-managed-helper-alpha")]
#[path="../mii/mii_managed_helper.rs"]
mod mii_managed_helper;
#[cfg(feature="mii-ordinary-cpu-alpha")]
#[path="../mii/mii_auto_init.rs"]
mod mii_auto_init;
#[cfg(feature="mii-ordinary-cpu-alpha")]
#[path="../mii/mii_replay.rs"]
pub(crate) mod mii_replay;
#[cfg(feature="mii-hud-portrait-alpha")]
pub(crate) use crate::engine::mii_root::mii_hud_portrait_policy as mii_hud_portrait_policy;
#[cfg(feature="mii-hud-portrait-alpha")]
#[path="../mii/mii_hud_portrait.rs"]
pub(crate) mod mii_hud_portrait; // WP-P: `swap_native::presentation` reads the card slot through `slot_of_owner`

/// Settled loaded Mii for a practice role. Role 0 is Player and role 1 is CPU.
/// The value is that role's original, saved, or discovered configuration only
/// while its class is the loaded descriptor and no transaction is in flight.
/// Replay receipts, helpers, and the other role are not returned.
pub fn current_mii_configuration(role: usize) -> Option<crate::engine::mii::lifetime::Configuration> {
    match role {
        0 => settled_player_mii(),
        1 => settled_cpu_mii(),
        _ => None,
    }
}
#[cfg(feature = "mii-ordinary-player-alpha")]
fn settled_player_mii() -> Option<crate::engine::mii::lifetime::Configuration> {
    mii_ordinary_player::current_configuration()
}
#[cfg(not(feature = "mii-ordinary-player-alpha"))]
fn settled_player_mii() -> Option<crate::engine::mii::lifetime::Configuration> { None }
#[cfg(feature = "mii-ordinary-cpu-alpha")]
fn settled_cpu_mii() -> Option<crate::engine::mii::lifetime::Configuration> {
    mii_ordinary_cpu::current_configuration()
}
#[cfg(not(feature = "mii-ordinary-cpu-alpha"))]
fn settled_cpu_mii() -> Option<crate::engine::mii::lifetime::Configuration> { None }

static BASE: AtomicUsize = AtomicUsize::new(0);
/// The main image base this module installed with (0 before install); for the WP-P
/// readback facade's lifecycle hooks outside this module (its only callers, in
/// `training/save_states.rs`, are compiled with the same feature).
#[cfg(feature = "mii-hud-portrait-alpha")]
pub(crate) fn main_base() -> usize { BASE.load(Ordering::Acquire) }
static INSTALL: InstallGate = InstallGate::new();
static OWNED_BUTTONS: AtomicU32 = AtomicU32::new(0);
/// The CPU operator's consumed chord: mapping index (high word) and buttons (low word).
static OPERATOR_BUTTONS: AtomicU64 = AtomicU64::new(0);
// The first ready body of this native entry supplies its character-select kind.
// Reloads retain the entry, so a practice replacement must not overwrite this.
static ORIGINAL_KINDS: [AtomicU32; 2] = [AtomicU32::new(u32::MAX), AtomicU32::new(u32::MAX)];
static SOLO_ACTIVE: [AtomicBool; 2] = [AtomicBool::new(false), AtomicBool::new(false)];
/// Set by `allow_portrait` when native reset rebuilt a role's Mii publication. Value-only:
/// that role's next service callback revalidates the receipt and body, then republishes.
static REBUILT: [AtomicBool; 2] = [AtomicBool::new(false), AtomicBool::new(false)];
static STATES: [OwnerMutex<State>; 2] = [OwnerMutex::new(State::empty()), OwnerMutex::new(State::empty())];
static WAITING_MEMORY: OwnerMutex<WaitingSession<WaitingValue>> = OwnerMutex::new(WaitingSession::empty());
type WaitingValue=crate::engine::mii::remembered::Value<TrioFighter,Forms>;

/// Loaded configuration, or an explicit original restoration. Absence is `NotMii`.
#[cfg(feature="mii-hud-portrait-alpha")]
#[derive(Clone, Copy, Debug)]
pub(crate) enum PortraitIntent {
    Exact(crate::engine::mii::lifetime::Configuration),
    Original { original_kind: u32 },
    NotMii,
}
#[cfg(feature="mii-saved-alpha")]
fn remember_owned(role:Role,generation:u64,kind:u32,forms:Forms,
    configuration:Option<crate::engine::mii::lifetime::Configuration>,cursor:Option<usize>,original:bool) {
    let mut memory=WAITING_MEMORY.lock();
    if !mii_backend::current().is_some_and(|l|l.waiting())
        || mii_saved_alpha::current_generation(role as u32)!=generation {return;}
    if let Some(fighter)=TrioFighter::from_kind(kind) {
        memory.commit(role as usize,
            WaitingValue {fighter,forms,cursor,configuration,owned:true,original});
        drop(memory);
        crate::runtime::roster::kirby_hats::fighter_committed(role as usize, kind);
    }
}
fn restore_owned(role:Role,value:WaitingValue)->bool {
    // Evaluate the role adapter even when the remembered value is already owned.
    // Missing qualification never falls through to a kind-only Mii load.
    match role {
        Role::Player => {
            #[cfg(feature="mii-ordinary-player-alpha")]
            { value.owned | mii_ordinary_player::request_restore(value) }
            #[cfg(not(feature="mii-ordinary-player-alpha"))]
            { value.owned }
        }
        Role::Cpu => {
            #[cfg(feature="mii-ordinary-cpu-alpha")]
            { value.owned | mii_ordinary_cpu::request_restore(value) }
            #[cfg(not(feature="mii-ordinary-cpu-alpha"))]
            { value.owned }
        }
    }
}
#[cfg(feature="mii-saved-alpha")]
fn observe_mii_origin(role:Role,generation:u64,kind:u32,forms:Forms,
    configuration:Option<crate::engine::mii::lifetime::Configuration>) {
    let (Some(configuration),Some(fighter))=(configuration,TrioFighter::from_kind(kind)) else{return;};
    let mut memory=WAITING_MEMORY.lock();
    if !mii_backend::current().is_some_and(|l|l.waiting())
        || mii_saved_alpha::current_generation(role as u32)!=generation {return;}
    // observe preserves a same-lobby remembered choice; a fresh room clears both roles.
    let fresh = memory.observe(role as usize,WaitingValue {fighter,forms,cursor:None,
        configuration:Some(configuration),owned:true,original:true});
    drop(memory);
    if fresh { crate::runtime::roster::kirby_hats::fresh_lobby(); }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    Player,
    Cpu,
}

impl Role {
    fn selected(self, m: &TrainingModpackMenu) -> TrioFighter {
        match self {
            Self::Player => m.player_character,
            Self::Cpu => m.cpu_character,
        }
    }
    fn enabled(self) -> bool {
        self == Self::Cpu || setting(&MENU).trio_practice == OnOff::ON
    }
    fn sync_selection(self, fighter: TrioFighter) {
        let mut m = lock_write(&MENU);
        match self {
            Self::Player => m.player_character = fighter,
            Self::Cpu => m.cpu_character = fighter,
        }
    }
}

fn state_for(role: Role) -> &'static OwnerMutex<State> {
    &STATES[role as usize]
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Scope {
    mode: u32,
    owner: u32,
}

impl Scope {
    fn waiting(self) -> bool {
        self.mode != 0x04040000
    }
}

#[derive(Clone, Copy, Debug)]
enum Selection {
    Direct(TrioFighter),
    Cycle,
    RefreshPartners,
}

#[derive(Clone, Copy, Debug)]
struct Request {
    scope: Scope,
    epoch: usize,
    entry: usize,
    selection: Selection,
    age: usize,
    restoring: bool,
    /// One explicit Direct load the user chose outside the cycle (a combo's Switch,
    /// `request_player_kind`): admitted even while the role's cycle is off.
    once: bool,
}

#[derive(Clone, Copy, Debug)]
struct Active {
    install: InstallToken,
    scope: Scope,
    epoch: usize,
    revision: usize,
    entry: usize,
    target: TrioFighter,
    forms: Forms,
    slot: Option<usize>,
    ticks: usize,
    phase: u32,
    departed: bool,
}

struct State {
    epoch: usize,
    revision: usize,
    pending: Option<Request>,
    active: Option<Active>,
    cursor: Option<usize>,
    loaded: Option<(usize, u32, u32)>,
    observed: Option<(usize, u32)>,
    /// Native-reset portrait admission for an accepted Mii-adapter publication.
    /// Read only by `allow_portrait`; never a CPU idleness input.
    presented: PresentedReceipt,
}

impl State {
    const fn empty() -> Self {
        Self {
            epoch: 0,
            revision: 0,
            pending: None,
            active: None,
            cursor: None,
            loaded: None,
            observed: None,
            presented: PresentedReceipt::NONE,
        }
    }
}

fn scope(role: Role) -> Option<Scope> {
    let mode = game_mode::current()?;
    if !mode.initialized {
        return None;
    }
    if mode.melee == 0x20 && mode.mode == 0x04040000 {
        Some(Scope {
            mode: mode.mode,
            owner: role as u32,
        })
    } else if cfg!(feature = "waiting-roster-reload") && lobby::active() {
        Some(Scope {
            mode: mode.mode,
            owner: if role == Role::Player { 0 } else { 4 },
        })
    } else {
        None
    }
}

pub fn busy() -> bool {
    INSTALL.suspends_practice()
}

/// Speculation cannot take the installer ahead of an accepted visible operation.
pub(crate) fn preload_blocked() -> bool {
    // Capture the immutable visible origins before speculative loads can take
    // INSTALL. The same native backend qualification serves Training and waiting.
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    if mii_auto_init::initializing() && unsafe {mii_backend::ready()} {return true;}
    if INSTALL.active() || crate::runtime::roster::kirby_hats::preload_pending()
        || STATES.iter().any(|s| {let s=s.lock();
            s.pending.is_some() || s.active.is_some()}) {return true;}
    #[cfg(feature="mii-ordinary-player-alpha")]
    if mii_ordinary_player::preload_pending() {return true;}
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    if mii_ordinary_cpu::preload_pending() {return true;}
    false
}

/// What may pause the Fighters notice: the accepted visible operations of `preload_blocked` only.
/// Visible roster and Mii installs take INSTALL with practice suspended; the preloader's own
/// speculative helper loads, hat-source loads and group prefetches take it without (and a hat
/// request is asset work), so they never read as a pause (C116.2 console "Paused 2/3").
pub(crate) fn notice_blocked() -> bool {
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    if mii_auto_init::initializing() && unsafe {mii_backend::ready()} {return true;}
    if INSTALL.suspends_practice()
        || STATES.iter().any(|s| {let s=s.lock();
            s.pending.is_some() || s.active.is_some()}) {return true;}
    #[cfg(feature="mii-ordinary-player-alpha")]
    if mii_ordinary_player::preload_pending() {return true;}
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    if mii_ordinary_cpu::preload_pending() {return true;}
    false
}

pub(crate) fn preload_targets(menu: &TrainingModpackMenu, role:usize)
    -> [Option<crate::runtime::roster::cycle_preload::Target>;crate::runtime::roster::cycle_preload::MAX_TARGETS] {
    let role=if role==0 {Role::Player}else{Role::Cpu};
    if !role.enabled() {return [None;crate::runtime::roster::cycle_preload::MAX_TARGETS];}
    #[cfg(feature="mii-ordinary-player-alpha")]
    if role==Role::Player {
        if let Some(targets)=mii_ordinary_player::preload_targets(menu) {return targets;}
    }
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    if role==Role::Cpu {
        if let Some(targets)=mii_ordinary_cpu::preload_targets(menu) {return targets;}
    }
    let (mut current,mut cursor)={let s=state_for(role).lock();
        let Some((_,kind))=s.observed else {return [None;crate::runtime::roster::cycle_preload::MAX_TARGETS];}; (kind,s.cursor)};
    let mut kinds=crate::engine::swap::ordered_roster::KindList::new(0);
    for f in roster_fixed(menu,role).iter() {kinds.push(f.kind());}
    let original=cycle_original(role);
    let mut targets=[None;crate::runtime::roster::cycle_preload::MAX_TARGETS];
    for target in &mut targets {
        let at_original=cursor.is_none() && original.is_some_and(|k|cycle_position(&[k],current).is_some());
        let Some((kind,next))=(if at_original {next_from_original(&kinds,original)}
            else {next_cycle(current,cursor,&kinds,original)}) else {break;};
        *target=Some(crate::runtime::roster::cycle_preload::Target::fighter(kind));
        current=kind;cursor=next;
    }
    targets
}

/// Hat continuation waits for the remembered practice fighter, including owned
/// Mii transitions, instead of acting on a briefly ready CSS body after return.
pub fn waiting_choice_settled(role: usize) -> bool {
    if busy() { return false; }
    if STATES[role].lock().observed.is_none_or(|(_, kind)| kind != 6) {
        return false;
    }
    // The other role's automatic replacement would cancel a newly queued hat.
    // Wait for both, without nesting roster/memory locks into hat acquisition.
    if STATES.iter().any(|state| {
        let state = state.lock();
        state.pending.is_some() || state.active.is_some()
    }) { return false; }
    let memory = WAITING_MEMORY.lock();
    !memory.restoration_pending([Role::Player.enabled(), Role::Cpu.enabled()])
}

/// Drafts resume after both returning entries have been observed and remembered
/// choices reconciled. Resuming on the first ready CSS body would let the later
/// automatic roster install close an unfinished panel as ordinary swap cleanup.
pub fn waiting_drafts_ready() -> bool {
    if busy() { return false; }
    if !cfg!(feature = "waiting-roster-reload") { return true; }
    if STATES.iter().any(|state| {
        let state = state.lock();
        state.observed.is_none() || state.pending.is_some() || state.active.is_some()
    }) { return false; }
    !WAITING_MEMORY.lock()
        .restoration_pending([Role::Player.enabled(), Role::Cpu.enabled()])
}

fn finish_install(token: InstallToken, reason: &str) -> bool {
    let finished = INSTALL.finish(token);
    #[cfg(feature = "managed-third-source-alpha")]
    if reason == "managed source destroyed" {
        trace!(
            "[NativeInstall] managed destructor token={:?} finished={} gate_active_after={}",
            token,
            finished,
            INSTALL.active()
        );
    }
    if !finished {
        trace!(
            "[NativeInstall] ignored stale completion token={:?} reason={}",
            token,
            reason
        );
    }
    finished
}

fn original_kind(role: Role) -> Option<u32> {
    let kind = ORIGINAL_KINDS[role as usize].load(Ordering::Acquire);
    (kind != u32::MAX).then_some(kind)
}

/// Published only from a ready native descriptor, not from an unapplied preference.
pub fn solo_popo_active(cpu: bool) -> bool {
    let role = if cpu { Role::Cpu } else { Role::Player };
    scope(role).is_some() && SOLO_ACTIVE[role as usize].load(Ordering::Acquire)
}

/// Forms for a fighter that is not at a known numbered slot: slot 0 or a direct pick.
#[cfg(feature = "mii-ordinary-cpu-alpha")]
fn requested_forms(m: &TrainingModpackMenu, role: Role, kind: u32) -> Forms {
    requested_forms_at(m, role, kind, None)
}

/// `slot` is the numbered cycle position holding the fighter. The same Ice Climbers kind
/// takes the original preference at Player slot 0 and the cycle's at a numbered slot.
fn requested_forms_at(m: &TrainingModpackMenu, role: Role, kind: u32, slot: Option<usize>) -> Forms {
    configured_forms(m, role == Role::Cpu, original_kind(role), kind, slot.is_some())
}

fn publish_partners(role: Role, forms: Forms) {
    SOLO_ACTIVE[role as usize].store(forms == Forms::solo_popo(), Ordering::Release);
}

/// Ordinary slot 0 for this role. A CPU Mii class kind is not a saved identity;
/// that return belongs to the saved-Mii cycle, not a default Swordfighter load.
fn cycle_original(role: Role) -> Option<u32> {
    let kind = original_kind(role)?;
    if role == Role::Cpu && (0x48..=0x4a).contains(&kind) {
        return None;
    }
    Some(kind)
}

fn notify_args(role: Role, args: std::fmt::Arguments<'_>) {
    let title = if role == Role::Player {
        "Player Character"
    } else {
        "CPU Character"
    };
    training::ui::notifications::notification_args(title.into(), args, 180);
}

fn notify_selection(role: Role, fighter: TrioFighter, slot: Option<usize>) {
    match slot {
        Some(index) => notify_args(role, format_args!("{}. {}", index + 1, fighter.name())),
        None if cycle_original(role) == Some(fighter.kind()) => notify_args(role, format_args!("0. {}", fighter.name())),
        None => notify(role, fighter.name().into()),
    }
}

fn roster(m: &TrainingModpackMenu, role: Role) -> Vec<TrioFighter> {
    roster_fixed(m, role).to_vec()
}

/// `roster` without heap (the preload planner's per-tick form).
fn roster_fixed(m: &TrainingModpackMenu, role: Role) -> crate::engine::swap::ordered_roster::FighterList {
    let excluded = match role {
        Role::Player => crate::engine::swap::roster_grid::cycle_exclusion(
            original_kind(role),
            m.solo_popo_original == OnOff::ON,
            m.solo_popo_player_cycle == OnOff::ON,
        ),
        Role::Cpu => {
            let solo = m.solo_popo_cpu == OnOff::ON;
            crate::engine::swap::roster_grid::cycle_exclusion(cycle_original(role), solo, solo)
        }
    };
    // P-CYCLE: the one cycle list (`cycle_members`), with no Mii class: this loader cannot load a Mii, and while it
    // serves the cycle no Mii owner does, so the menu's published admission is none too.
    let roster = match role {
        Role::Player => m.player_roster,
        Role::Cpu => m.cpu_roster,
    };
    crate::engine::swap::cycle_members::cycle_members_excluding(roster, crate::engine::swap::cycle_members::NO_MII, excluded)
}

/// The CPU cycle's Mii admission as the CPU Mii owner last computed it (`cycle_members::cpu_admission`, published from
/// its service; none while it is not qualified). Every CPU cycle walker and the menu read this one answer (P-CYCLE).
pub(crate) fn cpu_cycle_admission() -> crate::engine::swap::cycle_members::MiiAdmission {
    #[cfg(feature = "mii-ordinary-cpu-alpha")]
    return mii_ordinary_cpu::published_admission();
    #[cfg(not(feature = "mii-ordinary-cpu-alpha"))]
    crate::engine::swap::cycle_members::NO_MII
}

/// The Player cycle's Mii admission as the Player Mii owner last computed it (`cycle_members::player_admission`,
/// published from its service; none while it is not qualified). Every Player cycle walker and the menu read this one
/// answer (P1-CYCLE).
pub(crate) fn player_cycle_admission() -> crate::engine::swap::cycle_members::MiiAdmission {
    #[cfg(feature = "mii-ordinary-player-alpha")]
    return mii_ordinary_player::published_admission();
    #[cfg(not(feature = "mii-ordinary-player-alpha"))]
    crate::engine::swap::cycle_members::NO_MII
}

/// What the portrait grid shows of the cycles (P-CYCLE, P1-CYCLE): each role's Mii admission, and each role's slot 0
/// (`[Player, CPU]`: the scene's original kind, the fighter the cycle returns to) while a practice scene has one.
/// Atomics only: the menu worker reads them, never an owner.
pub fn cycle_facts() -> (crate::engine::swap::cycle_members::MiiAdmission, crate::engine::swap::cycle_members::MiiAdmission, [Option<u32>; 2]) {
    (player_cycle_admission(), cpu_cycle_admission(), [original_kind(Role::Player), original_kind(Role::Cpu)])
}

/// Numbered slot of `kind`'s group in `role`'s current cycle.
fn locate(m: &TrainingModpackMenu, role: Role, kind: u32) -> Option<usize> {
    let kinds: Vec<_> = roster(m, role).iter().map(|fighter| fighter.kind()).collect();
    cycle_position(&kinds, kind)
}

fn notify(role: Role, message: std::borrow::Cow<'static, str>) {
    let title = if role == Role::Player {
        "Player Character"
    } else {
        "CPU Character"
    };
    training::ui::notifications::notification(title.into(), message, 180);
}

/// Called after committing menu values. Editing an unrelated option must not
/// replace a fighter, and navigating/highlighting a fighter is not a request.
pub fn settings_changed(previous: &TrainingModpackMenu,
    manual: Option<(crate::engine::swap::roster_grid::Role, u32)>) {
    let current = setting(&MENU);
    if previous.player_roster != current.player_roster
        || previous.cpu_roster != current.cpu_roster
        || previous.player_kirby_hats != current.player_kirby_hats
        || previous.cpu_kirby_hats != current.cpu_kirby_hats
        || previous.solo_popo_original != current.solo_popo_original
        || previous.solo_popo_player_cycle != current.solo_popo_player_cycle
        || previous.solo_popo_cpu != current.solo_popo_cpu
        || previous.trio_practice != current.trio_practice
    {
        crate::runtime::roster::cycle_preload::reset_admission_grace();
    }
    for role in [Role::Player, Role::Cpu] {
        if roster(previous, role) != roster(&current, role) {
            let mut state = state_for(role).lock();
            state.revision = state.revision.wrapping_add(1);
            // A numbered fighter keeps its slot intent through an edit (Ice Climbers can be
            // both slot 0 and numbered). A removed entry keeps a stale hint: the
            // next cycle validates it by kind, while Solo preference still follows
            // the numbered origin until an explicit return to slot 0.
            let live = state.observed.map(|(_, kind)| kind);
            state.cursor = state.cursor.map(|old|
                live.and_then(|kind| locate(&current, role, kind)).unwrap_or(old));
            trace!(
                "[CharacterRoster] settings role={:?} size={} kinds={:x?}",
                role,
                roster(&current, role).len(),
                roster(&current, role)
                    .iter()
                    .map(|f| f.kind())
                    .collect::<Vec<_>>()
            );
        }
        let player_just_enabled = role == Role::Player
            && previous.trio_practice != OnOff::ON
            && current.trio_practice == OnOff::ON;
        // A portrait press is an action even when its one-hot setting is unchanged
        // (e.g. retrying a previously refused request). Dispatch it exactly once.
        let explicit = manual.is_some_and(|(picked_role, kind)|
            (picked_role == crate::engine::swap::roster_grid::Role::Player) == (role == Role::Player)
                && kind == role.selected(&current).kind());
        if explicit || role.selected(previous) != role.selected(&current) || player_just_enabled {
            queue(role, Selection::Direct(role.selected(&current)));
        }
    }
}

fn queue(role: Role, selection: Selection) {
    queue_request(role, selection, false);
}

fn queue_request(role: Role, selection: Selection, restoring: bool) {
    queue_admitted(role, selection, restoring, false);
}

/// `once`: one explicit Direct load outside the cycle; the role's cycle setting
/// (Player Swapping) does not refuse it and is not changed by it.
fn queue_admitted(role: Role, selection: Selection, restoring: bool, once: bool) {
    // FAIL 2: a user's request leaves one line on every path, including the ones that drop it.
    let user = !restoring && !matches!(selection, Selection::RefreshPartners);
    if !role.enabled() && !once && !matches!(selection, Selection::RefreshPartners) {
        if user {
            trace!("[CharacterRoster] request ignored role={:?} selection={:?}: this role's roster cycle is off", role, selection);
        }
        return;
    }
    if BASE.load(Ordering::Acquire) == 0 || !crate::runtime::roster::character_lifecycle::native_hooks_ready() {
        trace!("[CharacterRoster] request ignored role={:?} selection={:?}: native hooks unavailable", role, selection);
        notify(role, "Unavailable on this game version".into());
        return;
    }
    let Some(scope) = scope(role) else {
        if user {
            trace!("[CharacterRoster] request ignored role={:?} selection={:?}: not a practice scene", role, selection);
        }
        return;
    };
    let occupied = if busy() {
        Some("Wait for the character swap to finish")
    } else if training::recording_or_playback() {
        Some("Finish recording or playback before swapping")
    } else if training::save_states::is_loading() {
        Some("Wait for the save-state restore to finish")
    } else if lobby::native_ui_busy() {
        Some("Close the current game menu before swapping")
    } else if button_config::has_pending_request() {
        Some("Finish the current shortcut before swapping")
    } else {
        None
    };
    if let Some(reason) = occupied {
        if !restoring && !matches!(selection, Selection::RefreshPartners) {
            trace!("[CharacterRoster] request rejected: {}", reason);
            notify(role, reason.into());
        }
        return;
    }
    sources::pause_preload();
    if crate::runtime::roster::cycle_preload_config::snapshot().measurements
        && !restoring && !matches!(selection,Selection::RefreshPartners) {
        let target=match selection {Selection::Direct(f)=>Some(crate::runtime::roster::cycle_preload::Target::fighter(f.kind())),Selection::Cycle=>
            preload_targets(&setting(&MENU),role as usize)[0],_=>None};
        if let Some(target)=target {crate::runtime::roster::cycle_preload::request(role as usize,false,target);}
    }
    #[cfg(any(feature="mii-ordinary-player-alpha",feature="mii-ordinary-cpu-alpha"))]
    let user_request=!restoring && !matches!(selection,Selection::RefreshPartners);
    #[cfg(feature="mii-ordinary-player-alpha")]
    if mii_ordinary_player::intercept(role,selection,user_request)==Some(true){
        if user {trace!("[CharacterRoster] request handed to the ordinary Mii owner role={:?} selection={:?}",role,selection);}
        return;
    }
    // The CPU owner logs its own acceptance or refusal.
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    if mii_ordinary_cpu::intercept(role,selection,user_request)==Some(true){return;}
    #[cfg(feature="mii-saved-alpha")]
    if mii_saved_alpha::intercept(role,selection){
        if user {trace!("[CharacterRoster] request handed to the saved Mii owner role={:?} selection={:?}",role,selection);}
        return;
    }
    let mut state = state_for(role).lock();
    // The callback's restore peek may race a manual request. The role lock also
    // protects manual enqueue/cancellation: never overwrite newer explicit intent.
    if restoring
        && (state.pending.is_some()
            || state.active.is_some()
            || !matches!(selection, Selection::Direct(target)
                if WAITING_MEMORY.lock().restore_target(role as usize).is_some_and(|v|!v.owned && v.fighter==target)))
    {
        return;
    }
    let Some((entry, _)) = state.observed else {
        drop(state);
        trace!("[CharacterRoster] request rejected: no current practice entry");
        if !matches!(selection, Selection::RefreshPartners) {
            notify(
                role,
                "Wait for the practice character to finish loading".into(),
            );
        }
        return;
    };
    state.pending = Some(Request {
        scope,
        epoch: state.epoch,
        entry,
        selection,
        age: 0,
        restoring,
        once,
    });
    if scope.waiting() && !restoring && !matches!(selection, Selection::RefreshPartners) {
        WAITING_MEMORY.lock().manual_request(role as usize);
    }
    trace!(
        "[CharacterRoster] queued owner={} entry=0x{:x} epoch={} selection={:?}",
        scope.owner,
        entry,
        state.epoch,
        selection
    );
}

/// True when a CPU kind request can be queued without the rejection notice.
pub fn cpu_reload_ready() -> bool {
    if busy()
        || training::recording_or_playback()
        || training::save_states::is_loading()
        || lobby::native_ui_busy()
        || button_config::has_pending_request()
        || menu::captures_input()
    {
        return false;
    }
    let state = state_for(Role::Cpu).lock();
    state.pending.is_none() && state.observed.is_some() && scope(Role::Cpu).is_some()
}

/// Ask the existing loader to change the CPU to `kind`. Values only.
/// A kind outside the roster notifies once and does not queue.
pub fn request_cpu_kind(kind: u32) -> bool {
    let Some(fighter) = TrioFighter::from_kind(kind) else {
        notify(Role::Cpu, "Named macro fighter is not available".into());
        return false;
    };
    if state_for(Role::Cpu).lock().pending.is_some() {
        return false;
    }
    queue(Role::Cpu, Selection::Direct(fighter));
    state_for(Role::Cpu).lock().pending.is_some_and(
        |request| matches!(request.selection, Selection::Direct(target) if target == fighter),
    )
}

/// Player 1's fighter kind as the loader last observed it (None before a practice entry).
pub fn player_kind() -> Option<u32> {
    state_for(Role::Player).lock().observed.map(|(_, kind)| kind)
}

/// Ask the existing loader to change Player 1 to `kind` (a combo's recorded
/// fighter, SPEC-input-07 §4a): the Direct request a Player Character pick makes,
/// as one explicit load the user chose, so Player Swapping Off does not refuse it
/// (that setting keeps the plugin from cycling P1 on its own; it is not changed).
/// False when nothing was queued; the loader's notice says why.
pub fn request_player_kind(kind: u32) -> bool {
    let Some(fighter) = TrioFighter::from_kind(kind) else {
        return false;
    };
    if state_for(Role::Player).lock().pending.is_some() {
        return false;
    }
    queue_admitted(Role::Player, Selection::Direct(fighter), false, true);
    state_for(Role::Player).lock().pending.is_some_and(
        |request| matches!(request.selection, Selection::Direct(target) if target == fighter),
    )
}

/// Runs before the delay queue. Consume only our request, giving conflicting
/// save/record/menu requests precedence. Preserve suppression through a held
/// chord so it cannot turn into a delayed taunt or a player trio request.
pub fn handle_input(out: &mut MappedInputs) {
    let player = button_config::consume_player_cycle_request();
    let cpu = button_config::consume_cpu_cycle_request();
    if player || cpu {
        if let Some(reason) = chord_dropped() {
            trace!("[CharacterRoster] cycle chord dropped player={} cpu={}: {}", player, cpu, reason);
        } else {
            OWNED_BUTTONS.store(out.buttons.bits(), Ordering::Relaxed);
            if player && cpu {
                notify(
                    Role::Player,
                    "Use different mappings for player and CPU".into(),
                );
            } else {
                queue(
                    if player { Role::Player } else { Role::Cpu },
                    Selection::Cycle,
                );
            }
        }
    }
    suppress_owned_buttons(out);
}

/// Why a consumed cycle chord does not reach the loader: another shortcut or the menu
/// owns this input. FAIL 2: the drop is logged, never silent.
fn chord_dropped() -> Option<&'static str> {
    if button_config::has_pending_request() {
        Some("another shortcut pending")
    } else if menu::captures_input() {
        Some("menu captures input")
    } else {
        None
    }
}

/// Keep a consumed chord suppressed through the native body-loading interval,
/// when the rest of the practice hooks intentionally return early.
pub fn suppress_owned_buttons(out: &mut MappedInputs) {
    let owned = OWNED_BUTTONS.load(Ordering::Relaxed) & out.buttons.bits();
    OWNED_BUTTONS.store(owned, Ordering::Relaxed);
    out.buttons.remove(Buttons::from_bits_truncate(owned));
}

/// The controller operating the CPU under native Behavior=Control
/// (`practice::cpu_operator`). Its CPU cycle chord queues the CPU role only, through
/// P1's gates and loader; P1's chord and its suppression stay separate.
pub fn handle_cpu_operator_input(player_idx: i32, out: &mut MappedInputs, cycle: bool) {
    if cycle {
        if let Some(reason) = chord_dropped() {
            trace!("[CharacterRoster] operator cycle chord dropped controller={}: {}", player_idx, reason);
        } else {
            OPERATOR_BUTTONS.store(operator_buttons(player_idx, out.buttons.bits()), Ordering::Relaxed);
            queue(Role::Cpu, Selection::Cycle);
        }
    }
    suppress_operator_buttons(player_idx, out);
}

/// The operator's consumed chord stays suppressed on its own mapping only, including
/// through the native body-loading interval.
pub fn suppress_operator_buttons(player_idx: i32, out: &mut MappedInputs) {
    let stored = OPERATOR_BUTTONS.load(Ordering::Relaxed);
    if stored >> 32 != u64::from(player_idx as u32) {
        return;
    }
    let owned = stored as u32 & out.buttons.bits();
    OPERATOR_BUTTONS.store(operator_buttons(player_idx, owned), Ordering::Relaxed);
    out.buttons.remove(Buttons::from_bits_truncate(owned));
}

fn operator_buttons(player_idx: i32, buttons: u32) -> u64 {
    (u64::from(player_idx as u32) << 32) | u64::from(buttons)
}

/// Cancel plugin intent immediately. An already installed native reload belongs
/// to the native scene teardown; never clear its flags or force its state ahead.
pub fn scene_changed(mode: game_mode::Mode) {
    #[cfg(feature = "native-body-bank-alpha")]
    body_bank::scene_changed();
    #[cfg(feature="mii-saved-alpha")]
    mii_saved_alpha::scene_changed();
    #[cfg(feature="mii-ldn-qualification-probe")]
    mii_ldn_qualification::scene_changed();
    sources::scene_changed();
    #[cfg(feature="mii-hud-portrait-alpha")]
    mii_hud_portrait::scene_changed();
    crate::runtime::roster::character_items::scene_changed();
    let same_lobby = WAITING_MEMORY
        .lock()
        .scene_changed(mode.initialized, mode.melee, mode.mode);
    crate::runtime::roster::kirby_hats::scene_changed(same_lobby);
    training::named_macros::waiting_scene_changed(same_lobby);
    OWNED_BUTTONS.store(0, Ordering::Relaxed);
    OPERATOR_BUTTONS.store(0, Ordering::Relaxed);
    for role in [Role::Player, Role::Cpu] {
        ORIGINAL_KINDS[role as usize].store(u32::MAX, Ordering::Release);
        SOLO_ACTIVE[role as usize].store(false, Ordering::Release);
        let mut state = state_for(role).lock();
        state.epoch = state.epoch.wrapping_add(1);
        state.pending = None;
        state.cursor = None;
        state.loaded = None;
        state.observed = None;
        state.presented.retire();
        REBUILT[role as usize].store(false, Ordering::Release);
        if let Some(active) = state.active.as_mut() {
            active.departed = true;
            trace!("[CharacterRoster] scene departure during reload owner={} target=0x{:x} phase=0x{:x}; native teardown owns cancellation",
                active.scope.owner, active.target.kind(), active.phase);
        }
    }
}

#[inline]
fn aligned(ptr: *const u8) -> bool {
    crate::engine::seams::native_fields::object(ptr as usize)
}

// Required entry fields are borrowed only inside the qualified roster callback
// or install transaction. This helper neither owns nor extends their lifetime.
#[inline]
unsafe fn read<T: Copy>(entry: *const u8, offset: usize) -> T {
    crate::engine::seams::native_fields::read_unchecked(entry as usize, offset)
}

unsafe fn active_body(entry: *const u8, owner: u32, kind: u32) -> bool {
    ready_kind(entry, owner) == Some(kind)
}

unsafe fn ready_kind(entry: *const u8, owner: u32) -> Option<u32> {
    ready_forms(entry, owner).map(|(_, kind)| kind)
}

/// Borrow the outgoing pose only within the qualified visible loader callback.
/// All ordinary/exact-Mii reloads share this preflight; parked sources do not.
unsafe fn capture_reload_pose(entry: *mut u8, owner: u32) -> Option<native_reload_pose::ReloadPose> {
    use smash::app::{lua_bind::{PostureModule, StatusModule}, BattleObjectModuleAccessor};
    use smash::lib::lua_const::{FIGHTER_STATUS_KIND_DEAD, FIGHTER_STATUS_KIND_REBIRTH,
        FIGHTER_STATUS_KIND_ENTRY, FIGHTER_STATUS_KIND_STANDBY};
    native_reload_pose::capture(BASE.load(Ordering::Acquire), entry as usize, owner, |module| {
        let module = module as *mut BattleObjectModuleAccessor;
        let status = StatusModule::status_kind(module);
        if !crate::runtime::compat::common::is_ready_go()
            || [*FIGHTER_STATUS_KIND_DEAD, *FIGHTER_STATUS_KIND_REBIRTH,
                *FIGHTER_STATUS_KIND_ENTRY, *FIGHTER_STATUS_KIND_STANDBY].contains(&status)
        {
            return Some(native_reload_pose::SpawnPose::Native);
        }
        let position = PostureModule::pos(module);
        if position.is_null() { return None; }
        Some(native_reload_pose::SpawnPose::Current(
            [(*position).x, (*position).y, (*position).z], PostureModule::lr(module)))
    })
}

/// Same callback and entry as preflight, INSTALL owned, no intervening retirement.
/// Stage before native teardown so its asynchronous constructor sees this pose.
unsafe fn reload_at_pose(
    base: usize,
    entry: *mut u8,
    descriptor: &native_fighter_install::Descriptor,
    pose: native_reload_pose::ReloadPose,
) {
    // A Reload destroys only the native window: detached bank groups go first.
    #[cfg(feature = "native-body-bank-alpha")]
    body_bank::before_reload(entry);
    pose.stage();
    native_fighter_install::install(base, entry, descriptor, native_fighter_install::Flags::Reload);
}

unsafe fn ready_forms(entry: *const u8, owner: u32) -> Option<(Forms, u32)> {
    if !aligned(entry) || read::<u32>(entry, 0xe0) != owner {
        return None;
    }
    #[cfg(feature = "native-body-bank-alpha")]
    if let Some(ready) = body_bank::ready_forms(entry, owner) {
        return Some(ready);
    }
    let state = EntryState {
        count: read(entry, 0x14),
        kinds: [read(entry, 0x18), read(entry, 0x1c), read(entry, 0x20)],
        actor_count: read(entry, 0x28),
        installed: read(entry, 0x4150),
        index: read(entry, 0x5918),
        previous_or_next: read(entry, 0x5919),
        pending: read(entry, 0x591a),
        flags: read(entry, 0x591f),
        outer: read(entry, 0x5920),
        loader: read(entry, 0x591c),
        changing: read(entry, 0x5934),
        guard: read(entry, 0x5935),
    };
    TrioFighter::from_kind(state.kinds[0])?;
    let (forms, live_kind) = state.ready()?;
    for index in 0..forms.count as usize {
        let body = read::<*const u8>(entry, 0x4160 + index * 8);
        if !aligned(body)
            || read::<usize>(body, 0) != BASE.load(Ordering::Acquire) + 0x4f80210
            || read::<u32>(body, 0xc) != forms.kinds[index]
            || read::<u64>(body, 0x10) != u64::from(owner)
            || read::<u8>(body, 0x3a) != forms.body_state(index, state.index)
            || !aligned(read::<*const u8>(body, 0x20))
        {
            return None;
        }
    }
    Some((forms, live_kind))
}

/// `ready_forms` without the roster catalog admission: single-kind Mii entries only.
#[cfg(feature = "mii-saved-alpha")]
unsafe fn ready_single_mii(entry: *const u8, owner: u32) -> Option<u32> {
    if !aligned(entry) || read::<u32>(entry, 0xe0) != owner {
        return None;
    }
    let state = EntryState {
        count: read(entry, 0x14),
        kinds: [read(entry, 0x18), read(entry, 0x1c), read(entry, 0x20)],
        actor_count: read(entry, 0x28),
        installed: read(entry, 0x4150),
        index: read(entry, 0x5918),
        previous_or_next: read(entry, 0x5919),
        pending: read(entry, 0x591a),
        flags: read(entry, 0x591f),
        outer: read(entry, 0x5920),
        loader: read(entry, 0x591c),
        changing: read(entry, 0x5934),
        guard: read(entry, 0x5935),
    };
    if !(0x48..=0x4a).contains(&state.kinds[0]) {
        return None;
    }
    // A Mii one-member bank window (flags 4, count 1, `R` holding the Mii identity)
    // is the same single body as the canonical entry (MII-GROUPS-DESIGN §4): the
    // value rule `single_mii_bank_window` names the window shape, the bank proves
    // its own descriptor and body receipts, and the body checks below still apply.
    #[cfg(feature = "native-body-bank-alpha")]
    let ready = state.ready().or_else(|| {
        let forms = crate::engine::swap::body_bank::single_mii_bank_window(state)?;
        body_bank::ready_forms(entry, owner).filter(|(bank_forms, _)| *bank_forms == forms)
    });
    #[cfg(not(feature = "native-body-bank-alpha"))]
    let ready = state.ready();
    let (forms, live_kind) = ready?;
    if forms.count != 1 {
        return None;
    }
    let body = read::<*const u8>(entry, 0x4160);
    if !aligned(body)
        || read::<usize>(body, 0) != BASE.load(Ordering::Acquire) + 0x4f80210
        || read::<u32>(body, 0xc) != live_kind
        || read::<u64>(body, 0x10) != u64::from(owner)
        || read::<u8>(body, 0x3a) != forms.body_state(0, state.index)
        || !aligned(read::<*const u8>(body, 0x20))
    {
        return None;
    }
    Some(live_kind)
}

/// Existing readiness readers only; no native calls or Rust mutexes (also used under G lock).
#[cfg(feature = "mii-saved-alpha")]
pub(crate) unsafe fn mii_saved_cpu_visible_ready(entry: *const u8, owner: u32, kind: u32) -> bool {
    if !(0x48..=0x4a).contains(&kind) { return false; }
    match owner {
        0 => ready_forms(entry, owner).is_some_and(|(_, kind)| kind == 9),
        1 => ready_single_mii(entry, owner) == Some(kind),
        _ => false,
    }
}

/// Called before G lock; the current probe may already own INSTALL, never a roster replacement.
#[cfg(feature="mii-saved-alpha")]
pub(crate) unsafe fn mii_cpu_probe_roster_idle() -> bool {
    native_source::training_cpu_quantity() == Some(1)
        && STATES.iter().all(|state| {
            let state = state.lock();
            state.active.is_none() && state.pending.is_none() && state.loaded.is_none()
        })
}

// This current native callback owns entry lifetime. Cached numeric identities
// below are compared only; no saved fighter/entry/module pointer is dereferenced.
// Pre-service dispatch only. Returning here never suppresses the native call.
unsafe fn service(ctx: &ServiceEntryContext, entry: *mut u8) {
    crate::runtime::roster::hang_trace::ROSTER_SERVICE.hit();
    if !aligned(entry) {
        return;
    }
    let owner = read::<u32>(entry, 0xe0);
    #[cfg(feature = "native-body-bank-alpha")]
    body_bank::service(entry, owner);
    #[cfg(feature="mii-ldn-qualification-probe")]
    mii_ldn_qualification::service(entry, owner);
    sources::service(ctx, entry, owner);
    #[cfg(feature="mii-hud-portrait-alpha")]
    mii_hud_portrait::service(entry, owner);
    republish_after_rebuild(entry, owner);
    service_body(entry,owner,ctx.caller_lr());
}

unsafe fn service_body(entry:*mut u8,owner:u32,_caller_lr:usize) {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080000);
    if !matches!(owner, 0 | 1 | 4) {
        return;
    }
    let role = if owner == 0 { Role::Player } else { Role::Cpu };
    #[cfg(feature="mii-saved-alpha")]
    mii_catalog_request::service();
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    mii_auto_init::service(entry,owner);
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    mii_replay::service(entry,owner);
    #[cfg(feature="mii-ordinary-player-alpha")]
    if mii_ordinary_player::service(entry,owner){return;}
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    if mii_ordinary_cpu::service(entry,owner){return;}
    #[cfg(feature="mii-saved-alpha")]
    if mii_saved_alpha::service(entry,owner){return;}
    let active = state_for(role).lock().active;
    if let Some(mut active) = active {
        if active.entry != entry as usize || active.scope.owner != owner {
            return;
        }
        active.ticks += 1;
        let phase = u32::from(read::<u8>(entry, 0x5920))
            | (u32::from(read::<u8>(entry, 0x591c)) << 8)
            | (u32::from(read::<u8>(entry, 0x5935)) << 16);
        if phase != active.phase {
            active.phase = phase;
            trace!(
                "[CharacterRoster] phase owner={} target=0x{:x} outer={} loader={} guard={} tick={}",
                owner,
                active.target.kind(),
                phase & 255,
                (phase >> 8) & 255,
                phase >> 16,
                active.ticks
            );
        }
        if !active.departed
            && scope(role) == Some(active.scope)
            && ready_forms(entry, owner) == Some((active.forms, active.target.kind()))
            && read::<u32>(entry, 0x18) == active.target.kind()
            && read::<u8>(entry, 0x5918) == 0
        {
            let mut state = state_for(role).lock();
            if state.epoch != active.epoch
                || state.active.map(|current| current.install) != Some(active.install)
            {
                return;
            }
            state.active = None;
            state.loaded = Some((entry as usize, owner, active.target.kind()));
            state.observed = Some((entry as usize, active.target.kind()));
            state.cursor = active.slot.map(|old| {
                if state.revision == active.revision { old }
                else { locate(&setting(&MENU), role, active.target.kind()).unwrap_or(old) }
            });
            if active.scope.waiting() {
                WAITING_MEMORY
                    .lock()
                    .commit(role as usize, WaitingValue::ordinary(active.target,active.forms,active.slot));
            }
            drop(state);
            if active.scope.waiting() {
                crate::runtime::roster::kirby_hats::fighter_committed(role as usize, active.target.kind());
            }
            publish_partners(role, active.forms);
            role.sync_selection(active.target);
            #[cfg(feature="mii-hud-portrait-alpha")]
            mii_hud_portrait::before_refresh(entry, owner, active.target.kind());
            refresh_hud(entry, owner, active.target.kind());
            #[cfg(feature = "mii-ordinary-player-alpha")]
            if owner == 0 {
                mii_ordinary_player::complete_generic(entry, active.install, active.target.kind());
            }
            #[cfg(feature = "mii-ordinary-cpu-alpha")]
            if role == Role::Cpu {
                mii_ordinary_cpu::complete_generic(entry, active.install, active.target.kind(), active.slot);
            }
            crate::runtime::roster::cycle_preload::foreground_settled();
            finish_install(active.install, "roster ready");
            crate::runtime::roster::cycle_preload::completed(role as usize,false,active.target.kind());
            trace!("[CharacterRoster] complete owner={} target={} kind=0x{:x} slot={:?} ticks={} body_id=0x{:x}",
                owner, active.target.name(), active.target.kind(), active.slot.map(|i| i+1),
                active.ticks, read::<u32>(read::<*const u8>(entry, 0x4160), 8));
            notify_selection(role, active.target, active.slot);
        } else {
            // Departure cancels plugin intent, but the native entry may finish
            // before it is destroyed. Release only serialization; never publish
            // its old selection, HUD or readiness into the new scene.
            if active.departed
                && ready_forms(entry, owner) == Some((active.forms, active.target.kind()))
                && read::<u8>(entry, 0x5918) == 0
            {
                let mut state = state_for(role).lock();
                if state.active.map(|current| current.install) == Some(active.install) {
                    state.active = None;
                    drop(state);
                    finish_install(active.install, "departed roster completed");
                    trace!("[CharacterRoster] departed install completed owner={}; selection remains cancelled", owner);
                }
                return;
            }
            if active.ticks == 600 {
                trace!("[CharacterRoster] reload stalled owner={} phase=0x{:x} departed={}; no forced native advance", owner, phase, active.departed);
            }
            let mut state = state_for(role).lock();
            if let Some(current) = state.active.as_mut() {
                if current.entry == active.entry && current.epoch == active.epoch {
                    // Preserve a scene-change cancellation from another callback.
                    current.ticks = active.ticks;
                    current.phase = active.phase;
                }
            }
        }
        return;
    }
    if INSTALL.active() {
        return;
    } // Serialize player and CPU resource mutations.
    let request = state_for(role).lock().pending;
    let Some(mut request) = request else {
        // Display the live fighter in the picker, including the initial CSS choice.
        // This permits choosing the same menu fighter again after a cycle.
        if let Some(current_scope) =
            scope(role).filter(|s| s.owner == owner && !menu::captures_input())
        {
            let Some((forms, kind)) = ready_forms(entry, owner) else {
                return;
            };
            if let Some(fighter) = TrioFighter::from_kind(kind) {
                let mut state = state_for(role).lock();
                // A new same-owner preview can coexist with a departing entry
                // before the mode changes. Keep the identity acquired for this
                // scene until its destructor or scene invalidation releases it.
                if state
                    .observed
                    .is_some_and(|(identity, _)| identity != entry as usize)
                {
                    return;
                }
                let newly_observed = state.observed != Some((entry as usize, kind));
                if newly_observed {
                    let _ = ORIGINAL_KINDS[role as usize].compare_exchange(
                        u32::MAX,
                        kind,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    );
                    state.observed = Some((entry as usize, kind));
                    if state.loaded.is_some_and(|(identity, loaded_owner, _)| {
                        identity == entry as usize && loaded_owner == owner
                    }) {
                        state.loaded = Some((entry as usize, owner, kind));
                    }
                    state.presented.observe(entry as usize, owner, kind);
                    trace!(
                        "[CharacterRoster] observed owner={} entry=0x{:x} kind=0x{:x} epoch={}",
                        owner,
                        entry as usize,
                        kind,
                        state.epoch
                    );
                }
                let slot = state.cursor;
                drop(state);
                publish_partners(role, forms);
                if newly_observed {
                    // Exact native origin capture supplies a saved Mii default. A kind-only
                    // observation must not suppress its configuration on a later return.
                    #[cfg(feature="mii-saved-alpha")]
                    let exact_origin_pending=mii_backend::current().is_some_and(|l|l.waiting())
                        && (0x48..=0x4a).contains(&kind)
                        && ((role==Role::Player && cfg!(feature="mii-ordinary-player-alpha"))
                            || (role==Role::Cpu && cfg!(feature="mii-ordinary-cpu-alpha")));
                    #[cfg(not(feature="mii-saved-alpha"))]
                    let exact_origin_pending=false;
                    let fresh_lobby = current_scope.waiting()
                        && !exact_origin_pending
                        && WAITING_MEMORY
                            .lock()
                            .observe(role as usize, WaitingValue::ordinary(fighter,forms,None));
                    if fresh_lobby {
                        trace!("[CharacterRoster] fresh waiting entry lifetime; cleared both practice choices");
                        crate::runtime::roster::kirby_hats::fresh_lobby();
                        training::named_macros::waiting_scene_changed(false);
                    }
                    role.sync_selection(fighter);
                }
                // A ready entry can precede the scene's first gameplay frame.
                // Reloading it during stage initialization races the native
                // fighter/script setup. Reconcile persisted Solo only once the
                // match is active; menu changes use the same native boundary.
                if kind == 0x4b
                    && crate::runtime::compat::common::is_ready_go()
                    && forms != requested_forms_at(&setting(&MENU), role, kind, slot)
                {
                    queue(role, Selection::RefreshPartners);
                    return;
                }
                if current_scope.waiting() && role.enabled() {
                    let target = WAITING_MEMORY.lock().restore_target(role as usize);
                    if let Some(target) = target {
                        if restore_owned(role,target) {return;}
                        if target.fighter.kind() == kind && target.forms==forms {
                            WAITING_MEMORY.lock().commit(role as usize, target);
                            crate::runtime::roster::kirby_hats::fighter_committed(role as usize, kind);
                            state_for(role).lock().cursor=target.cursor;
                            role.sync_selection(target.fighter);
                        } else {
                            queue_request(role, Selection::Direct(target.fighter), true);
                        }
                    }
                }
            }
        }
        return;
    };
    if request.scope.owner != owner || request.entry != entry as usize {
        return;
    }
    let mut state = state_for(role).lock();
    if request.epoch != state.epoch
        || scope(role) != Some(request.scope)
        || (!role.enabled() && !request.once && !matches!(request.selection, Selection::RefreshPartners))
    {
        state.pending = None;
        trace!("[CharacterRoster] dropped stale request at native boundary");
        return;
    }
    request.age += 1;
    // A committed menu change waits for the menu's normal input release grace.
    if menu::captures_input() && request.age < 180 {
        state.pending = Some(request);
        return;
    }
    state.pending = None;
    let current = ready_kind(entry, owner);
    if menu::captures_input()
        || lobby::native_ui_busy()
        || button_config::has_pending_request()
        || training::save_states::is_loading()
        || training::recording_or_playback()
        || current.is_none()
    {
        drop(state);
        if !request.restoring {
            trace!(
                "[CharacterRoster] rejected at native boundary: fighter not idle or another action owns practice"
            );
            notify(role, "Character swap unavailable during this action".into());
        }
        return;
    }
    let current = current.unwrap();
    let m = setting(&MENU);
    let (target, slot) = match request.selection {
        Selection::Direct(target) => (target, None),
        Selection::RefreshPartners => (TrioFighter::from_kind(current).unwrap(), state.cursor),
        Selection::Cycle => {
            let choices = roster(&m, role);
            let kinds: Vec<_> = choices.iter().map(|fighter| fighter.kind()).collect();
            let original = cycle_original(role);
            // Slot 0 is told apart from a numbered slot of the same group (Ice Climbers whose
            // Solo preference differs) by the cursor, which a by-group search cannot see.
            let at_original = state.cursor.is_none()
                && original.is_some_and(|kind| cycle_position(&[kind], current).is_some());
            let step = if at_original {
                next_from_original(&kinds, original)
            } else {
                next_cycle(current, state.cursor, &kinds, original)
            };
            let Some((kind, slot)) = step else {
                drop(state);
                notify(role, "Select a character for this cycle".into());
                return;
            };
            let Some(target) = TrioFighter::from_kind(kind) else {
                return;
            };
            (target, slot)
        }
    };
    let remembered=if request.restoring {WAITING_MEMORY.lock().restore_target(role as usize)}else{None};
    let slot=remembered.map_or(slot,|v|v.cursor);
    let forms = remembered.map_or_else(||requested_forms_at(&m, role, target.kind(), slot),|v|v.forms);
    if ready_forms(entry, owner) == Some((forms, target.kind())) {
        state.cursor = slot;
        if request.scope.waiting() {
            WAITING_MEMORY.lock().commit(role as usize, WaitingValue::ordinary(target,forms,slot));
        }
        drop(state);
        if request.scope.waiting() {
            crate::runtime::roster::kirby_hats::fighter_committed(role as usize, target.kind());
        }
        role.sync_selection(target);
        notify_selection(role, target, slot);
        trace!(
            "[CharacterRoster] already active owner={} kind=0x{:x} slot={:?}",
            owner,
            current,
            slot.map(|i| i + 1)
        );
        return;
    }
    let Some(pose) = capture_reload_pose(entry, owner) else {
        drop(state);
        notify(role, "Character swap unavailable: current position not ready".into());
        return;
    };
    let Some(install_token) = INSTALL.try_begin(true) else {
        state.pending = Some(request);
        return;
    };
    let active = Active {
        install: install_token,
        scope: request.scope,
        epoch: state.epoch,
        revision: state.revision,
        entry: entry as usize,
        target,
        forms,
        slot,
        ticks: 0,
        phase: u32::MAX,
        departed: false,
    };
    state.loaded = None;
    state.presented.retire();
    REBUILT[role as usize].store(false, Ordering::Release);
    state.active = Some(active);
    drop(state);
    // 653240 synchronously copies this descriptor. Preserve all native role,
    // controller/team/costume options; replace the complete canonical group.
    let mut descriptor = native_fighter_install::Descriptor::copy_from(entry);
    descriptor.request_group(native_fighter_install::Group {
        count: forms.count,
        kinds: forms.kinds,
        actor_count: forms.actor_count,
    });
    training::clear_session_state();
    trace!("[CharacterRoster] install owner={} entry=0x{:x} from=0x{:x} to=0x{:x} slot={:?} epoch={} mode=0x{:x}",
        owner, entry as usize, current, target.kind(), slot.map(|i| i+1), active.epoch, active.scope.mode);
    #[cfg(feature = "console-swap-trace")]
    crate::runtime::roster::practice_trace::checkpoint(901, [entry as u64, u64::from(owner), u64::from(current), u64::from(target.kind()), active.epoch as u64]);
    #[cfg(feature = "mii-ordinary-player-alpha")]
    if owner == 0 {
        mii_ordinary_player::begin_generic(entry, install_token, descriptor.bytes());
    }
    #[cfg(feature = "mii-ordinary-cpu-alpha")]
    if role == Role::Cpu {
        mii_ordinary_cpu::begin_generic(entry, install_token, descriptor.bytes());
    }
    #[cfg(feature = "console-swap-trace")]
    crate::runtime::roster::practice_trace::checkpoint(902, [entry as u64, u64::from(owner), u64::from(current), u64::from(target.kind()), active.epoch as u64]);
    let base = BASE.load(Ordering::Acquire);
    #[cfg(feature = "native-unlink-probe")]
    crate::runtime::roster::native_unlink_probe::arm(entry as usize, owner, current, target.kind());
    reload_at_pose(base, entry, &descriptor, pose);
    #[cfg(feature = "console-swap-trace")]
    crate::runtime::roster::practice_trace::checkpoint(903, [entry as u64, u64::from(owner), u64::from(current), u64::from(target.kind()), active.epoch as u64]);
}

/// Called before native entry destruction, while that argument is still owned.
pub unsafe fn on_destroy(entry: *const u8) {
    if !aligned(entry) {
        return;
    }
    let owner = read::<u32>(entry, 0xe0);
    // R3-1: revoke both body registrations before body-bank/nested destruction.
    training::practice::entry_departed(entry as usize, owner);
    #[cfg(feature = "native-body-bank-alpha")]
    body_bank::on_destroy(entry);
    #[cfg(feature="mii-saved-alpha")]
    mii_saved_alpha::on_destroy(entry,owner);
    #[cfg(feature="mii-ordinary-player-alpha")]
    mii_ordinary_player::on_destroy(entry);
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    mii_ordinary_cpu::on_destroy(entry);
    #[cfg(feature="mii-ldn-qualification-probe")]
    mii_ldn_qualification::on_destroy(entry, owner);
    sources::on_destroy(entry, owner);
    #[cfg(feature="mii-hud-portrait-alpha")]
    mii_hud_portrait::on_destroy(entry, owner);
    if !matches!(owner, 0 | 1 | 4) {
        return;
    }
    let role = if owner == 0 { Role::Player } else { Role::Cpu };
    let mut state = state_for(role).lock();
    let practice_departure = role == Role::Player
        && (state
            .observed
            .is_some_and(|(identity, _)| identity == entry as usize)
            || state
                .active
                .is_some_and(|active| active.entry == entry as usize));
    if role == Role::Player
        && lobby::is_waiting_scene()
        && (state
            .observed
            .is_some_and(|(identity, _)| identity == entry as usize)
            || state.active.is_some_and(|a| {
                a.entry == entry as usize
                    && a.epoch == state.epoch
                    && !a.departed
                    && a.scope.waiting()
            }))
    {
        WAITING_MEMORY.lock().player_departed();
    }
    if state.pending.is_some_and(|r| r.entry == entry as usize) {
        trace!(
            "[CharacterRoster] canceled queued request on entry destruction owner={} entry=0x{:x}",
            owner,
            entry as usize
        );
        state.pending = None;
    }
    if state
        .loaded
        .is_some_and(|(identity, _, _)| identity == entry as usize)
    {
        state.loaded = None;
        state.cursor = None;
    }
    state.presented.retire_entry(entry as usize);
    if state.observed.is_some_and(|(identity, _)| identity == entry as usize) {
        REBUILT[role as usize].store(false, Ordering::Release);
    }
    if state
        .observed
        .is_some_and(|(identity, _)| identity == entry as usize)
    {
        trace!(
            "[CharacterRoster] invalidated observed entry before destruction owner={} entry=0x{:x}",
            owner,
            entry as usize
        );
        state.observed = None;
        // Mii adapters publish a cursor without setting the ordinary loader's
        // `loaded` receipt. Retire it with the observed entry as well: Training
        // -> CSS -> Training can keep the same mode tuple, and a new CSS fighter
        // must start at slot 0. Lobby continuation lives in WAITING_MEMORY.
        state.cursor = None;
        ORIGINAL_KINDS[role as usize].store(u32::MAX, Ordering::Release);
        SOLO_ACTIVE[role as usize].store(false, Ordering::Release);
    }
    if state.active.is_some_and(|a| a.entry == entry as usize) {
        trace!("[CharacterRoster] entry_destroy owner={} active={:?}; references invalidated before native destruction", owner, state.active);
        let active = state.active.take().unwrap();
        state.cursor = None;
        finish_install(active.install, "roster destroyed");
    }
    drop(state);
    if practice_departure {
        // Native Training -> CSS -> Training retains the same mode tuple. The
        // actual P1 entry destructor is also a scene boundary for plugin intent.
        sources::scene_changed();
        #[cfg(feature="mii-hud-portrait-alpha")]
        mii_hud_portrait::scene_changed();
        crate::runtime::roster::kirby_hats::practice_departed(lobby::is_waiting_scene());
        crate::runtime::roster::character_items::scene_changed();
        trace!("[CharacterRoster] practice consumers invalidated on P1 entry destruction");
    }
}

/// Exact Miis keep their single-body proof. An ordinary bank is admitted only
/// through its owned descriptor and all current body receipts; flags4 alone is
/// never enough. Presentation names the active logical member, not kind word0.
#[cfg(feature = "mii-saved-alpha")]
unsafe fn presentation_ready(entry: *const u8, owner: u32, kind: u32) -> bool {
    if !active_body(entry, owner, kind) { return false; }
    #[cfg(feature = "native-body-bank-alpha")]
    if body_bank::ready_kind(entry, owner) == Some(kind) { return true; }
    read::<u32>(entry, 0x18) == kind && read::<u8>(entry, 0x5918) == 0
}

/// Called only after a Mii adapter accepts its exact completion, while its
/// current callback and INSTALL token are still owned. Publish presentation only;
/// original CSS identity and saved configuration/preferences remain untouched.
#[cfg(feature = "mii-saved-alpha")]
unsafe fn publish_mii_presentation(
    entry: *const u8,
    owner: u32,
    generation: u64,
    kind: u32,
    #[cfg(feature="mii-hud-portrait-alpha")] portrait: PortraitIntent,
) {
    let Some(logical)=mii_backend::current().and_then(|l|l.logical(owner)) else{return;};
    let role = if owner == 0 { Role::Player } else { Role::Cpu };
    let Some(fighter) = TrioFighter::from_kind(kind) else { return; };
    if scope(role).is_none_or(|s|s.owner!=owner)
        || mii_saved_alpha::current_generation(logical as u32) != generation
        || !presentation_ready(entry, owner, kind)
    {
        return;
    }
    let mut state = state_for(role).lock();
    if mii_saved_alpha::current_generation(logical as u32) != generation
        || state.active.is_some()
        || state.pending.is_some()
        || state.observed.is_some_and(|(identity, _)| identity != entry as usize)
        || state.loaded.is_some_and(|(identity, loaded_owner, _)| {
            identity != entry as usize || loaded_owner != owner
        })
    {
        return;
    }
    state.observed = Some((entry as usize, kind));
    // Never `loaded`: that receipt gates saved-CPU idleness. Native reset admits
    // this publication through the presentation-only receipt instead.
    state.presented.publish(entry as usize, owner, kind);
    // This publication supersedes a reset deferred during its INSTALL.
    REBUILT[role as usize].store(false, Ordering::Release);
    drop(state);
    role.sync_selection(fighter);
    #[cfg(feature="mii-hud-portrait-alpha")]
    if mii_hud_portrait::should_defer_hud(owner, kind, portrait) {
        // The exact texture does not exist yet. Keep the previous portrait and let the
        // service callback refresh and bind after the render proof. A rejected hold
        // still paints the new kind, so the old portrait cannot stick.
        let accepted = mii_hud_portrait::request(entry, owner, generation, kind, portrait, true);
        if policy_rejected_hud(accepted) {
            mii_hud_portrait::before_refresh(entry, owner, kind);
            refresh_hud(entry, owner, kind);
        }
    } else {
        mii_hud_portrait::before_refresh(entry, owner, kind);
        refresh_hud(entry, owner, kind);
        mii_hud_portrait::request(entry, owner, generation, kind, portrait, false);
    }
    // WP-P shadow readback (rule 5): the card after this publication, value-only. A deferred
    // exact bust is still pending here; the exact job's own bind is read back by the reset
    // and state-load hooks, so only a non-exact publication is judged against "no bust".
    #[cfg(feature="mii-hud-portrait-alpha")]
    if !matches!(portrait, PortraitIntent::Exact(_)) {
        crate::runtime::roster::presentation::shadow_readback(BASE.load(Ordering::Acquire), owner, kind, false, "publish");
    }
    #[cfg(not(feature="mii-hud-portrait-alpha"))]
    refresh_hud(entry, owner, kind);
}

/// The publish site is the engine entry-service callback, after
/// `mii_hud_portrait::service` has returned, so this call does not hold that
/// function's `ServiceCallback`. Rejection still falls back here.
#[cfg(feature="mii-hud-portrait-alpha")]
fn policy_rejected_hud(accepted: bool) -> bool {
    mii_hud_portrait_policy::rejected_deferral(true, accepted, true, true)
        == mii_hud_portrait_policy::HudStep::RefreshFallback
}

/// Native reset already owns the HUD lookup. Admit our fresh, active ordinary fighter
/// body into its existing presentation helper; never retain a HUD pane reference. A Mii
/// publication keeps native's own rebuild and is republished on the service callback.
/// Both native reset hooks arrive here: one short roster lock, no native calls.
pub unsafe fn allow_portrait(ctx: &mut InlineCtx, entry_id: u64, entry: *const u8) {
    if entry_id > 1 {
        return;
    }
    let role = if entry_id == 0 {
        Role::Player
    } else {
        Role::Cpu
    };
    if busy() {
        // The caller already qualified native Training. INSTALL may be inside a
        // native call made with the roster lock held: record no pointers and take
        // no lock here. The service callback evaluates the current receipt later.
        REBUILT[role as usize].store(true, Ordering::Release);
        return;
    }
    if scope(role).is_none_or(|s| s.mode != 0x04040000) {
        return;
    }
    let rebuild = {
        let state = state_for(role).lock();
        state
            .presented
            .rebuild(state.loaded, entry as usize, ctx.registers[0].x() as u32)
    };
    match rebuild {
        Rebuild::Admit((_, owner, kind)) => {
            if active_body(entry, owner, kind) {
                ctx.registers[8].set_x(0);
            }
        }
        Rebuild::Republish(_) => REBUILT[role as usize].store(true, Ordering::Release),
        Rebuild::Native => {}
    }
}

/// Native reset rebuilt the original CSS portrait over this role's live Mii publication.
/// On the service callback, after `mii_hud_portrait::service`: revalidate the receipt and
/// the live body, then republish the live kind once. The exact lifecycle restores its own
/// texture. Global INSTALL activity can belong to another role or a helper: defer
/// consumption while it is busy, rather than lose this role's reset notification.
unsafe fn republish_after_rebuild(entry: *mut u8, owner: u32) {
    let role = match owner {
        0 => Role::Player,
        1 => Role::Cpu,
        _ => return,
    };
    if !REBUILT[role as usize].load(Ordering::Acquire) || busy() {
        return;
    }
    if scope(role).is_none_or(|s| s.owner != owner || s.mode != 0x04040000) {
        REBUILT[role as usize].store(false, Ordering::Release);
        return;
    }
    let Some(live) = ready_kind(entry, owner) else { return; };
    let rebuild = {
        let state = state_for(role).lock();
        // Temporary loading and a coexisting preview entry must not consume the
        // visible owner's notification. Its publication or destruction retires it.
        if state.active.is_some() || state.pending.is_some()
            || state.observed.is_some_and(|(identity, _)| identity != entry as usize)
        {
            return;
        }
        state.presented.rebuild(state.loaded, entry as usize, live)
    };
    REBUILT[role as usize].store(false, Ordering::Release);
    let Rebuild::Republish((_, published, kind)) = rebuild else { return; };
    if published != owner {
        return;
    }
    #[cfg(feature = "mii-hud-portrait-alpha")]
    {
        mii_hud_portrait::represent(entry, owner, kind);
        // WP-P shadow readback after the native reset republish (the L+R+A class).
        crate::runtime::roster::presentation::shadow_readback(BASE.load(Ordering::Acquire), owner, kind, false, "native reset");
    }
    #[cfg(not(feature = "mii-hud-portrait-alpha"))]
    {
        refresh_hud(entry, owner, kind);
        trace!("[CharacterRoster] native reset: Mii kind 0x{:x} republished owner={}", kind, owner);
    }
}

#[repr(C)]
struct PortraitEvent {
    vtable: usize,
    event_type: u32,
    entry: u32,
    active_index: u32,
    kind: u32,
}

/// Full reload emits body-ID event1e but not the portrait event1d. Find only
/// the registered HUD listener now and invoke its native functor. The native
/// dispatch depth/deferred-removal protocol protects iterator lifetime: removal
/// is postponed while depth != 0, and the flush runs when this call returns the
/// depth to zero. The functor itself runs on this caller. Publication and the
/// later portrait service are both the 650010 entry callback, so a deferred
/// refresh uses that same callback and does not hop threads.
unsafe fn refresh_hud(entry: *const u8, owner: u32, kind: u32) {
    let role = if owner == 0 { Role::Player } else { Role::Cpu };
    let current_scope = scope(role);
    let waiting_cpu = owner == 4 && current_scope.is_some_and(|s| s.owner == owner);
    let ordinary = owner <= 1 && current_scope.is_some_and(|s| s.mode == 0x04040000);
    if !ordinary && !waiting_cpu {
        trace!(
            "[CharacterRoster] HUD skipped: owner={} scope={:?} game={:?}",
            owner,
            scope(role),
            game_mode::current()
        );
        return;
    }
    let base = BASE.load(Ordering::Acquire);
    let symbol = setting(&crate::runtime::compat::common::FIGHTER_MANAGER_ADDR);
    let hud_singleton = read::<usize>(base as *const u8, 0x52c3420);
    if symbol == 0 || hud_singleton == 0 {
        trace!(
            "[CharacterRoster] HUD skipped: manager_symbol=0x{:x} hud_singleton=0x{:x}",
            symbol,
            hud_singleton
        );
        return;
    }
    let fighter_manager = std::ptr::read_volatile(symbol as *const *mut u8);
    if !aligned(fighter_manager) {
        trace!(
            "[CharacterRoster] HUD skipped: fighter_manager=0x{:x}",
            fighter_manager as usize
        );
        return;
    }
    // The exported singleton is the FighterManager, whose first field points
    // to the event dispatcher. Native65617c/656194 perform these two loads.
    // The dispatch count/lists/depth belong to that second object.
    let manager = read::<*mut u8>(fighter_manager, 0);
    if !aligned(manager) {
        trace!(
            "[CharacterRoster] HUD skipped: manager=0x{:x}",
            manager as usize
        );
        return;
    }
    let list_count = read::<u32>(manager, 0);
    if list_count <= 0x1d {
        trace!(
            "[CharacterRoster] HUD skipped: manager=0x{:x} list_count={}",
            manager as usize,
            list_count
        );
        return;
    }
    let lists = read::<*mut u8>(manager, 8);
    if !aligned(lists) {
        trace!(
            "[CharacterRoster] HUD skipped: lists=0x{:x} list_count={}",
            lists as usize,
            list_count
        );
        return;
    }
    let sentinel = lists.add(0x2c0);
    let event = PortraitEvent {
        vtable: base + 0x4f82ef8,
        event_type: 0x1d,
        entry: read(entry, 0x41e4),
        active_index: {
            #[cfg(feature = "native-body-bank-alpha")]
            { if body_bank::ready_kind(entry, owner) == Some(kind) {
                u32::from(read::<u8>(entry, 0x5918))
            } else { 0 } }
            #[cfg(not(feature = "native-body-bank-alpha"))]
            { 0 }
        },
        kind,
    };
    if event.entry != owner {
        trace!(
            "[CharacterRoster] HUD skipped: event_entry={} owner={}",
            event.entry,
            owner
        );
        return;
    }
    let depth = read::<u32>(manager, 0x18);
    if depth == u32::MAX {
        trace!("[CharacterRoster] HUD skipped: invalid dispatch depth");
        return;
    }
    crate::engine::seams::native_fields::write_unchecked(manager as usize, 0x18, depth + 1);
    let mut node = read::<*const u8>(sentinel, 0);
    let mut delivered = 0;
    let mut visited = 0;
    while node != sentinel && aligned(node) && visited < 64 {
        visited += 1;
        let listener = read::<*mut u8>(node, 0x10);
        if aligned(listener) {
            trace!(
                "[CharacterRoster] HUD listener owner={} vtable=0x{:x} member=0x{:x}",
                owner,
                read::<usize>(listener, 0).wrapping_sub(base),
                read::<usize>(listener, 0x18).wrapping_sub(base)
            );
        }
        if aligned(listener)
            && read::<usize>(listener, 0) == base + 0x5063320
            && read::<usize>(listener, 0x18) == base + 0x1377b40
            && read::<usize>(listener, 0x20) == 0
            && aligned(read::<*const u8>(listener, 0x10))
            && read::<usize>((base + 0x5063320) as *const u8, 0x18) == base + 0x1378ad0
        {
            if waiting_cpu {
                refresh_waiting_icon(listener, owner, kind, base);
            } else {
                let invoke: unsafe extern "C" fn(*mut u8, *const PortraitEvent) =
                    std::mem::transmute(base + 0x1378ad0);
                let hud = pointer_chain(base as *const u8, &[0x52c3420, 8, 0])
                    .map_or(0, |root| root as usize);
                native_hud_refresh::without_roster_animation(hud, owner, || {
                    invoke(listener, &event);
                });
            }
            delivered += 1;
        }
        node = read::<*const u8>(node, 0);
    }
    let remaining = read::<u32>(manager, 0x18) - 1;
    crate::engine::seams::native_fields::write_unchecked(manager as usize, 0x18, remaining);
    if remaining == 0 && read::<u32>(manager, 0x1c) != 0 {
        let flush: unsafe extern "C" fn(*mut u8) = std::mem::transmute(base + 0x37ae770);
        flush(manager);
    }
    trace!(
        "[CharacterRoster] HUD refresh owner={} kind=0x{:x} delivered={} listeners={}",
        owner,
        kind,
        delivered,
        visited
    );
}

/// Diagnostic: how many listeners the FighterManager's dispatcher holds for `event_type`, with
/// their vtable offsets (a read-only walk of the same list `refresh_hud` dispatches on; nothing is
/// invoked). Body event `0x1e` is what `0x655f60` case 7 posts after a group exchange with the ids
/// of `cell[+0x5910]` and `cell[+0x5918]`; whether anything listens in Training bounds how much the
/// stand-in "previous body" id matters (C116.2 `previous_cell_index`).
#[cfg(feature = "diag")]
pub(crate) unsafe fn trace_event_listeners(label: &str, event_type: u32) {
    let base = BASE.load(Ordering::Acquire);
    let symbol = setting(&crate::runtime::compat::common::FIGHTER_MANAGER_ADDR);
    if symbol == 0 {
        return;
    }
    let fighter_manager = std::ptr::read_volatile(symbol as *const *mut u8);
    if !aligned(fighter_manager) {
        return;
    }
    let manager = read::<*mut u8>(fighter_manager, 0);
    if !aligned(manager) || read::<u32>(manager, 0) <= event_type {
        return;
    }
    let lists = read::<*mut u8>(manager, 8);
    if !aligned(lists) {
        return;
    }
    // List heads are 0x18 bytes apart behind an 8-byte header: `0x2c0` for `0x1d`, `0x2d8` for `0x1e`.
    let sentinel = lists.add(8 + 0x18 * event_type as usize);
    let mut node = read::<*const u8>(sentinel, 0);
    let mut vtables = Vec::new();
    while node != sentinel && aligned(node) && vtables.len() < 64 {
        let listener = read::<*mut u8>(node, 0x10);
        vtables.push(if aligned(listener) { read::<usize>(listener, 0).wrapping_sub(base) } else { 0 });
        node = read::<*const u8>(node, 0);
    }
    trace!("{} event=0x{:x} listeners={} vtables={:x?}", label, event_type, vtables.len(), vtables);
}

/// Waiting HUD construction at 14ede24 uses the same eight-record presentation
/// table as ordinary Training, with a distinct inner vtable. Check that fresh
/// registry-owned object before admitting CPU4; never cache a pane or HUD pointer.
unsafe fn refresh_waiting_icon(listener: *const u8, owner: u32, kind: u32, base: usize) {
    match waiting_icon(listener, owner, kind, base) {
        Ok(costume) => trace!(
            "[CharacterRoster] waiting icon owner={} kind=0x{:x} refreshed=true costume={}",
            owner,
            kind,
            costume
        ),
        Err(reason) => trace!(
            "[CharacterRoster] waiting icon owner={} kind=0x{:x} refreshed=false reason=\"{}\"",
            owner,
            kind,
            reason
        ),
    }
}

/// CPU4's waiting-card icon pane, read fresh through the generation-checked waiting UI
/// handle for both the local and the online waiting controller. No texture assignment
/// and no heap use.
#[cfg(feature="mii-hud-portrait-alpha")]
pub(super) unsafe fn waiting_icon_pane(base: usize, owner: u32) -> Option<*const u8> {
    if owner != 4 {
        return None;
    }
    waiting_icon_source(base).map(|(_, pane)| pane)
}

/// Undo a custom waiting portrait with the same native kind assignment used by
/// ordinary swaps. Resolve the registered listener and generation-checked pane
/// now; never use a saved UI address during departure. Success retires its native
/// texture override, allowing the portrait batch to be destroyed afterward.
#[cfg(feature="mii-hud-portrait-alpha")]
pub(super) unsafe fn restore_waiting_kind_icon(base: usize, kind: u32) -> bool {
    waiting_icon_source(base).is_some_and(|(listener, _)| waiting_icon(listener, 4, kind, base).is_ok())
}

/// F-HUD-c readback for `[Hud] readback … icon=` (diag): CPU4's bound waiting icon against the icon its body's own
/// costume names (`R+0x64` of the CPU entry, the word the body's files are keyed on), plus the row colour the publisher
/// reads. Value only; the cache lookup is the one `waiting_icon` makes. Err when the card or `R` is not present.
#[cfg(feature="mii-hud-portrait-alpha")]
pub(crate) unsafe fn waiting_icon_observation(
    base: usize, kind: u32,
) -> Result<crate::engine::swap::swap_model::presentation::WaitingIconObservation, &'static str> {
    use crate::engine::swap::swap_model::presentation as model;
    let (listener, _) = waiting_icon_source(base).ok_or("waiting card unavailable")?;
    let (icon, ui) = waiting_partner_icon(listener, model::WAITING_CPU_OWNER, base, false)
        .ok_or("waiting card unavailable")?;
    let r = native_source::resource_owner(model::WAITING_CPU_OWNER).ok_or("CPU resource owner unavailable")?;
    let body_costume = read::<u32>(r, 0x64);
    let expected = waiting_icon_texture(ui, base, kind, model::waiting_icon_costume(kind, body_costume))
        .map_or(0, |(_, word)| word);
    let picture = pointer_chain(icon, &[8]).ok_or("partner picture absent")?;
    Ok(model::WaitingIconObservation {
        body_costume,
        row_costume: row_colour(base, model::WAITING_CPU_OWNER),
        bound: crate::engine::seams::native_fields::read::<usize>(picture as usize, 0x130).ok_or("partner picture unreadable")?,
        expected,
    })
}

#[cfg(feature="mii-hud-portrait-alpha")]
unsafe fn waiting_icon_source(base: usize) -> Option<(*const u8, *const u8)> {
    let symbol = setting(&crate::runtime::compat::common::FIGHTER_MANAGER_ADDR);
    if symbol == 0 || read::<usize>(base as *const u8, 0x52c3420) == 0 {
        return None;
    }
    let fighter_manager = read::<*mut u8>(symbol as *const u8, 0);
    if !aligned(fighter_manager) {
        return None;
    }
    let manager = read::<*mut u8>(fighter_manager, 0);
    if !aligned(manager) || read::<u32>(manager, 0) <= 0x1d {
        return None;
    }
    let lists = read::<*mut u8>(manager, 8);
    if !aligned(lists) {
        return None;
    }
    let sentinel = lists.add(0x2c0);
    let mut node = read::<*const u8>(sentinel, 0);
    let mut visited = 0;
    while node != sentinel && aligned(node) && visited < 64 {
        visited += 1;
        let listener = read::<*mut u8>(node, 0x10);
        if aligned(listener)
            && read::<usize>(listener, 0) == base + 0x5063320
            && read::<usize>(listener, 0x18) == base + 0x1377b40
            && read::<usize>(listener, 0x20) == 0
            && aligned(read::<*const u8>(listener, 0x10))
            && read::<usize>((base + 0x5063320) as *const u8, 0x18) == base + 0x1378ad0
        {
            if let Some(pane) = waiting_card_pane(listener, base) {
                return Some((listener, pane));
            }
        }
        node = read::<*const u8>(node, 0);
    }
    None
}

#[cfg(feature="mii-hud-portrait-alpha")]
unsafe fn waiting_card_pane(listener: *const u8, base: usize) -> Option<*const u8> {
    waiting_partner_icon(listener, 4, base, false).map(|(icon, _)| icon)
}

/// Resolve CPU `owner`'s partner icon pane and the waiting UI root that owns its
/// texture cache, through the game's generation-checked waiting UI handle (136aef8).
/// The local (`view+0xd3a == 0`) and online waiting controllers differ only in their
/// offsets; presentation ID -> visible partner index is resolved exactly as the native
/// damage setters do (13cecb0 / 13d0f50), never assuming CPU4 is pane zero. `log`
/// traces rejections for the swap path; the per-tick portrait path stays silent.
/// Reads only; no heap use; no address survives this call.
unsafe fn waiting_partner_icon(
    listener: *const u8, owner: u32, base: usize, log: bool,
) -> Option<(*const u8, *const u8)> {
    if owner != 4 {
        return None;
    }
    let hud = read::<*const u8>(listener, 0x10);
    let inner = read::<*const u8>(hud, 0x20);
    if !aligned(inner) || read::<usize>(inner, 0) != base + 0x5062f60 {
        if log {
            trace!("[CharacterRoster] waiting HUD rejected: inner=0x{:x}", inner as usize);
        }
        return None;
    }
    let records = read::<*const u8>(inner, 0x18);
    if records as usize != base + 0x52c5730 {
        if log {
            trace!("[CharacterRoster] waiting HUD rejected: records=0x{:x}", records as usize);
        }
        return None;
    }
    let slot = read::<i32>(records, owner as usize * 0x260 + 0xc);
    if log {
        trace!("[CharacterRoster] waiting HUD owner={} presentation_slot={}", owner, slot);
    }
    if !(0..8).contains(&slot) {
        return None;
    }
    let root = pointer_chain(base as *const u8, &[0x52c3420, 8, 0])?;
    let context = read::<*const u8>(root, 0xb98);
    if !aligned(context) {
        return None;
    }
    let handle = [base + 0x5063bc0, context as usize, read::<u32>(root, 0xba0) as usize];
    let valid: unsafe extern "C" fn(*const usize) -> bool = std::mem::transmute(base + 0x138b6c0);
    if !valid(handle.as_ptr()) {
        return None;
    }
    let view = pointer_chain(context, &[0])?;
    let online = read::<u8>(view, 0xd3a) != 0;
    let controller = pointer_chain(view, &[if online { 0xa50 } else { 0x420 }])?;
    let control_state = pointer_chain(controller, &[0])?;
    if read::<u8>(control_state, 0x14) == 0 {
        return None;
    }
    let mut indices = [i32::MIN; 3];
    let entries = if online {
        let collection = pointer_chain(controller, &[0x2f0])?;
        if read::<u8>(collection, 0) == 0 {
            return None;
        }
        let begin = read::<*const u8>(collection, 0x18);
        let end = read::<usize>(collection, 0x20);
        let length = end.wrapping_sub(begin as usize);
        if !aligned(begin) || length > 12 || !length.is_multiple_of(4) {
            return None;
        }
        for (i, value) in indices.iter_mut().enumerate().take(length / 4) {
            *value = read::<i32>(begin, i * 4);
        }
        read::<*const u8>(collection, 8)
    } else {
        for (i, value) in indices.iter_mut().enumerate() {
            *value = read::<i32>(controller, 0x150 + i * 4);
        }
        read::<*const u8>(controller, 0x148)
    };
    if !aligned(entries) {
        return None;
    }
    let index = indices.iter().position(|&i| {
        (0..8).contains(&i) && read::<i32>(entries, i as usize * 0x1d8 + 0x50) == slot
    })?;
    let ui = pointer_chain(controller, &[if online { 8 } else { 0x130 }, 0x80])?;
    let partners = pointer_chain(ui, &[if online { 0x20 } else { 0x220 }])?;
    let icon = pointer_chain(partners, &[0x98 + index * 0x58])?;
    Some((icon, ui))
}

/// Repaint CPU4's waiting-card icon as native builds it (`0x13ce298–0x13ce2b4`: `0x323f330(cache, ui_chara, costume, 0)`
/// then `0x3778b80`), at the costume the entry's bodies wear (F-HUD-c): the CPU row's colour `E[4]+0x19`, the byte the
/// entry allocation hands the descriptor (`0x14ec1f8–0x14ec204` → `R+0x64`) and Training's portrait setter `0x13639d0`
/// reads (`crate::engine::swap::swap_model::presentation`). Returns the costume bound.
unsafe fn waiting_icon(listener: *const u8, owner: u32, kind: u32, base: usize) -> Result<u32, &'static str> {
    use crate::engine::swap::swap_model::presentation as model;
    let (icon, ui) = waiting_partner_icon(listener, owner, base, true).ok_or("waiting card unavailable")?;
    let costume = model::waiting_icon_costume(kind, row_colour(base, owner));
    let (image, _) = waiting_icon_texture(ui, base, kind, costume).inspect_err(|reason| {
        trace!(
            "[CharacterRoster] waiting icon texture missing kind=0x{:x} costume={} reason=\"{}\"",
            kind,
            costume,
            reason
        );
    })?;
    let assign: unsafe extern "C" fn(*const u8, *const u8) = std::mem::transmute(base + 0x3778b80);
    // Only the texture binding changes; partner descriptor, damage and CSS
    // stay native. Native setter retains/releases the texture reference.
    assign(icon, image);
    Ok(costume)
}

/// The waiting UI's cached small stock icon for `kind` at `costume` (`0x323f330`, a lookup that never loads) and its
/// texture word `**(image + 8)`, the value `0x3778b80` binds on the partner picture. Err when the cache holds none.
unsafe fn waiting_icon_texture(
    ui: *const u8, base: usize, kind: u32, costume: u32,
) -> Result<(*const u8, usize), &'static str> {
    let cache = pointer_chain(ui, &[0, 0x78]).ok_or("waiting UI cache unavailable")?;
    let texture: unsafe extern "C" fn(*const u8, u64, u32, u32) -> *const u8 =
        std::mem::transmute(base + 0x323f330);
    let image = texture(cache, fighter_kind_hash(base, kind), costume, 0);
    let image_data = pointer_chain(image, &[8]).ok_or("icon image unreadable")?;
    match read::<usize>(image_data, 0) {
        0 => Err("icon texture not cached"),
        word => Ok((image, word)),
    }
}

/// The entry colour of presentation row `E[owner]` (`main+0x52c5730 + 0x260*owner + 0x19`): static main-image rows,
/// read by value. Callers pass the waiting CPU (owner 4).
#[inline]
unsafe fn row_colour(base: usize, owner: u32) -> u32 {
    use crate::engine::swap::swap_model::presentation as model;
    u32::from(read::<u8>(
        (base + model::PRESENTATION_ROWS_OFFSET) as *const u8,
        owner as usize * model::PRESENTATION_ROW_STRIDE + model::PRESENTATION_ROW_COLOUR,
    ))
}

// These are the local/online waiting HUD's resource-registration callbacks,
// before its native load barrier and pane construction. Append small stock icons
// to that same owned loader so swaps need no asynchronous UI work or saved panes.
#[skyline::hook(offset = 0x13cdc00)]
unsafe fn load_local_waiting_icons(context: *const u8) {
    original!()(context);
    preload_waiting_icons(context);
}

#[skyline::hook(offset = 0x13d0360)]
unsafe fn load_online_waiting_icons(context: *const u8) {
    original!()(context);
    preload_waiting_icons(context);
}

unsafe fn preload_waiting_icons(context: *const u8) {
    // Settings are loaded before waiting entry. The practice menu cannot open
    // while Lobby Practice is off; enabling it takes effect on the next entry.
    // Check the persisted setting here, not scene/UI readiness: this callback
    // runs before the waiting HUD's native loading barrier and pane creation.
    if setting(&MENU).lobby_practice != OnOff::ON {
        trace!("[CharacterRoster] waiting icon preload skipped: Lobby Practice off");
        return;
    }
    let Some(loader) = pointer_chain(context, &[0x50, 0, 8, 0, 0xcd0, 0x28]) else {
        trace!("[CharacterRoster] waiting icon preload skipped: no native loader");
        return;
    };
    let base = BASE.load(Ordering::Acquire);
    let request: unsafe extern "C" fn(*const u8, *const u64, i64, u32) =
        std::mem::transmute(base + 0x323c4e0);
    // F-HUD-c: the icons a swap will bind, at the CPU entry's costume. Its row is final here: the waiting entry
    // allocation read it into the CPU's descriptor before this callback (`[TrioBoundary] allocation owner=4` precedes
    // every `waiting icon preload` line), and a TM swap never changes it.
    use crate::engine::swap::swap_model::presentation as model;
    let cpu_costume = row_colour(base, model::WAITING_CPU_OWNER);
    let mut count = 0;
    for fighter in TrioFighter::available() {
        let costume = model::waiting_icon_costume(fighter.kind(), cpu_costume);
        let hash = fighter_picture_hash(base, fighter.kind(), costume, 0);
        if hash & 0xffffffffff != 0 {
            request(
                loader,
                &hash,
                i64::from(read::<i32>(loader, 0x70)),
                read(loader, 0x68),
            );
            count += 1;
        }
    }
    trace!("[CharacterRoster] waiting icon preload requested={} costume={} loader=0x{:x}; native load barrier retained", count, cpu_costume, loader as usize);
}

/// Native UI key builder shared by waiting-icon preloads and menu portraits.
unsafe fn fighter_picture_hash(base: usize, kind: u32, costume: u32, picture: u32) -> u64 {
    let key: unsafe extern "C" fn(u64, u32, u32) -> u64 = std::mem::transmute(base + 0x3238a70);
    key(fighter_kind_hash(base, kind), costume, picture)
}

/// The roster-grid stock icon, queried on the UI job thread. The native builder supplies patch
/// paths, names and regional costume remapping. `picture=2` is the game's stock-icon set
/// (`chara_2`, the 64×64 flat head icon — the art the fork's menu showed, its SmashWiki
/// stock-icon PNGs, which the operator asked the character grids to use; 2026-11-03 decision
/// amended 2026-11-04, SPEC-core-07 Q16), loaded at run time through the game's mod-aware
/// file-resource service instead of baked into the layout. `picture=0` is the game's 128×128
/// `chara_0` portrait crop, not the stock icon.
///
/// The costume is the fighter's default (0), never a presentation-row byte: the grid lists every
/// fighter, and the costume a live entry wears is not a property of the list. The first cut read
/// the role's presentation row (`E[0]`/`E[1]`) here; that byte belongs to the match entries, so a
/// grid followed whatever the entries wore (Eden glm-icons-03: the CPU list drew every fighter at
/// costume 7, wedding Mario and pink DK) or named files that do not exist (glm-gifs-04 and
/// glm-mods-10: blank text tiles; the byte then held a value without a `chara_2_*` file). The
/// tracepoints on the key builder `main+0x3238a70` (glm-icons-03 out/04-trace-grid) captured
/// costume 0 for the Player page and costume 7 for the CPU page of the same grid.
pub(crate) unsafe fn menu_stock_icon_hash(kind: u32) -> Option<u64> {
    let base = BASE.load(Ordering::Acquire);
    // init publishes BASE only after the enforcing character_roster seam gate
    // accepts roster.kind_hash and roster.icon_hash (both 64-byte guards).
    // These carried seams are not members of game::Granted's core Site tables.
    if base == 0 { return None; }
    let hash = fighter_picture_hash(base, kind, 0, 2);
    (hash & 0xffffffffff != 0).then_some(hash)
}

// Both icon paths use this roster module's published, verified text base.
// The conversion borrows no native object and retains no address.
#[inline]
unsafe fn fighter_kind_hash(base: usize, kind: u32) -> u64 {
    let convert: unsafe extern "C" fn(u32) -> u64 = std::mem::transmute(base + 0x138c9b0);
    convert(kind)
}

#[inline]
unsafe fn pointer_chain(object: *const u8, offsets: &[usize]) -> Option<*const u8> {
    crate::engine::seams::native_fields::chain(object as usize, offsets).map(|address| address as *const u8)
}

/// Seam-table rows (`native_seams::rows_a1`) the roster subtree needs: the native service entry, the two
/// lifecycle sites, every function it calls or compares, the waiting-icon hooks and the spawn-pose loads.
/// WP-A1 shadow: the word checks below still decide until `character_roster` is flipped to authoritative.
const ROSTER_SEAMS: &[&str] = &["roster.native_service", "lifecycle.reload_entry", "lifecycle.destruction_entry",
    "roster.listener_identity", "roster.listener_dispatch", "roster.guard_13639d0", "roster.hud_flush",
    "roster.icon_valid", "roster.kind_hash", "roster.icon_texture", "roster.waiting_local_icons",
    "roster.waiting_online_icons", "roster.icon_hash", "roster.icon_request", "roster.icon_assign",
    "roster.spawn_pose_loads", "roster.spawn_pose_facing"];

pub fn init(#[cfg(feature = "managed-third-source-alpha")] load_contract: Option<VerifiedLoad>) {
    unsafe {
        let base = getRegionAddress(Region::Text) as usize;
        let legacy = [
            (
                0x650010,
                [0xd103c3ffu32, 0xa9096ffc, 0xa90a67fa, 0xa90b5ff8],
            ),
            (
                0x653240,
                [0xd101c3ffu32, 0xf90013f9, 0xa9035ff8, 0xa90457f6],
            ),
            (
                0x652160,
                [0xd10103ffu32, 0xa90157f6, 0xa9024ff4, 0xa9037bfd],
            ),
            (
                0x1377b40,
                [0xf9401000u32, 0xb40000e0, 0xf9400009, 0x29418828],
            ),
            (
                0x1378ad0,
                [0xa9410808u32, 0xf9401009, 0x8b890500, 0x36000069],
            ),
            (
                0x13639d0,
                [0xd10203ffu32, 0xf9001bf9, 0xa9045ff8, 0xa90557f6],
            ),
            (
                0x37ae770,
                [0xb9401c08u32, 0x340007e8, 0xa9bb67fa, 0xa9015ff8],
            ),
            (
                0x138b6c0,
                [0xf9400408u32, 0xb4000168u32, 0xf9400108u32, 0x3974e909u32],
            ),
            (
                0x138c9b0,
                [0xf81d0ff5u32, 0xa9014ff4u32, 0xa9027bfdu32, 0x910083fdu32],
            ),
            (
                0x323f330,
                [0xd100c3ffu32, 0xf9000bf3u32, 0xa9027bfdu32, 0x910083fdu32],
            ),
            (
                0x13cdc00,
                [0xf81e0ff3u32, 0xa9017bfdu32, 0x910043fdu32, 0xf9402808u32],
            ),
            (
                0x13d0360,
                [0xf81e0ff3u32, 0xa9017bfdu32, 0x910043fdu32, 0xf9402808u32],
            ),
            (
                0x3238a70,
                [0xd106c3ffu32, 0xa91667fcu32, 0xa9175ff8u32, 0xa91857f6u32],
            ),
            (
                0x323c4e0,
                [0xd10243ffu32, 0xf9001bfbu32, 0xa90467fau32, 0xa9055ff8u32],
            ),
            (
                0x3778b80,
                [0xd100c3ffu32, 0xa9014ff4u32, 0xa9027bfdu32, 0x910083fdu32],
            ),
        ].iter().all(|&(offset, words)| {
            let stock = std::slice::from_raw_parts((base + offset) as *const u32, 4) == words;
            if !stock {
                trace!(
                    "[CharacterRoster] fingerprint mismatch +0x{:x}; disabled",
                    offset
                );
            }
            stock
        })
        // Constructor's position/facing loads used by native_reload_pose. These
        // sites are read-only and are not lifecycle-hook patch locations.
        && [(0x656f54, 0xf9407e88u32), (0x656f58, 0x3dc27d00u32), (0x656f70, 0xb949e109u32)].iter().all(|&(offset, word)| {
            let stock = crate::engine::seams::native_fields::read_unchecked::<u32>(base, offset) == word;
            if !stock {
                trace!("[CharacterRoster] spawn-pose fingerprint mismatch +0x{:x}; disabled", offset);
            }
            stock
        });
        if !crate::runtime::seams::seam_install::gate("character_roster", ROSTER_SEAMS, legacy) {
            if legacy {
                trace!("[CharacterRoster] seam guard refused; disabled");
            }
            return;
        }
        BASE.store(base, Ordering::Release);
    #[cfg(feature="mii-saved-alpha")]
    mii_saved_alpha::init(base);
    #[cfg(feature="mii-managed-helper-alpha")]
    mii_managed_helper::init_storage();
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    mii_replay::init();
        // Validate the original destructor range before capacity installs its
        // listener-removed hook at 0x652214 inside that range. The Mii gate's
        // own patch at 0x6510f4 is outside all capacity fingerprint ranges.
        #[cfg(feature="mii-ordinary-player-alpha")]
        if !mii_ordinary_player::init(base) {return;}
        #[cfg(feature = "managed-third-source-alpha")]
        native_source_capacity::init(base, load_contract);
        #[cfg(feature="mii-ldn-qualification-probe")]
        mii_ldn_qualification::init(base);
        #[cfg(feature="mii-ordinary-cpu-alpha")]
        mii_ordinary_cpu::init();
        native_service::install(base);
        #[cfg(feature = "native-body-bank-alpha")]
        body_bank::install(base);
        #[cfg(feature="mii-hud-portrait-alpha")]
        mii_hud_portrait::init(base);
        if cfg!(feature = "waiting-roster-reload") {
            skyline::install_hooks!(load_local_waiting_icons, load_online_waiting_icons);
        }
        trace!("[CharacterRoster] native async loader installed; ordinary P1/CPU1; waiting_enabled={}; one existing entry per role; no manual loader pumping", cfg!(feature = "waiting-roster-reload"));
    }
}
