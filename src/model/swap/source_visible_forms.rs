//! Value-only qualification of one visible native alternate-form input route.
//! The native caller reacquires every member; these addresses are equality
//! evidence and are never dereferenced here. Pending-load epoch ownership stays
//! with the existing SourceToken/Preflight/INSTALL contract.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Member {
    pub(crate) actor: usize,
    pub(crate) control: usize,
    pub(crate) kind: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Bank {
    pub(crate) entry: usize,
    pub(crate) owner: u32,
    pub(crate) resource: (usize, u32, u32),
    pub(crate) kinds: [u32; 3],
    pub(crate) bodies: [(usize, u32); 3],
    pub(crate) members: [Option<Member>; 3],
    pub(crate) active: usize,
    pub(crate) record: usize,
    // Actual active control's input_index, binding, alias_index.
    pub(crate) route: (i32, usize, i32),
}

pub(crate) type Slot = (usize, u8, i32, i32, i32);

fn binding(bank: Bank) -> Option<(usize, i32)> {
    // Only Aegis and Trainer are alternate forms. IC/SoloPopo and ordinary
    // copy/character changes cannot obtain permission through this predicate.
    let family: &[u32] = match bank.kinds[0] {
        0x5a | 0x5b => &[0x5a, 0x5b],
        0x24..=0x26 => &[0x24, 0x25, 0x26],
        _ => return None,
    };
    let count = family.len();
    if !matches!(bank.owner, 0 | 1 | 4)
        || bank.entry == 0
        || bank.resource.0 == 0
        || bank.resource.1 != bank.kinds[0]
        || bank.record == 0
        || bank.route != (bank.owner as i32, bank.record, bank.owner as i32)
        || bank.active >= count
        || bank.kinds[..count].iter().any(|kind| !family.contains(kind))
        || bank.kinds[count..].iter().any(|kind| *kind != u32::MAX)
    {
        return None;
    }
    for index in 0..3 {
        if index >= count {
            if bank.members[index].is_some() || bank.bodies[index] != (0, 0) {
                return None;
            }
            continue;
        }
        let member = bank.members[index]?;
        if member.actor == 0 || member.control == 0
            || bank.bodies[index].0 == 0 || !family.contains(&member.kind)
            || bank.kinds[..index].contains(&bank.kinds[index])
            || bank.bodies[..index].iter().any(|body| body.0 == bank.bodies[index].0)
            || bank.members[..index].iter().flatten().any(|previous| {
                previous.actor == member.actor || previous.control == member.control
                    || previous.kind == member.kind
            })
        {
            return None;
        }
    }
    let active = bank.members[bank.active]?;
    Some((active.control, active.kind as i32))
}

/// Derive an expected row from independent native member authority. Preserve
/// occupancy and owner/alias fields exactly; never copy current input as truth.
pub(crate) fn expected_slot(previous: Slot, before: Bank, current: Bank) -> Option<Slot> {
    let old_binding = binding(before)?;
    let new_binding = binding(current)?;
    let mut stable = current;
    stable.active = before.active;
    if stable != before || (previous.0, previous.2) != old_binding {
        return None;
    }
    Some((new_binding.0, previous.1, new_binding.1, previous.3, previous.4))
}

/// Exact merge used by the native caller. Optional authority never changes the
/// unchanged strict path. Changed rows need same-owner native bank evidence;
/// state/owner/alias and all other rows remain the strict expected values.
pub(crate) fn merge_slots(
    mut expected: [Slot; 16], observed: [Slot; 16], owners: [u32; 2],
    before: [Option<Bank>; 2], current: [Option<Bank>; 2],
) -> Option<[Slot; 16]> {
    if expected == observed { return Some(expected); }
    if !matches!(owners, [0, 1] | [0, 4]) { return None; }
    for (index, owner) in owners.iter().copied().enumerate() {
        let slot = owner as usize;
        if expected[slot] == observed[slot] { continue; }
        let previous = before[index]?;
        let active = current[index]?;
        if previous.owner != owner || active.owner != owner { return None; }
        expected[slot] = expected_slot(expected[slot], previous, active)?;
    }
    (expected == observed).then_some(expected)
}

