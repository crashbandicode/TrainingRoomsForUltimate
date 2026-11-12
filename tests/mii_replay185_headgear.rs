//! Source candidate tests, not native loading acceptance.
//! Independent facts: REPLAY185 native CFMK/companion/import agree on Gunner,
//! outfit75/hat125/colour0/voice37/specials2221 and the canonical below. Only
//! CharInfo's noncanonical create-id/name prefix is redacted. Hat7 is explicitly
//! a synthetic variation; its resource presence is not claimed.
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

use {crate::engine::mii::backend::Layout, crate::engine::mii::inventory::MiiClass, crate::engine::mii::lifetime::{Binding, Configuration, Tokens}, crate::engine::mii::native_values::{Mirrors, RowIds}, crate::engine::mii::ordinary_cpu::{self as cpu, HatPublication}, crate::engine::mii::ordinary_player::{self, Owner, SameClass, SameClassValues}, crate::engine::mii::replay::ReplayMiis};

const REPLAY_ROWS: RowIds = RowIds { outfit: 0x1300_4b14_67a2_d93a, hat: 0x5a00_7d15_3956_e6b8 };
const HATLESS_ROWS: RowIds = RowIds { outfit: 0x1300_441a_f17e_2dff, hat: 0x5a00_000f_a9ef_a5ea };
const OTHER_ROWS: RowIds = RowIds { outfit: 0x1300_4b14_67a2_d93a, hat: 0x5a00_0712_3456_789a };
// mii-presets.json / CPU preference, independently captured, unaffected by redaction.
const CPU_PREF: &str = "4a0000000000000000000000000000000000000000000000000000000000000000000000000000000b0040400000050900004b08000a0807060100120c08070601010c0204090d0808060108050108051114050401040214000202020100253ad9a26714b8e6563915871dab5712";

fn companion() -> Vec<u8> {
    let mut bytes = vec![1, 1, 1, 0x4a, 1, 0]; // schema,count,entry,kind,source index
    bytes.extend_from_slice(&[0; 0x26]); // redacted, noncanonical metadata only
    bytes.extend_from_slice(&[
        0,11,0,64,64,0,0,5,9,0,0,75,8,0,10,8,7,6,1,0,18,12,8,7,6,
        1,1,12,2,4,9,13,8,8,6,1,8,5,1,8,5,17,20,5,4,1,4,2,20,0,
    ]);
    bytes.extend_from_slice(&[75,0,125,0,0,0,37,0,2,2,2,1]);
    bytes
}
fn replay() -> Configuration {
    let bytes = companion();
    assert_eq!(bytes.len(), 106);
    let miis = ReplayMiis::from_bytes(&bytes).unwrap();
    let raw = miis.for_track(1, 0x4a).unwrap();
    assert_eq!((raw.entry(), raw.class(), raw.source_index()), (1, MiiClass::Gunner, 1));
    assert_eq!(raw.tokens(), Tokens { outfit:75, hat:125, colour:0, voice:37, specials:[2,2,2,1] });
    let c = raw.resolve(REPLAY_ROWS).unwrap();
    assert_eq!(c.canonical().to_hex(), CPU_PREF);
    c
}
fn hatless() -> Configuration {
    // Saved11194 token/row facts; synthetic face, not a native save fixture.
    Configuration::from_observed(MiiClass::Gunner,
        Tokens { outfit:68, hat:0, colour:4, voice:37, specials:[0;4] },
        [0;88], HATLESS_ROWS).unwrap()
}
fn other() -> Configuration {
    let c = replay();
    Configuration::from_observed(MiiClass::Gunner, Tokens { hat:7, ..c.tokens() },
        *c.charinfo(), OTHER_ROWS).unwrap()
}
fn origin() -> [u8;0x70] {
    let mut d = [0xa5;0x70];
    d[4..8].copy_from_slice(&1u32.to_le_bytes());
    d[0x18..0x1c].copy_from_slice(&10u32.to_le_bytes()); // captured CPU Ness
    d[0x37] = 77; // live native-rank regression must remain preserved
    d[0x46..0x48].copy_from_slice(&0u16.to_le_bytes());
    d[0x4a..0x4c].copy_from_slice(&0u16.to_le_bytes());
    d
}

fn saved_row_uses_the_written_selector_and_origin_restore_uses_its_own_index() {
    let c = replay();
    // (layout, physical owner, immutable origin index, selector destination)
    let rows = [
        (Layout::Training, 1usize, 0u16, 1u16),
        (Layout::Ldn, 4, 26, 4),
        (Layout::Quickplay, 4, 26, 4),
        (Layout::Ldn, 5, 27, 5),
        (Layout::Quickplay, 5, 27, 5),
        (Layout::Ldn, 6, 28, 6),
        (Layout::Quickplay, 6, 28, 6),
    ];
    for (layout, owner, origin_index, destination) in rows {
        let mut origin = [0u8; 0x70];
        origin[0x46..0x48].copy_from_slice(&origin_index.to_le_bytes());
        let frozen = origin;
        let saved = cpu::staged_descriptor_in(layout, owner, origin, Some(c)).unwrap();
        let saved_index = u16::from_le_bytes(saved[0x46..0x48].try_into().unwrap());
        assert_eq!(saved_index, destination);
        assert_eq!(origin, frozen);
        assert_ne!(origin_index, destination);
        assert!(cpu::saved_payload_routes_to_destination(saved_index, destination));
        assert!(!cpu::saved_payload_routes_to_destination(origin_index, destination));
        let saved_pub = HatPublication {
            layout, owner: owner as u32, source: layout.helper(owner).is_some(),
            opt_in: true, initialized: false, fingerprints: true,
            before: 0, target: c.tokens().hat,
            destination_index: destination, original_index: origin_index,
            descriptor: &saved, payload: Some(c),
        };
        assert!(saved_pub.validate().is_ok());
        // Same hat and kind as the saved row. Only +0x46 is the origin index.
        let mut origin_index_on_saved = saved;
        origin_index_on_saved[0x46..0x48].copy_from_slice(&origin_index.to_le_bytes());
        assert!(HatPublication { descriptor: &origin_index_on_saved, ..saved_pub }.validate().is_err());
        let restore = HatPublication {
            before: c.tokens().hat, target: 0, payload: None, descriptor: &origin, ..saved_pub
        };
        assert!(restore.validate().is_ok());
        let mut saved_as_origin = saved;
        saved_as_origin[0x4a..0x4c].copy_from_slice(&0u16.to_le_bytes());
        assert!(HatPublication { descriptor: &saved_as_origin, ..restore }.validate().is_err());
    }
}

fn retained_same_class_proof_keeps_its_original_changed_hat_refusal() {
    let before = hatless(); let after = replay(); let t = before.tokens();
    let d = cpu::staged_descriptor(origin(), Some(before));
    let owner = Owner { entry:0x1000,row:0x2000,information:0x3000,object:0x4000,
        index:1,selector:1,alias:0 };
    let values = SameClassValues {
        binding:Binding { epoch:7,owner:1,entry:owner.entry,row:owner.row,holder:0x5000,
            manager:0x6000,vector_begin:0x7000,vector_end:0x7000+4*0x68,record_index:1 },
        thread:0x9000,owner,header:[1,0x4a,u32::MAX,u32::MAX,u32::from(d[0x41])],
        mirrors:Mirrors { index:1,tokens:t,fe:[1,68,0,4,37],d:[1,68,0,4],
            d_specials:[0;4],owner_index:1,owner:[68,0,4,37] },
        initialized:true,task_idle:true,rebuild_open:true,
    };
    let record = ordinary_player::retained_record([0xa5;0x68], Some(before));
    assert_eq!(SameClass::prepare_cpu_in(Layout::Training,before,after,values,&d,
        [HATLESS_ROWS,REPLAY_ROWS],&record).err(), Some("same-class hat changed"));
}

fn existing_completion_verifier_requires_target_hat_in_all_four_mirrors() {
    for (c,rows) in [(replay(),REPLAY_ROWS),(other(),OTHER_ROWS)] {
        let t=c.tokens();
        let mirrors=Mirrors {index:1,tokens:t,fe:[1,75,t.hat,0,37],d:[1,75,t.hat,0],
            d_specials:[2,2,2,1],owner_index:1,owner:[75,u32::from(t.hat),0,37]};
        let record=ordinary_player::retained_record([0xa5;0x68],Some(c));
        assert!(crate::engine::mii::role_owner::validate_saved(c,1,mirrors,rows,&record).is_ok());
        for field in 0..4 {
            let mut stale=mirrors;
            match field {
                0=>stale.tokens.hat=0,
                1=>stale.fe[2]=0,
                2=>stale.d[2]=0,
                _=>stale.owner[1]=0,
            }
            assert!(crate::engine::mii::role_owner::validate_saved(c,1,stale,rows,&record).is_err());
        }
    }
}

fn idle_cpu_preflight_recaptures_native_rank_before_exact_transaction_checks() {
    // Runtime wiring matters: menu-open recapture alone misses AI changes while
    // waiting practice continues behind the menu. Policy's native_rank_tests
    // execute the accepted deltas and reject all unrelated descriptor changes.
    let cpu = include_str!("../src/runtime/mii/mii_ordinary_cpu.rs");
    let refresh = cpu.split("unsafe fn refresh_idle_metadata(").nth(1).unwrap()
        .split("unsafe fn begin(").next().unwrap();
    assert!(refresh.contains("ordinary_cpu::recapture_rank("));
    assert!(refresh.contains("native::same_owned(&now,&comparison,retained || active_present)"));
    assert!(refresh.contains("state.reservation!=reservation || epoch()!=generation"));
    assert!(refresh.contains("owner.expected==expected && owner.live==live"));
    assert!(refresh.contains("expected.descriptor=comparison.descriptor"));
    // rc.8 R5: the preflight snapshot is `preflight_current`, in its own frame, and takes the refreshed `expected`.
    let preflight = cpu.split("unsafe fn preflight_current(").nth(1).unwrap().split("unsafe fn ").next().unwrap();
    assert!(preflight.contains("Ok(now) if native::same_owned(&now,expected,retained)=>Some(now),"));
    let begin = cpu.split("unsafe fn begin_prepare(").nth(1).unwrap();
    assert!(begin.find("let expected=refresh_idle_metadata(").unwrap()
        < begin.find("let Some(current)=preflight_current(entry,origin.original_kind,&expected,retained || active.is_some(),request)").unwrap());
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("saved_row_uses_the_written_selector_and_origin_restore_uses_its_own_index", saved_row_uses_the_written_selector_and_origin_restore_uses_its_own_index as fn()),
        ("retained_same_class_proof_keeps_its_original_changed_hat_refusal", retained_same_class_proof_keeps_its_original_changed_hat_refusal as fn()),
        ("existing_completion_verifier_requires_target_hat_in_all_four_mirrors", existing_completion_verifier_requires_target_hat_in_all_four_mirrors as fn()),
        ("idle_cpu_preflight_recaptures_native_rank_before_exact_transaction_checks", idle_cpu_preflight_recaptures_native_rank_before_exact_transaction_checks as fn()),
    ]);
}
