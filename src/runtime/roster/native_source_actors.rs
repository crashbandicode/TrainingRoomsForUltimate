//! Canonical parked created-entry (7, or 6/5 extended) actor/control contract
//! plus guarded visible form input comparison. The entry's routes are its own
//! records `Inputs.slot`/`+8`. Current native members authorize reads; retained
//! sets are numeric comparison evidence only. This module never mutates native state.
use super::{aligned, inputs_for, native_source, read, Inputs, Snapshot, BASE};
use skyline::hooks::InlineCtx;
use std::sync::atomic::Ordering;

use crate::engine::swap::source_visible_forms as visible;
pub(crate) use visible::Bank as VisibleBank;

pub(crate) fn supported(kind: u32) -> bool {
    if crate::engine::swap::trio::TrioFighter::from_kind(kind).is_none() {
        return false;
    }
    let forms = super::super::Forms::starting(kind);
    (forms.count == 1 && forms.actor_count == 1 && forms.flags == 0)
        || (forms.count == 2 && forms.actor_count == 2 && forms.flags == 2)
        || ((2..=3).contains(&forms.count) && forms.actor_count == 1 && forms.flags & 4 != 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Actor {
    pub(crate) actor: usize,
    control: usize,
    partner: usize,
    owner: u32,
    cleanup_owner: u32,
    pub(crate) kind: u32,
    flags: [u8; 3],
    control_actor: usize,
    input_index: i32,
    binding: usize,
    alias_index: i32,
    partner_control: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Actors {
    pointers: [usize; 5],
    pub(crate) actors: [Option<Actor>; 3],
}

#[derive(Clone, Copy)]
struct ActorPlan {
    first: usize,
    count: usize,
    kinds: [u32; 3],
    partners: bool,
    /// Pair-lead bank window: cells 0/1 are the Popo/Nana partner pair (Nana's
    /// context links to Popo's and routes on `owner+8`), cell 2 an ordinary kind.
    pair_lead: bool,
}

unsafe fn actor_plan(kind: u32) -> Option<ActorPlan> {
    let forms = super::super::Forms::starting(kind);
    if !supported(kind) {
        return None;
    }
    if forms.count == 1 {
        return Some(ActorPlan {
            first: 0,
            count: 1,
            kinds: forms.kinds,
            partners: false,
            pair_lead: false,
        });
    }
    if forms.flags & 4 == 0 {
        return (forms.count == 2 && forms.actor_count == 2).then_some(ActorPlan {
            first: 0,
            count: 2,
            kinds: forms.kinds,
            partners: true,
            pair_lead: false,
        });
    }
    if forms.actor_count != 1 || !(2..=3).contains(&forms.count) {
        return None;
    }
    // Same table selection as native655450/6516b0. The two immutable
    // tables are fingerprinted before hooks; descriptor order may differ.
    let offset = if kind & !1 == 0x5a {
        0x453c158
    } else {
        0x453c164
    };
    let table = (BASE.load(Ordering::Acquire) + offset) as *const u8;
    let kinds = [0usize, 1, 2].map(|index| read::<u32>(table, index * 4));
    let count = forms.count as usize;
    if kinds[..count].iter().any(|kind| !forms.same_group(*kind))
        || kinds[..count]
            .iter()
            .enumerate()
            .any(|(index, kind)| kinds[..index].contains(kind))
        || kinds[count..].iter().any(|kind| *kind != u32::MAX)
    {
        return None;
    }
    Some(ActorPlan {
        first: 2,
        count,
        kinds,
        partners: false,
        pair_lead: false,
    })
}

/// Physical actors of a currently owned visible bank. The shared roster view
/// intentionally returns the live fighter's canonical Forms; capacity evidence
/// separately preserves every physical body/context, including parked members.
/// A pair-lead bank reports its three group contexts with the pair's partner links.
unsafe fn visible_plan(entry: *const u8, owner: u32, forms: super::super::Forms) -> Option<ActorPlan> {
    if let Some(members) = super::super::body_bank::ready_members(entry, owner) {
        return Some(ActorPlan { first: 2, count: members.count(), kinds: members.kinds(), partners: false,
            pair_lead: members.is_pair_lead() });
    }
    if forms.flags & 4 != 0 { return actor_plan(forms.kinds[0]); }
    if (forms.count == 2 && forms.actor_count == 2 && forms.flags == 2)
        || (forms.count == 1 && forms.actor_count == 1 && forms.flags == 0) {
        return Some(ActorPlan { first: 0, count: forms.count as usize, kinds: forms.kinds,
            partners: forms.count == 2, pair_lead: false });
    }
    None
}

unsafe fn actor_record(actor: *const u8) -> Option<Actor> {
    if !aligned(actor) || read::<usize>(actor, 0) != BASE.load(Ordering::Acquire) + 0x4f85d30 {
        return None;
    }
    let control = read::<*const u8>(actor, 0x58);
    if !aligned(control) {
        return None;
    }
    Some(Actor {
        actor: actor as usize,
        control: control as usize,
        partner: read(actor, 0x60),
        owner: read(actor, 8),
        cleanup_owner: read(actor, 0x6c),
        kind: read(actor, 0x74),
        flags: [read(actor, 0x68), read(actor, 0x69), read(actor, 0x70)],
        control_actor: read(control, 8),
        input_index: read(control, 0x160),
        binding: read(control, 0x168),
        alias_index: read(control, 0xc50),
        partner_control: read(control, 0x980),
    })
}

unsafe fn group_descriptor(entry: *const u8, kind: u32) -> bool {
    let forms = super::super::Forms::starting(kind);
    actor_plan(kind).is_some()
        && read::<u32>(entry, 0x14) == forms.count
        && read::<u32>(entry, 0x28) == forms.actor_count
        && [0usize, 1, 2].map(|i| read::<u32>(entry, 0x18 + i * 4)) == forms.kinds
        && read::<u8>(entry, 0x591f) == forms.flags
}

unsafe fn group_actor_observation(entry: *const u8, slot: usize, kind: u32) -> Option<Actors> {
    if native_source::entry(slot as u32) != Some(entry) || !group_descriptor(entry, kind) {
        return None;
    }
    let plan = actor_plan(kind)?;
    let pointers: [usize; 5] =
        [0x4178, 0x4180, 0x4188, 0x4190, 0x4198].map(|offset| read(entry, offset));
    let actors = std::array::from_fn(|i| {
        if i < plan.count {
            actor_record(pointers[plan.first + i] as *const u8)
        } else {
            None
        }
    });
    if native_source::entry(slot as u32) != Some(entry)
        || !group_descriptor(entry, kind)
        || [0x4178, 0x4180, 0x4188, 0x4190, 0x4198].map(|offset| read::<usize>(entry, offset))
            != pointers
    {
        return None;
    }
    Some(Actors { pointers, actors })
}

/// `input` is the created entry's own view: owner and routes are `input.slot`/`+8`.
fn actor_fields_valid(actor: Actor, index: usize, input: Inputs, plan: ActorPlan) -> bool {
    let route = input.slot + if plan.partners { index * 8 } else { 0 };
    actor.owner == input.slot as u32
        && actor.cleanup_owner == input.slot as u32
        && actor.kind == plan.kinds[index]
        && actor.flags == [0, 1, 0]
        && actor.control_actor == actor.actor
        && actor.input_index == route as i32
        && actor.alias_index == route as i32
        && actor.binding == input.records + route * 0x280
}

// Native658af0 runs 655320 and then 64f820. For an alternate bank, 64f820
// tail-calls 64fb70, which aliases the current form's member at +4178 and
// calls 313b00(service, route, entry+34, null, -1). That stores only the
// entry's own route state byte; each member destructor's 313e90(route)
// restores state 2. Witnessed for route 7; 6/5 need runtime confirmation.
unsafe fn activated_member(entry: *const u8, kind: u32, plan: ActorPlan) -> Option<usize> {
    let form = read::<u8>(entry, 0x5918) as usize;
    if plan.first != 2
        || form >= super::super::Forms::starting(kind).count as usize
        || !group_descriptor(entry, kind)
    {
        return None;
    }
    let kind = read::<u32>(entry, 0x18 + form * 4);
    let index = plan.kinds[..plan.count]
        .iter()
        .position(|member| *member == kind)?;
    let member = read::<usize>(entry, 0x4178 + (plan.first + index) * 8);
    (member != 0).then_some(member)
}

pub(crate) unsafe fn ready(entry: *const u8, slot: usize, kind: u32) -> Option<Actors> {
    let input = inputs_for(slot).ok()?;
    let observed = group_actor_observation(entry, slot, kind)?;
    let plan = actor_plan(kind)?;
    let information = read::<*const u8>(entry, 0xf8);
    for (index, record) in observed.actors.iter().copied().enumerate().take(plan.count) {
        let record = record?;
        if !actor_fields_valid(record, index, input, plan)
            || observed.actors[..index].iter().flatten().any(|previous| {
                previous.actor == record.actor || previous.control == record.control
            })
        {
            return None;
        }
        if plan.partners {
            let first = observed.actors[0]?;
            let second = observed.actors[1]?;
            if first.partner != 0
                || second.partner != first.actor
                || first.partner_control != second.control
                || second.partner_control != 0
            {
                return None;
            }
        } else if record.partner != 0 || record.partner_control != 0 {
            return None;
        }
    }
    // Construction leaves the non-owning slots zero; an alternate bank's 64fb70
    // then aliases exactly the current form's member at +4178. Never follow it.
    let alias = if plan.first == 2 {
        activated_member(entry, kind, plan)?
    } else {
        0
    };
    let non_owning = [alias, 0, 0, 0, 0];
    if observed
        .pointers
        .iter()
        .zip(non_owning)
        .enumerate()
        .any(|(index, (pointer, expected))| {
            !(plan.first..plan.first + plan.count).contains(&index) && *pointer != expected
        })
        || !aligned(information)
        || read::<u8>(information, 0x92) != 0
        || input.cpu_flag != 1
        || !input.unclaimed()
        || read::<u8>(entry, 0x34) != 0
        || input.slots[slot].state != if plan.first == 2 { 0 } else { 2 }
        || input.slots[slot + 8].state != 2
        || inputs_for(slot) != Ok(input)
    {
        return None;
    }
    Some(observed)
}

impl Actors {
    pub(crate) fn contains(self, actor: usize) -> bool {
        self.actors
            .iter()
            .flatten()
            .any(|record| record.actor == actor)
    }
    pub(crate) fn count(self) -> u32 {
        self.actors.iter().flatten().count() as u32
    }
    pub(crate) fn unretired(self, actor: usize, seen: u8) -> bool {
        self.actors.iter().enumerate().any(|(index, record)| {
            seen & (1 << index) == 0 && record.is_some_and(|record| record.actor == actor)
        })
    }
}

/// Before a new install, the old complete bank's route is native-active (0),
/// otherwise native-unclaimed (2). Release313e90 resets it to2; 64fb70/313b00
/// sets only state7 to0 for the new alternate bank. This is the strict base
/// expectation; loaded_visible_input additionally qualifies visible form routes.
pub(crate) fn loaded_input(before: Inputs, previous: Option<u32>, kind: u32) -> Option<Inputs> {
    if !supported(kind) || !preflight_input(before, previous) {
        return None;
    }
    let mut after = before;
    after.slots[before.slot].state = if super::super::Forms::starting(kind).flags & 4 != 0 {
        0
    } else {
        2
    };
    Some(after)
}

// Current native members authorize every pointer read. A saved bank is used
// only as a numeric identity comparison after reacquiring the same live FE.
unsafe fn visible_bank(scene: Snapshot, input: Inputs, index: usize) -> Option<VisibleBank> {
    visible_bank_checked(scene, input, index).ok()
}

/// Every `visible_bank` check except the active input row, with that row (lobby-preload B splits
/// it out so extra-capacity creation can apply the committed body-bank rule to it). The reads and
/// their order are unchanged.
type BankRow = (VisibleBank, visible::Slot);
unsafe fn visible_bank_parts(scene: Snapshot, input: Inputs, index: usize) -> Result<BankRow, &'static str> {
    if index >= 2 { return Err("bank: not a visible owner"); }
    let owner = scene.layout.owners()[index];
    let entry = native_source::entry(owner).ok_or("bank: entry absent")?;
    let original = scene.original[index];
    if entry as usize != scene.primary[owner as usize]
        || super::entry_observation(entry, owner) != Some(original)
    { return Err("bank: entry or observation changed"); }
    let (forms, active_kind) = super::super::ready_forms(entry, owner).ok_or("bank: forms not ready")?;
    let plan = visible_plan(entry, owner, forms).ok_or("bank: no actor plan")?;
    if plan.first != 2 { return Err("bank: not an alternate-bank plan"); }
    let resource = native_source::resource_owner(owner).ok_or("bank: resource owner absent")?;
    if resource as usize != scene.resources[owner as usize]
        || resource as usize != scene.aliases[owner as usize]
        || read::<usize>(resource, 0) != BASE.load(Ordering::Acquire) + 0x509fb10
    { return Err("bank: resource root, alias or vtable"); }
    let pointers: [usize; 5] =
        [0x4178, 0x4180, 0x4188, 0x4190, 0x4198].map(|offset| read(entry, offset));
    let mut members = [None; 3];
    let mut records = [None; 3];
    let mut active = None;
    let mut active_actor = None;
    let mut route = None;
    for (index, member) in members.iter_mut().enumerate().take(plan.count) {
        let actor = actor_record(pointers[plan.first + index] as *const u8).ok_or("bank: member actor unreadable")?;
        // The pair-lead Popo/Nana contexts keep their creation-time partner link
        // (checked as a pair below); every other member has none.
        let partnered = plan.pair_lead && index < 2;
        if actor.owner != owner || actor.cleanup_owner != owner
            || actor.kind != plan.kinds[index] || actor.control_actor != actor.actor
            || (!partnered && (actor.partner != 0 || actor.partner_control != 0))
        { return Err("bank: member actor owner, kind, control or partner"); }
        records[index] = Some(actor);
        *member = Some(visible::Member {
            actor: actor.actor, control: actor.control, kind: actor.kind,
        });
        if actor.kind == active_kind {
            if pointers[0] != actor.actor { return Err("bank: active member is not the first actor"); }
            active = Some(index);
            active_actor = Some(actor);
            route = Some((actor.input_index, actor.binding, actor.alias_index));
        }
    }
    if plan.pair_lead {
        let (popo, nana) = (records[0].ok_or("bank: pair member absent")?, records[1].ok_or("bank: pair member absent")?);
        if popo.partner != 0 || nana.partner != popo.actor
            || popo.partner_control != nana.control || nana.partner_control != 0
        { return Err("bank: pair partner link"); }
    }
    // Second alias: Nana's own group context while the pair is active (N2), zero otherwise.
    let alias_ok = if plan.pair_lead && active == Some(0) {
        pointers[1] != 0 && pointers[1] == pointers[3]
    } else {
        pointers[1] == 0
    };
    if !alias_ok || (plan.count == 2 && pointers[4] != 0) { return Err("bank: alias or spare actor pointer"); }
    let bank = VisibleBank {
        entry: entry as usize, owner,
        resource: (resource as usize, read(resource, 0x58), read(resource, 0x64)),
        kinds: plan.kinds, bodies: original.bodies, members,
        active: active.ok_or("bank: no active member")?, record: input.records + owner as usize * 0x280,
        route: route.ok_or("bank: no active route")?,
    };
    // Recheck native membership and active control after the bounded reads.
    if native_source::entry(owner) != Some(entry)
        || super::entry_observation(entry, owner) != Some(original)
        || super::super::ready_forms(entry, owner) != Some((forms, active_kind))
        || native_source::resource_owner(owner) != Some(resource)
    { return Err("bank: membership changed during read"); }
    let current_pointers =
        [0x4178, 0x4180, 0x4188, 0x4190, 0x4198].map(|offset| read::<usize>(entry, offset));
    if current_pointers != pointers
        || actor_record(current_pointers[plan.first + bank.active] as *const u8) != active_actor
        || (read(resource, 0x58), read(resource, 0x64)) != (bank.resource.1, bank.resource.2)
        || inputs_for(input.slot) != Ok(input)
    { return Err("bank: actors, resource words or input changed during read"); }
    let slot = input.slots[owner as usize];
    Ok((bank, (slot.pointer, slot.state, slot.key, slot.owner, slot.alias)))
}

/// `visible_bank` naming its first failed check (lobby-preload D: the creation-waiting line).
/// Even unchanged preflight input must bind the active native member: the native alternate-form
/// rule (`visible::expected_slot`) is the only row rule for loads.
unsafe fn visible_bank_checked(scene: Snapshot, input: Inputs, index: usize) -> Result<VisibleBank, &'static str> {
    let (bank, row) = visible_bank_parts(scene, input, index)?;
    if visible::expected_slot(row, bank, bank) == Some(row) { Ok(bank) } else { Err("bank: active input row") }
}

/// lobby-preload B, extra-capacity creation only: a visible bank window's active input row is
/// admitted by the native alternate-form rule (Aegis/Trainer) or, for a body bank, by the bank's
/// own committed window (`body_bank::ready_members`, re-read now under the bank lock: Ready,
/// committed descriptor and bodies, active cell), whose active member's control and kind the row
/// must name (`capacity_creation::creation_row`). Every other bank check is `visible_bank`'s.
/// `true`: admitted by the committed body-bank rule.
unsafe fn creation_bank(scene: Snapshot, input: Inputs, index: usize) -> Result<(VisibleBank, bool), &'static str> {
    let (bank, row) = visible_bank_parts(scene, input, index)?;
    let alternate = visible::expected_slot(row, bank, bank);
    let window = if alternate == Some(row) { None } else { Some(committed_window(bank)) };
    let body_bank = crate::engine::swap::capacity_creation::creation_row(row, alternate, window)?;
    Ok((bank, body_bank))
}

/// lobby-preload B: the captured window as values, with the body bank's committed member kinds
/// re-read now (None when no Ready body bank owns this exact window).
unsafe fn committed_window(bank: VisibleBank) -> crate::engine::swap::capacity_creation::BankWindow {
    let committed = super::super::body_bank::ready_members(bank.entry as *const u8, bank.owner)
        .map(|members| members.kinds());
    crate::engine::swap::capacity_creation::BankWindow {
        owner: bank.owner,
        entry: bank.entry,
        resource: (bank.resource.0, bank.resource.1),
        kinds: bank.kinds,
        bodies: bank.bodies.map(|body| body.0),
        members: bank.members.map(|member| member.map(|m| (m.actor, m.control, m.kind))),
        active: bank.active,
        record: bank.record,
        route: bank.route,
        committed,
    }
}

pub(crate) unsafe fn visible_preflight(scene: Snapshot, input: Inputs) -> [Option<VisibleBank>; 2] {
    let mut banks = [None; 2];
    for (index, bank) in banks.iter_mut().enumerate() {
        if scene.original[index].count > 1 {
            *bank = visible_bank(scene, input, index);
        }
    }
    banks
}

// Extra evidence for initial native reservation only. No saved pointer is
// dereferenced: every member comes from the currently resolved original entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CreationVisible {
    descriptor: [u32; 28],
    resource: (usize, u32, u32),
    forms: super::super::Forms,
    active_kind: u32,
    form: [u8; 5],
    bodies: [(usize, u32, u32, u64, u8, usize); 3],
    actors: Actors,
    alternate: Option<VisibleBank>,
    /// lobby-preload B: the window was admitted by the committed body-bank row rule.
    body_bank: bool,
}

impl CreationVisible {
    /// lobby-preload B: admitted through the committed body-bank rule (`visible_bank_kept` watches it).
    pub(crate) fn body_bank(&self) -> bool { self.body_bank }
}

/// The creation capture of one visible owner, naming its first failed check (lobby-preload D:
/// the creation-waiting line). The reads, their order and the captured value are unchanged.
unsafe fn creation_visible_one(scene: Snapshot, input: Inputs, index: usize) -> Result<CreationVisible, &'static str> {
    let owner = scene.layout.owners()[index];
    let entry = native_source::entry(owner).ok_or("visible: entry absent")?;
    let original = scene.original[index];
    if entry as usize != scene.primary[owner as usize]
        || super::entry_observation(entry, owner) != Some(original)
    { return Err("visible: entry or observation changed"); }
    let (forms, active_kind) = super::super::ready_forms(entry, owner).ok_or("visible: forms not ready")?;
    let plan = visible_plan(entry, owner, forms).ok_or("visible: no actor plan")?;
    let resource = native_source::resource_owner(owner).ok_or("visible: resource owner absent")?;
    if resource as usize != scene.resources[owner as usize]
        || resource as usize != scene.aliases[owner as usize]
        || read::<usize>(resource, 0) != BASE.load(Ordering::Acquire) + 0x509fb10
        || read::<u32>(resource, 0x58) != original.kinds[0]
    { return Err("visible: resource root, alias, vtable or kind word"); }
    let resource_fields = (resource as usize, read(resource, 0x58), read(resource, 0x64));
    let (alternate, body_bank) = if plan.first == 2 {
        let (bank, body_bank) = creation_bank(scene, input, index)?;
        (Some(bank), body_bank)
    } else { (None, false) };
    let descriptor = std::array::from_fn(|i| read(entry, i * 4));
    let form = [0x5918, 0x5919, 0x591a, 0x591f, 0x5934].map(|offset| read(entry, offset));
    let pointers: [usize; 5] = [0x4178, 0x4180, 0x4188, 0x4190, 0x4198]
        .map(|offset| read(entry, offset));
    let mut actors = Actors { pointers, actors: [None; 3] };
    for i in 0..plan.count {
        let actor = actor_record(pointers[plan.first + i] as *const u8).ok_or("visible: actor unreadable")?;
        if actor.owner != owner || actor.cleanup_owner != owner
            || actor.kind != plan.kinds[i] || actor.control_actor != actor.actor
            || actors.actors[..i].iter().flatten().any(|old|
                old.actor == actor.actor || old.control == actor.control)
        { return Err("visible: actor owner, kind, control or duplicate"); }
        if alternate.is_some() {
            // Every inactive bank member must also retain its own visible
            // owner's route. An active-row match alone cannot reserve7 safely.
            if actor.partner != 0 || actor.partner_control != 0
                || actor.input_index != owner as i32 || actor.alias_index != owner as i32
                || actor.binding != input.records + owner as usize * 0x280
            { return Err("visible: bank member route"); }
        } else {
            let route = owner as usize + i * 8;
            let expected = if owner == 0 && i == 0 { (0, -1, -1) }
                else { (1, route as i32, route as i32) };
            let slot = input.slots[route];
            if actor.input_index != route as i32 || actor.alias_index != route as i32
                || actor.binding != input.records + route * 0x280
                || (slot.pointer, slot.key) != (actor.control, actor.kind as i32)
                || (slot.state, slot.owner, slot.alias) != expected
            { return Err("visible: actor input route row"); }
        }
        actors.actors[i] = Some(actor);
    }
    if alternate.is_none() {
        if pointers[plan.count..].iter().any(|pointer| *pointer != 0) { return Err("visible: spare actor pointer"); }
        let first = actors.actors[0].ok_or("visible: first actor absent")?;
        if plan.partners {
            let second = actors.actors[1].ok_or("visible: partner actor absent")?;
            if first.partner != 0 || second.partner != first.actor
                || first.partner_control != second.control || second.partner_control != 0
            { return Err("visible: partner link"); }
        } else if first.partner != 0 || first.partner_control != 0 { return Err("visible: unexpected partner"); }
    }
    let mut bodies = [(0, 0, 0, 0, 0, 0); 3];
    for (i, output) in bodies.iter_mut().enumerate().take(plan.count) {
        // ready_forms just validated each current body/vtable/owner/kind/state.
        let body = read::<*const u8>(entry, 0x4160 + i * 8);
        if !aligned(body) || read::<usize>(body, 0) != BASE.load(Ordering::Acquire) + 0x4f80210 {
            return Err("visible: body pointer or vtable");
        }
        *output = (body as usize, read(body, 8), read(body, 0xc), read(body, 0x10),
            read(body, 0x3a), read(body, 0x20));
    }
    // Reacquire roots and recheck membership before rereading any member.
    if native_source::entry(owner) != Some(entry)
        || super::entry_observation(entry, owner) != Some(original)
        || super::super::ready_forms(entry, owner) != Some((forms, active_kind))
        || native_source::resource_owner(owner) != Some(resource)
        || (read(resource, 0x58), read(resource, 0x64)) != (resource_fields.1, resource_fields.2)
        || std::array::from_fn::<_, 28, _>(|i| read::<u32>(entry, i * 4)) != descriptor
        || [0x5918, 0x5919, 0x591a, 0x591f, 0x5934].map(|offset| read::<u8>(entry, offset)) != form
        || [0x4178, 0x4180, 0x4188, 0x4190, 0x4198].map(|offset| read::<usize>(entry, offset)) != pointers
        || inputs_for(input.slot) != Ok(input)
    { return Err("visible: roots, descriptor, form, pointers or input changed during read"); }
    for i in 0..plan.count {
        if actor_record(pointers[plan.first + i] as *const u8) != actors.actors[i] {
            return Err("visible: actor changed during read");
        }
    }
    for (i, expected) in bodies.iter().enumerate().take(plan.count) {
        let body = read::<*const u8>(entry, 0x4160 + i * 8);
        if !aligned(body) || body as usize != expected.0
            || read::<usize>(body, 0) != BASE.load(Ordering::Acquire) + 0x4f80210
            || (body as usize, read(body, 8), read(body, 0xc), read(body, 0x10),
                read(body, 0x3a), read(body, 0x20)) != *expected
        { return Err("visible: body changed during read"); }
    }
    Ok(CreationVisible { descriptor, resource: resource_fields, forms, active_kind,
        form, bodies, actors, alternate, body_bank })
}

pub(crate) unsafe fn creation_visible(scene: Snapshot, input: Inputs) -> Option<[Option<CreationVisible>; 2]> {
    creation_visible_checked(scene, input).ok()
}

/// The visible owner (owners-array index) and the check its creation capture failed.
pub(crate) type VisibleFailure = (u8, &'static str);

/// `creation_visible` naming the visible owner (owners-array index) and the check that failed
/// (lobby-preload D: the creation-waiting line). Same owners, order and captures.
pub(crate) unsafe fn creation_visible_checked(scene: Snapshot, input: Inputs)
    -> Result<[Option<CreationVisible>; 2], VisibleFailure> {
    let mut originals = [None; 2];
    for (index, output) in originals.iter_mut().enumerate() {
        if scene.original[index].count > 1 || !native_source::ordinary_kind(scene.original[index].kinds[0]) {
            *output = Some(creation_visible_one(scene, input, index).map_err(|check| (index as u8, check))?);
        }
    }
    Ok(originals)
}

pub(crate) unsafe fn loaded_visible_input(
    scene: Snapshot, before: Inputs, previous: Option<u32>, kind: u32,
    banks: [Option<VisibleBank>; 2], observed: Inputs,
) -> Option<Inputs> {
    let mut expected = loaded_input(before, previous, kind)?;
    if expected == observed { return Some(expected); }
    let mut current = [None; 2];
    let owners = [scene.layout.owners()[0], scene.layout.owners()[1]];
    for (index, bank) in banks.iter().copied().enumerate() {
        let owner = owners[index] as usize;
        if expected.slots[owner] == observed.slots[owner] { continue; }
        if bank.is_some() { current[index] = visible_bank(scene, observed, index); }
    }
    let rows = |input: Inputs| input.slots.map(|s| (s.pointer, s.state, s.key, s.owner, s.alias));
    let merged = visible::merge_slots(rows(expected), rows(observed), owners, banks, current)?;
    for (slot, row) in expected.slots.iter_mut().zip(merged) {
        // Only the independently authorized pointer/key may differ.
        slot.pointer = row.0;
        slot.key = row.2;
    }
    (observed == expected && inputs_for(observed.slot) == Ok(observed)).then_some(expected)
}

pub(crate) fn preflight_input(before: Inputs, previous: Option<u32>) -> bool {
    if previous.is_some_and(|kind| !supported(kind)) {
        return false;
    }
    let alternate = |kind| super::super::Forms::starting(kind).flags & 4 != 0;
    let expected_state = if previous.is_some_and(alternate) {
        0
    } else {
        2
    };
    if before.slot >= 8 {
        return false;
    }
    let row = before.slots[before.slot];
    before.unclaimed()
        && before.cpu_flag == 1
        && (row.pointer, row.state, row.key, row.owner, row.alias)
            == (0, expected_state, -1, -1, -1)
        && before.slots[before.slot + 8].state == 2
}

/// The load's current scene for the comparison with its commit-time scene `prior` (`loaded`, `stalled_checks`).
/// A waiting room's snapshot also carries an input member (`Snapshot::ldn.input`): the legacy entry-7 view the
/// probes read, whatever entry is loading. Its parts and what the load's validity takes from them:
/// - the 16 route rows and the shared service/data/records/owner-map/CPU-flag fields: the same in every entry's
///   view, and exactly what the load's own input contract (`expected`) proves, including the load's own route
///   change (an alternate bank sets its row's state 2 -> 0);
/// - the member entry's own mapping, controller, property flag and CSS role: native reads them only when it
///   builds that entry's controls (`0x6b6f10`), so they belong to loads on that entry.
///
/// The member is compared as its prior value when `source_load::waiting_member_proven` holds; any other change
/// stays visible (scene bit 20). FAIL 2 scene-proof follow-up (l1, l1b): the rule before this change forgave
/// the member only when it was the loading entry's own view, so every PT/Aegis load on created extras 3/2
/// failed after 900 callbacks.
pub(crate) fn comparison_view(
    current: Snapshot,
    prior: Snapshot,
    before: Inputs,
    expected: Option<Inputs>,
) -> Snapshot {
    {
        let mut current = current;
        let prior_member = prior.ldn.and_then(|fields| fields.input);
        if let Some(fields) = current.ldn.as_mut() {
            let proven = fields.input.zip(prior_member).zip(expected).is_some_and(|((member, prior), expected)| {
                crate::engine::swap::source_load::waiting_member_proven(
                    &input_view(prior), &input_view(member), &input_view(before), &input_view(expected))
            });
            if proven {
                fields.input = prior_member;
            }
        }
        current
    }
}

type ViewRow = (usize, u8, i32, i32, i32);
type ViewShared = (usize, usize, usize, usize, u8);
type ViewOwn = (i32, usize, usize, u8, u8);

/// One entry's input view as the scene proof's model sees it: rows, shared fields, the entry's own fields.
fn input_view(input: Inputs) -> crate::engine::swap::source_load::InputView<ViewRow, ViewShared, ViewOwn> {
    crate::engine::swap::source_load::InputView {
        slot: input.slot,
        rows: input.slots.map(|s| (s.pointer, s.state, s.key, s.owner, s.alias)),
        shared: (input.service, input.data, input.records, input.mapping, input.cpu_flag),
        own: (input.map7, input.controller, input.controller_property, input.controller_flag, input.css_role),
    }
}

/// The release loop clears a member field only after this destructor returns.
/// Plan comes from its source generation, not a newly published target kind.
pub(crate) unsafe fn cleanup(
    ctx: &InlineCtx,
    entry: *const u8,
    slot: usize,
    kind: u32,
    expected: Option<Actors>,
    first: Option<Actor>,
) -> Option<(usize, Option<Actor>, bool)> {
    let plan = actor_plan(kind)?;
    let actor = ctx.registers[20].x() as usize;
    let offsets = [0x4178, 0x4180, 0x4188, 0x4190, 0x4198];
    let index = (0..plan.count)
        .find(|index| actor != 0 && read::<usize>(entry, offsets[plan.first + index]) == actor)?;
    let record = actor_record(actor as *const u8);
    let input = inputs_for(slot);
    let identity = record.is_some_and(|record| {
        input.is_ok_and(|input| actor_fields_valid(record, index, input, plan))
            && expected.is_none_or(|set| set.actors[index] == Some(record))
            && if plan.partners {
                if index == 0 {
                    record.partner == 0
                } else {
                    expected
                        .and_then(|set| set.actors[0])
                        .or(first)
                        .is_some_and(|first| {
                            record.partner == first.actor && first.partner_control == record.control
                        })
                }
            } else {
                record.partner == 0 && record.partner_control == 0
            }
    });
    if native_source::entry(slot as u32) != Some(entry)
        || read::<usize>(entry, offsets[plan.first + index]) != actor
    {
        return None;
    }
    Some((index, record, identity))
}
