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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    // The two route values are the exact LDN05 B #531/#556 input failure.
    // Other bank fields are explicit native-reader contract fixtures, not a
    // claim that B logged each actor/control field (it did not).
    fn bank(kinds: [u32; 3], owner: u32) -> Bank {
        let count = if kinds[2] == u32::MAX { 2 } else { 3 };
        let mut result = Bank {
            entry: 0x12_0be0_4000, owner, resource: (0x11_faf7_5100, kinds[0], 0),
            kinds, bodies: [(0, 0); 3], members: [None; 3], active: 0,
            record: 0x11_f543_f010 + owner as usize * 0x280,
            route: (0, 0, 0),
        };
        result.route = (owner as i32, result.record, owner as i32);
        for (index, kind) in kinds.iter().copied().enumerate().take(count) {
            result.bodies[index] = (0x1000 + index * 0x100, index as u32);
            result.members[index] = Some(Member {
                actor: 0x2000 + index * 0x80, control: 0x3000 + index * 0x100, kind,
            });
        }
        result
    }

    fn row(bank: Bank) -> Slot {
        let member = bank.members[bank.active].unwrap();
        (member.control, if bank.owner == 0 { 0 } else { 1 }, member.kind as i32,
            if bank.owner == 0 { -1 } else { bank.owner as i32 },
            if bank.owner == 0 { -1 } else { bank.owner as i32 })
    }

    fn rows(bank: Bank) -> [Slot; 16] {
        let mut rows = [(0, 2, -1, -1, -1); 16];
        rows[bank.owner as usize] = row(bank);
        rows
    }

    fn captured_ldn05_aegis_delta_requires_same_native_bank() {
        let mut before = bank([0x5b, 0x5a, u32::MAX], 0);
        // Canonical actor order is Pyra/Mythra even when body order starts Mythra.
        before.members.swap(0, 1);
        before.active = 1;
        before.members[0].as_mut().unwrap().control = 0x11_dba6_0c00;
        before.members[1].as_mut().unwrap().control = 0x11_80ee_2e00;
        let previous = (0x11_80ee_2e00, 0, 0x5b, -1, -1);
        let observed = (0x11_dba6_0c00, 0, 0x5a, -1, -1);
        assert_ne!(previous, observed); // The unchanged strict comparator rejects B.
        let mut after = before; after.active = 0;
        assert_eq!(expected_slot(previous, before, after), Some(observed));
        assert_eq!(merge_slots(rows(before), rows(after), [0, 4],
            [Some(before), None], [Some(after), None]), Some(rows(after)));
        let mut wrong = after; wrong.entry += 8;
        assert_eq!(expected_slot(previous, before, wrong), None);
        wrong = after; wrong.members[0].as_mut().unwrap().control += 8;
        assert_eq!(expected_slot(previous, before, wrong), None);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("captured_ldn05_aegis_delta_requires_same_native_bank", captured_ldn05_aegis_delta_requires_same_native_bank as fn()),
        ]);
    }
    }
}
