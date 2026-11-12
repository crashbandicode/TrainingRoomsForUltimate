// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[allow(dead_code, unused_imports)]
#[path = "../src/model/mii/mod.rs"]
pub mod mii;
#[allow(unused_imports)]
mod engine {
    pub use super::mii;
}

// Copied CPU01 configuration/token/association evidence. Epoch7 is a TEST generation:
// native diagnostic epoch0 is not upgraded to a production scene epoch by this fixture.
use crate::engine::mii::inventory::{
    self, Appearance, Canonical, MiiClass, MiiConfig, TaggedId, CHARINFO_MASK_END,
};
use crate::engine::mii::lifetime::{Binding, Configuration, Observed, Operation, Phase, Tokens};
use crate::engine::mii::native_values::{CpuHelper, CpuOwner, Mirrors, RowIds, RECORD_LEN};
use crate::engine::mii::role_adapter::{Current, Error, SavedRole, Scope};
use crate::engine::mii::selection::{Action, Choice, Role, Source};

fn configuration(alternate: bool) -> Configuration {
    configuration_for_class(MiiClass::Swordfighter, alternate)
}

// Class-parametric MODEL values; Brawler/Gunner values are not claimed native captures.
fn configuration_for_class(class: MiiClass, alternate: bool) -> Configuration {
    let info = if alternate { ALT } else { ORIGINAL };
    let mut masked = info;
    masked[..CHARINFO_MASK_END].fill(0);
    let (outfit, colour, specials) = if alternate {
        (0x13002b147bc59435u64, 4, [1, 0, 0, 0])
    } else {
        (0x1300291cce3b03f5, 5, [0, 0, 0, 0])
    };
    let c = Canonical {
        class,
        appearance: Appearance {
            charinfo_masked: masked,
            specials,
            colour,
            voice: 37,
            outfit: outfit & 0xff_ffff_ffff,
            hat: 0x0fa9efa5ea,
            chara: class.chara_hash40(),
        },
    };
    // Deliberately different metadata: no literal saved ID is a native-write authority.
    Configuration::from_saved(&MiiConfig {
        slot_id: if alternate { 70002 } else { 70001 },
        slot_index: 0,
        name: String::new(),
        charinfo: info,
        raw_tags: [
            TaggedId(outfit),
            TaggedId(0x5a00000fa9efa5ea),
            TaggedId(0xc1004d0000000000 | class.chara_hash40()),
        ],
        canonical: c,
        copies: vec![],
    })
    .unwrap()
}
fn rows(c: Configuration) -> RowIds {
    let t = c.tokens();
    let a = c.canonical().appearance;
    RowIds {
        outfit: (0x13u64 << 56) | (u64::from(t.outfit) << 40) | a.outfit,
        hat: (0x5au64 << 56) | (u64::from(t.hat) << 40) | a.hat,
    }
}
fn mirrors(index: u16, t: Tokens) -> Mirrors {
    Mirrors {
        index,
        tokens: t,
        fe: [index, t.outfit, t.hat, t.colour, t.voice],
        d: [index, t.outfit, t.hat, t.colour],
        d_specials: t.specials.map(u32::from),
        owner_index: index,
        owner: [t.outfit, t.hat, t.colour, t.voice].map(u32::from),
    }
}
fn choice(c: Configuration) -> Choice {
    Choice {
        canonical: c.canonical(),
        slot_id: Some(70002),
        source: Source::Preference,
    }
}
fn cpu() -> Current {
    let c = configuration(false);
    let owners = [
        CpuOwner {
            owner: 0,
            kind: 9,
            selector: 0,
            index: 0,
            entry: 0x11f239a000,
            row: 0x86211730,
            information: 0x11f239a110,
            object: 0x11fbe5b700,
            alias: 0,
        },
        CpuOwner {
            owner: 1,
            kind: 73,
            selector: 1,
            index: 1,
            entry: 0x11f7220000,
            row: 0x86211990,
            information: 0x11f7220110,
            object: 0x11fc686e00,
            alias: 0,
        },
        CpuOwner {
            owner: 2,
            kind: 73,
            selector: 2,
            index: 2,
            entry: 0x11f7226000,
            row: 0x86211bf0,
            information: 0x11f7226110,
            object: 0x11fc687300,
            alias: 0,
        },
        CpuOwner {
            owner: 3,
            kind: 73,
            selector: 3,
            index: 3,
            entry: 0x11f722c000,
            row: 0x86211e50,
            information: 0x11f722c110,
            object: 0x11fc687800,
            alias: 0,
        },
    ];
    let helpers = core::array::from_fn(|i| {
        let mut record = [0; RECORD_LEN];
        record[..88].copy_from_slice(c.charinfo());
        let resources: [[u64; 2]; 2] = [[0x11ad4e7020, 0x11fd91be40], [0x11ad4e7050, 0x11fd91bf60]];
        record[88..96].copy_from_slice(&resources[i][0].to_le_bytes());
        record[96..104].copy_from_slice(&resources[i][1].to_le_bytes());
        CpuHelper {
            owner: owners[i + 2],
            mirrors: mirrors((i + 2) as u16, c.tokens()),
            record,
            body: [0x11f9cc9ed0, 0x11f9cd9810][i],
            body_id: ((i + 2) as u32) << 16,
        }
    });
    // Target ownership bytes are test sentinels; helper ownership words above are captured.
    let mut record = [0xa5; RECORD_LEN];
    record[..88].copy_from_slice(c.charinfo());
    Current {
        live: Observed {
            binding: Binding {
                epoch: 7,
                owner: 1,
                entry: 0x11f7220000,
                row: 0x86211990,
                holder: 0x11e105cc00,
                manager: 0x1182ff6800,
                vector_begin: 0x118407a800,
                vector_end: 0x118407de08,
                record_index: 1,
            },
            configuration: c,
        },
        thread: 0x89793088,
        information: 0x11f7220110,
        object: 0x11fc686e00,
        scope: Scope::cpu(owners, helpers, false),
        mirrors: mirrors(1, c.tokens()),
        row_ids: rows(c),
        record,
    }
}
fn with_config(mut current: Current, c: Configuration) -> Current {
    current.live.configuration = c;
    current.mirrors = mirrors(current.live.binding.record_index, c.tokens());
    current.row_ids = rows(c);
    current.record[..88].copy_from_slice(c.charinfo());
    current
}

fn menu_recapture_never_rebases_failed_cpu_scope_or_inflight_expected_state() {
    let original = cpu();
    let alternate = configuration(true);
    let mut state = SavedRole::capture(Role::Cpu, original).unwrap();
    assert_eq!(state.menu_current(original), Ok(()));
    let mut player_became_mii = original;
    player_became_mii.scope.cpu.as_mut().unwrap().owners[0].kind = 0x49;
    assert!(state.menu_current(player_became_mii).is_err());
    let mut changed_helper = original;
    changed_helper.scope.cpu.as_mut().unwrap().helpers[0].record[88] ^= 1;
    assert_eq!(state.menu_current(changed_helper), Err(Error::ContextChanged));
    assert_eq!(state.original(), original.live.configuration);
    assert_eq!(state.expected(), original.live.configuration);
    assert_eq!(state.phase(), Phase::Ready);

    let tx = state.select(7, original, Action::ReloadCpu(choice(alternate)), alternate)
        .unwrap().unwrap();
    assert!(state.menu_current(original).is_err(), "a prepared request is not a new ready capture");
    state.submit(tx.ticket(), original).unwrap();
    let ready = with_config(original, alternate);
    assert!(state.menu_current(ready).is_err(), "full completion must be accepted first");
    state.complete(tx.ticket(), ready).unwrap();
    assert_eq!(state.menu_current(ready), Ok(()));
    assert!(state.menu_current(original).is_err(), "old valid bytes are not the current expected state");
    assert!(state.menu_current(player_became_mii).is_err());
    assert_eq!(state.original(), original.live.configuration);
    assert_eq!(state.expected(), alternate);
    let restore = state.restore(ready).unwrap().unwrap();
    state.submit(restore.ticket(), ready).unwrap();
    state.complete(restore.ticket(), original).unwrap();
    assert_eq!(state.menu_current(original), Ok(()));
    state.depart();
    assert!(state.menu_current(original).is_err());
}

fn captured_cpu_apply_restore_preserves_original_helpers_and_owned_resource_tail() {
    let original = cpu();
    let alt = configuration(true);
    assert_eq!(
        inventory::fnv1a64(original.live.configuration.charinfo()),
        0x59b3890d69460ca5
    );
    assert_eq!(inventory::fnv1a64(alt.charinfo()), 0x7d8dbd9987f025b9);
    let mut state = SavedRole::capture(Role::Cpu, original).unwrap();
    let tx = state
        .select(7, original, Action::ReloadCpu(choice(alt)), alt)
        .unwrap()
        .unwrap();
    let commit = state.submit(tx.ticket(), original).unwrap();
    let payload = commit.prepare_write(original, rows(alt)).unwrap();
    let mut copied = original.record;
    payload.apply_to_copy(&mut copied).unwrap();
    assert_eq!(&copied[88..], &original.record[88..]);
    let mut ready = with_config(original, alt);
    ready.record = copied;
    // Native INSTALL owns target resource replacement; do not freeze that pair after reload.
    ready.record[88..].fill(0xb6);
    assert_eq!(state.complete(tx.ticket(), ready), Ok(Operation::Apply));
    assert_eq!(state.original(), original.live.configuration);
    let restore = state
        .select(
            7,
            ready,
            Action::ReloadCpu(choice(state.original())),
            state.original(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(restore.operation(), Operation::Restore);
    assert_eq!(restore.after(), original.live.configuration);
    state.submit(restore.ticket(), ready).unwrap();
    assert_eq!(
        state.complete(restore.ticket(), original),
        Ok(Operation::Restore)
    );
    assert!(state.can_change_kind_or_return_slot0(original).is_ok());
}

fn unwritten_submission_can_recover_and_restore_previous_alternate() {
    let original = cpu();
    let alt = configuration(true);
    let mut state = SavedRole::capture(Role::Cpu, original).unwrap();
    let tx = state
        .select(7, original, Action::ReloadCpu(choice(alt)), alt)
        .unwrap()
        .unwrap();
    state
        .submit(tx.ticket(), original)
        .unwrap()
        .prepare_write(original, rows(alt))
        .unwrap();
    let ready = with_config(original, alt);
    state.complete(tx.ticket(), ready).unwrap();
    let restore = state.restore(ready).unwrap().unwrap();
    let commit = state.submit(restore.ticket(), ready).unwrap();
    let mut changed = ready;
    changed.scope.cpu.as_mut().unwrap().helpers[0].record[96] ^= 1;
    assert!(commit
        .prepare_write(changed, rows(original.live.configuration))
        .is_err());
    // The native caller supplies the proved pre-copy/pre-INSTALL premise; this is not timeout.
    state.abandon_unwritten(restore.ticket()).unwrap();
    assert_eq!(state.phase(), Phase::Ready);
    assert_eq!(state.expected(), alt);
    assert_eq!(state.pending_ticket(), None);
    assert!(state.abandon_unwritten(restore.ticket()).is_err());
    let retry = state.restore(ready).unwrap().unwrap();
    state
        .submit(retry.ticket(), ready)
        .unwrap()
        .prepare_write(ready, rows(original.live.configuration))
        .unwrap();
    assert_eq!(
        state.complete(retry.ticket(), original),
        Ok(Operation::Restore)
    );
    let mut alias = original;
    alias.scope.cpu.as_mut().unwrap().owners[2].alias = 1;
    assert!(SavedRole::capture(Role::Cpu, alias).is_err());
}

fn terminal_native_readiness_settlement_retains_original_expected_and_refusal() {
    let original = cpu();
    let alt = configuration(true);
    for failure in 0..3 {
        let mut state = SavedRole::capture(Role::Cpu, original).unwrap();
        let apply = state
            .select(7, original, Action::ReloadCpu(choice(alt)), alt)
            .unwrap()
            .unwrap();
        state.submit(apply.ticket(), original).unwrap();
        let ready = with_config(original, alt);
        state.complete(apply.ticket(), ready).unwrap();
        let restore = state.restore(ready).unwrap().unwrap();
        state.submit(restore.ticket(), ready).unwrap();
        assert!(state.settle_terminal(restore.ticket()).is_err()); // Loading is not terminal.
        match failure {
            0 => {
                assert!(state.complete(restore.ticket(), ready).is_err());
            }
            1 => {
                state.refuse(restore.ticket()).unwrap();
            } // Fresh native comparison unavailable.
            _ => state.depart(),
        }
        let terminal = state.phase();
        assert!(matches!(terminal, Phase::Refused | Phase::Departed));
        assert!(state.settle_terminal(apply.ticket()).is_err()); // Prior transaction is not authority.
                                                                 // Glue must separately prove witnessed reload + exact live native readiness.
                                                                 // This pure receipt settlement is not evidence of that native premise.
        state.settle_terminal(restore.ticket()).unwrap();
        assert_eq!(state.pending_ticket(), None);
        assert_eq!(state.phase(), terminal);
        assert_eq!(state.original(), original.live.configuration);
        assert_eq!(state.expected(), alt);
        assert!(state.restore(ready).is_err());
        assert!(state
            .select(
                7,
                ready,
                Action::ReloadCpu(choice(original.live.configuration)),
                original.live.configuration
            )
            .is_err());
        assert!(state.settle_terminal(restore.ticket()).is_err());
    }
}

const ORIGINAL: [u8; 88] = [
    0xaa, 0x23, 0x0d, 0xce, 0x53, 0x8d, 0x4a, 0xb7, 0x84, 0x54, 0xf6, 0x84, 0x7d, 0x4b, 0xee, 0x68,
    0x51, 0x30, 0x93, 0x30, 0x58, 0x30, 0x85, 0x30, 0x64, 0x30, 0x45, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x40, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x2b, 0x07, 0x01, 0x02, 0x0c, 0x04, 0x03, 0x04, 0x02, 0x0b, 0x0c, 0x07, 0x03, 0x00, 0x06,
    0x03, 0x0a, 0x01, 0x04, 0x09, 0x17, 0x13, 0x04, 0x03, 0x0d, 0x08, 0x00, 0x00, 0x04, 0x0a, 0x00,
    0x08, 0x04, 0x0a, 0x00, 0x04, 0x02, 0x14, 0x00,
];
const ALT: [u8; 88] = [
    0x9c, 0x61, 0x64, 0x5d, 0xda, 0x0f, 0x4b, 0x2d, 0xb9, 0xfb, 0xe7, 0xcf, 0x1a, 0x4d, 0xb6, 0x7a,
    0x6e, 0x00, 0x6f, 0x00, 0x20, 0x00, 0x6e, 0x00, 0x61, 0x00, 0x6d, 0x00, 0x65, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x40, 0x40, 0x00, 0x00, 0x00, 0x04, 0x00,
    0x00, 0x44, 0x08, 0x00, 0x02, 0x08, 0x04, 0x03, 0x04, 0x02, 0x0c, 0x06, 0x08, 0x04, 0x03, 0x06,
    0x02, 0x0a, 0x01, 0x04, 0x09, 0x17, 0x13, 0x04, 0x03, 0x0d, 0x08, 0x00, 0x00, 0x04, 0x0a, 0x00,
    0x08, 0x04, 0x0a, 0x00, 0x04, 0x02, 0x14, 0x00,
];

// Changes only model class/configuration; original CPU address/resource bytes above retain
// their captured provenance. This proves transaction invariants, not native class readiness.
fn class_current(class: MiiClass, role: Role) -> Current {
    let config = configuration_for_class(class, false);
    let mut current = with_config(cpu(), config);
    if role == Role::Player {
        current.live.binding.owner = 0;
        current.live.binding.record_index = 0;
        current.scope = Scope::player([
            Some(class as u32), Some(9), Some(9), Some(9), None, None, None, None,
        ]);
        current.mirrors = mirrors(0, config.tokens());
    } else {
        let scope = current.scope.cpu.as_mut().unwrap();
        for owner in &mut scope.owners[1..] { owner.kind = class as u32; }
        for helper in &mut scope.helpers {
            helper.owner.kind = class as u32;
            helper.mirrors = mirrors(helper.owner.index, config.tokens());
            helper.record[..88].copy_from_slice(config.charinfo());
        }
    }
    current
}

fn new_saved_classes_keep_exact_restore_and_reject_cross_class_or_ordinary_scope() {
    for class in [MiiClass::Brawler, MiiClass::Gunner] {
        for role in [Role::Player, Role::Cpu] {
            let original = class_current(class, role);
            let alt = configuration_for_class(class, true);
            let action = |c| match role {
                Role::Player => Action::ReloadPlayer(choice(c)),
                Role::Cpu => Action::ReloadCpu(choice(c)),
            };
            let mut state = SavedRole::capture(role, original).unwrap();
            let wrong_class = configuration(true);
            assert!(state.select(7, original, action(wrong_class), wrong_class).is_err());
            assert_eq!(state.phase(), Phase::Ready);
            let tx = state.select(7, original, action(alt), alt).unwrap().unwrap();
            let commit = state.submit(tx.ticket(), original).unwrap();
            let payload = commit.prepare_write(original, rows(alt)).unwrap();
            let mut record = original.record;
            payload.apply_to_copy(&mut record).unwrap();
            assert_eq!(&record[..88], alt.charinfo());
            assert_eq!(&record[88..], &original.record[88..]);
            // Legacy P1 diagnostic stays Sword-only, even with a valid new-class transaction.
            assert!(crate::engine::mii::native_values::RecordUpdate::prepare(
                tx, original.live.binding, original.mirrors, [rows(original.live.configuration), rows(alt)], &original.record,
            ).is_err());
            let ready = with_config(original, alt);
            state.complete(tx.ticket(), ready).unwrap();
            let restore = state.restore(ready).unwrap().unwrap();
            let restore_payload = state.submit(restore.ticket(), ready).unwrap()
                .prepare_write(ready, rows(original.live.configuration)).unwrap();
            restore_payload.apply_to_copy(&mut record).unwrap();
            assert_eq!(record, original.record);
            state.complete(restore.ticket(), original).unwrap();
            assert_eq!(state.expected(), state.original());
            assert!(state.can_change_kind_or_return_slot0(original).is_ok());

            let mut ordinary = original;
            if role == Role::Player {
                ordinary.scope.player_kinds.as_mut().unwrap()[0] = Some(9);
            } else {
                ordinary.scope.cpu.as_mut().unwrap().owners[1].kind = 9;
            }
            assert!(SavedRole::capture(role, ordinary).is_err());
        }
    }
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("menu_recapture_never_rebases_failed_cpu_scope_or_inflight_expected_state", menu_recapture_never_rebases_failed_cpu_scope_or_inflight_expected_state as fn()),
        ("captured_cpu_apply_restore_preserves_original_helpers_and_owned_resource_tail", captured_cpu_apply_restore_preserves_original_helpers_and_owned_resource_tail as fn()),
        ("unwritten_submission_can_recover_and_restore_previous_alternate", unwritten_submission_can_recover_and_restore_previous_alternate as fn()),
        ("terminal_native_readiness_settlement_retains_original_expected_and_refusal", terminal_native_readiness_settlement_retains_original_expected_and_refusal as fn()),
        ("new_saved_classes_keep_exact_restore_and_reject_cross_class_or_ordinary_scope", new_saved_classes_keep_exact_restore_and_reject_cross_class_or_ordinary_scope as fn()),
    ]);
}
