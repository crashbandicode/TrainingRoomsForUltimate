//! File, index, and migration behavior for the recording redesign.

use std::os::unix::fs::MetadataExt;
use std::path::Path;

use serde_json::{json, Value};

use super::*;
use crate::engine::macros::{InputFrame, Library, MacroRecording, StartState};
use crate::model::combo::{Aim, Combo, Event, LinkRec};
use crate::model::combo_record::{ComboLibrary, ComboRecord};
use crate::model::frame_strip::{Button, Move, Verdict};
use crate::model::state::file::SaveFile;
use crate::model::state::record::SavedState;
use crate::model::store::{Store, SELECTED};

const DATE: &str = "2026-03-04";

fn scratch(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("tr-rr1-{tag}-{n}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn frame(buttons: u32) -> Vec<InputFrame> {
    vec![InputFrame { buttons, ..InputFrame::default() }]
}

fn start(kind: i32) -> StartState {
    StartState::neutral(kind, 0, 1.0)
}

fn live(kind: i32, buttons: u32) -> LegacySlot {
    LegacySlot { frames: frame(buttons), start: start(kind) }
}

fn link() -> LinkRec {
    LinkRec::presses(
        Move::Aerial,
        Move::Aerial,
        Verdict::True(None),
        Some((0, 4)),
        &[Event::press(Button::Jump, Aim::Neutral, 2)],
    )
}

fn combo(name: &str) -> ComboRecord {
    ComboRecord::new(name.to_owned(), Combo::from_links([link()]).unwrap(), None)
}

fn saved(kind: i32) -> SavedState {
    let mut state = SavedState::never_saved();
    state.fighter_kind = kind;
    state
}

fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        let mut paths: Vec<_> = std::fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().path()).collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.push((rel, std::fs::read(&path).unwrap()));
            }
        }
    }
    walk(root, root, &mut out);
    out.sort();
    out
}

#[test]
fn migration_is_idempotent() {
    let root = scratch("migrate");
    let library = Library::new(root.join(MACROS_DIR));
    let existing = MacroRecording::new("Already", "13.0.5", start(1), frame(4)).unwrap();
    let existing_id = library.save_new(&existing).unwrap();
    let selected = root.join(MACROS_DIR).join(SELECTED);
    std::fs::write(&selected, format!("{existing_id}\n")).unwrap();
    let selected_bytes = std::fs::read(&selected).unwrap();

    let mut saves = SaveFile::default();
    saves.player[0] = saved(7);
    saves.cpu[0] = saved(8);
    saves.player[2] = saved(9);
    let save_bytes = saves.to_pretty();
    std::fs::write(root.join("save_states.json"), &save_bytes).unwrap();
    // Slot index 2 is the settings slot. Indexes 1, 3 and 4 were never written.
    let settings = json!({"menu": {
        "playback_loop": [0, 1],
        "save_state_slot": [0, 0, 1, 0, 0],
        "save_state_playback": [1, 0, 1, 0, 0],
        "record_trigger": [1, 1]
    }});
    std::fs::write(root.join("settings.json"), serde_json::to_vec_pretty(&settings).unwrap()).unwrap();
    let settings_bytes = std::fs::read(root.join("settings.json")).unwrap();

    let slots = [
        Some(live(8, 1)),
        None,
        Some(live(9, 2)),
        Some(LegacySlot { frames: Vec::new(), start: start(8) }),
        None,
    ];
    let receipt = migrate(&root, &slots, DATE).unwrap();
    assert_eq!(receipt.recordings.len(), 2);
    assert_eq!(receipt.saves.len(), 2);
    assert_eq!(receipt.active_player.as_deref(), Some(existing_id.as_str()));
    assert_eq!(receipt.loop_player, LoopCount::Finite(1));
    assert_eq!(receipt.loop_cpu, LoopCount::Finite(1));

    let index = RecordingIndex::load(&root).unwrap();
    assert!(index.migrated());
    assert_eq!(index.active(Role::Player), Some(existing_id.as_str()));
    assert_eq!(index.active(Role::Cpu), None);
    assert_eq!(index.bound(), receipt.bound_save.as_deref());

    let macros = Library::new(root.join(MACROS_DIR));
    let macro_names: Vec<_> = receipt
        .recordings
        .iter()
        .map(|id| macros.load(id).unwrap().name)
        .collect();
    assert_eq!(macro_names, vec![format!("Slot 1 ({DATE})"), format!("Slot 3 ({DATE})")]);
    let save_store = Store::<NamedSave>::new(root.join(SAVES_DIR));
    let save_names: Vec<_> = receipt.saves.iter().map(|id| save_store.load(id).unwrap().name).collect();
    assert_eq!(save_names, vec![format!("Slot 1 ({DATE})"), format!("Slot 3 ({DATE})")]);
    let bound = receipt.bound_save.clone().unwrap();
    assert_eq!(save_store.load(&bound).unwrap().name, format!("Slot 3 ({DATE})"));
    assert!(!macro_names.iter().any(|name| name.contains("2026-11-05")));

    for id in &receipt.recordings {
        let text = std::fs::read_to_string(root.join(MACROS_DIR).join(format!("{id}.json"))).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        assert!(value.get("start_save").is_none());
        assert_eq!(value["game_version"], "13.0.5");
        assert_eq!(macros.load(id).unwrap().start_save, None);
    }
    let hot = index.hot(Kind::Macro);
    assert_eq!(macros.load(&hot[0]).unwrap().name, format!("Slot 1 ({DATE})"));
    assert_eq!(macros.load(&hot[1]).unwrap().name, format!("Slot 3 ({DATE})"));
    assert!(!hot.contains(&existing_id));
    assert!(!root.join(SEQUENCES_DIR).exists());
    assert!(Store::<Chain>::new(root.join(MACROS_DIR)).list().unwrap().entries.is_empty());
    assert_eq!(std::fs::read(root.join("save_states.json")).unwrap().as_slice(), save_bytes.as_bytes());
    assert_eq!(std::fs::read(root.join("settings.json")).unwrap().as_slice(), settings_bytes.as_slice());
    assert_eq!(std::fs::read(&selected).unwrap(), selected_bytes);

    let before = snapshot(&root);
    let index_path = root.join(INDEX_FILE);
    let inode = std::fs::metadata(&index_path).unwrap().ino();
    let again = migrate(&root, &slots, DATE).unwrap();
    assert_eq!(again, receipt);
    assert_eq!(std::fs::metadata(&index_path).unwrap().ino(), inode);
    assert_eq!(snapshot(&root), before);

    // A crash before the flag: the index is gone, the named files are still there, and a retry reuses them.
    std::fs::remove_file(&index_path).unwrap();
    let retried = migrate(&root, &slots, DATE).unwrap();
    assert_eq!(retried.recordings, receipt.recordings);
    assert_eq!(retried.saves, receipt.saves);
    assert_eq!(retried.bound_save, receipt.bound_save);
    let macro_files = std::fs::read_dir(root.join(MACROS_DIR))
        .unwrap()
        .filter(|entry| entry.as_ref().unwrap().path().extension().is_some_and(|ext| ext == "json"))
        .count();
    assert_eq!(macro_files, 3);

    let bare = scratch("no-settings");
    let mut only = SaveFile::default();
    only.player[0] = saved(4);
    std::fs::write(bare.join("save_states.json"), only.to_pretty()).unwrap();
    let bare_slots = [None, None, None, None, None];
    let bare_receipt = migrate(&bare, &bare_slots, DATE).unwrap();
    let bound = bare_receipt.bound_save.unwrap();
    let bare_saves = Store::<NamedSave>::new(bare.join(SAVES_DIR));
    assert_eq!(bare_saves.load(&bound).unwrap().name, format!("Slot 1 ({DATE})"));
}

#[test]
fn migration_does_not_build_a_chain() {
    let root = scratch("no-chain");
    let settings = json!({"menu": {
        "save_state_playback": [1, 0, 1, 1, 0],
        "record_trigger": [0, 1]
    }});
    std::fs::write(root.join("settings.json"), serde_json::to_vec(&settings).unwrap()).unwrap();
    let mut saves = SaveFile::default();
    saves.player[0] = saved(3);
    std::fs::write(root.join("save_states.json"), saves.to_pretty()).unwrap();
    let slots = [Some(live(8, 1)), Some(live(8, 2)), None, None, None];
    let receipt = migrate(&root, &slots, DATE).unwrap();
    assert_eq!(receipt.recordings.len(), 2);
    assert!(Store::<Chain>::new(root.join(MACROS_DIR)).list().unwrap().entries.is_empty());
    assert!(!root.join(SEQUENCES_DIR).exists());
    for (rel, bytes) in snapshot(&root) {
        if rel.ends_with(".json") {
            let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
            assert_ne!(value.get("kind").and_then(Value::as_str), Some("chain"), "{rel}");
        }
    }
}

#[test]
fn migration_maps_the_loop() {
    assert_eq!(mapped("loop-on", Some(vec![1, 0])), LoopCount::Infinite);
    assert_eq!(mapped("loop-off", Some(vec![0, 1])), LoopCount::Finite(1));
    assert_eq!(mapped("loop-missing", None), LoopCount::Finite(1));
    let fresh = RecordingIndex::load(&scratch("fresh-index")).unwrap();
    assert_eq!(fresh.loop_count(Role::Player), LoopCount::Finite(1));
    assert_eq!(fresh.loop_count(Role::Cpu), LoopCount::Finite(1));
    assert!(!fresh.migrated());
}

fn mapped(tag: &str, row: Option<Vec<u8>>) -> LoopCount {
    let root = scratch(tag);
    if let Some(row) = row {
        let text = serde_json::to_vec(&json!({"menu": {"playback_loop": row}})).unwrap();
        std::fs::write(root.join("settings.json"), text).unwrap();
    }
    let slots = [Some(live(8, 1)), None, None, None, None];
    let receipt = migrate(&root, &slots, DATE).unwrap();
    let index = RecordingIndex::load(&root).unwrap();
    assert_eq!(index.loop_count(Role::Player), receipt.loop_player);
    assert_eq!(index.loop_count(Role::Cpu), receipt.loop_cpu);
    assert_eq!(receipt.loop_player, receipt.loop_cpu);
    if receipt.loop_player == LoopCount::Infinite {
        let text = std::fs::read_to_string(root.join(INDEX_FILE)).unwrap();
        assert!(text.contains("infinite"));
    }
    receipt.loop_player
}

#[test]
fn hot_five_are_the_most_recently_used() {
    let root = scratch("hot");
    let mut index = RecordingIndex::fresh();
    let reasons = [Use::Bound, Use::SetActive, Use::Played, Use::Drilled, Use::Recorded, Use::Played];

    let macros = Library::new(root.join(MACROS_DIR));
    let mut macro_ids = Vec::new();
    for n in 1..=6 {
        macro_ids.push(macros.save_new(&MacroRecording::new(format!("Macro {n}"), "13.0.5", start(8), frame(1)).unwrap()).unwrap());
    }
    note_all(&mut index, Kind::Macro, &macro_ids, &reasons);
    assert_eq!(index.hot(Kind::Macro).len(), 5);
    assert_eq!(index.hot(Kind::Macro)[0], macro_ids[5]);
    assert!(!index.hot(Kind::Macro).contains(&macro_ids[0]));
    assert!(root.join(MACROS_DIR).join(format!("{}.json", macro_ids[0])).is_file());

    let chain = Chain::new("Bread", vec![macro_ids[5].clone()]).unwrap();
    let chain_id = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&chain).unwrap();
    index.note_use(Kind::Macro, &chain_id, Use::Played, 7).unwrap();
    assert_eq!(index.hot(Kind::Macro)[0], chain_id);
    assert!(root.join(MACROS_DIR).join(format!("{chain_id}.json")).is_file());
    assert!(!index.hot(Kind::Save).contains(&chain_id));

    let save_store = Store::<NamedSave>::new(root.join(SAVES_DIR));
    let mut save_ids = Vec::new();
    for n in 1..=6 {
        save_ids.push(save_store.save_new(&NamedSave::new(format!("Save {n}"), saved(n), saved(n)).unwrap()).unwrap());
    }
    note_all(&mut index, Kind::Save, &save_ids, &reasons);
    assert_eq!(index.hot(Kind::Save)[0], save_ids[5]);
    assert!(!index.hot(Kind::Save).contains(&save_ids[0]));
    assert!(root.join(SAVES_DIR).join(format!("{}.json", save_ids[0])).is_file());

    let sequences = Store::<Sequence>::new(root.join(SEQUENCES_DIR));
    let mut sequence_ids = Vec::new();
    for n in 1..=6 {
        sequence_ids.push(sequences.save_new(&Sequence::new(format!("Seq {n}"), Vec::new()).unwrap()).unwrap());
    }
    note_all(&mut index, Kind::Sequence, &sequence_ids, &reasons);
    assert_eq!(index.hot(Kind::Sequence)[0], sequence_ids[5]);
    assert!(!index.hot(Kind::Sequence).contains(&sequence_ids[0]));
    assert!(root.join(SEQUENCES_DIR).join(format!("{}.json", sequence_ids[0])).is_file());

    let combos = ComboLibrary::new(root.join("combos"));
    let mut combo_ids = Vec::new();
    for n in 1..=6 {
        combo_ids.push(combos.save_new(&combo(&format!("Combo {n}"))).unwrap());
    }
    note_all(&mut index, Kind::Combo, &combo_ids, &reasons);
    assert_eq!(index.hot(Kind::Combo)[0], combo_ids[5]);
    assert!(!index.hot(Kind::Combo).contains(&combo_ids[0]));
    assert!(root.join("combos").join(format!("{}.json", combo_ids[0])).is_file());

    index.set_drill(Some(Drill { kind: DrillKind::Combo, id: combo_ids[5].clone() }), 20).unwrap();
    assert_eq!(index.drill().unwrap().id, combo_ids[5]);
    let text = serde_json::to_string(&index).unwrap();
    assert!(!text.contains("\"chain\""));
}

fn note_all(index: &mut RecordingIndex, kind: Kind, ids: &[String], reasons: &[Use]) {
    for (offset, (id, reason)) in ids.iter().zip(reasons).enumerate() {
        index.note_use(kind, id, *reason, (offset as u64) + 1).unwrap();
    }
}

#[test]
fn a_chain_file_is_an_ordered_list_of_recordings() {
    let root = scratch("chain");
    let chain = Chain::new("Bread and butter", vec!["nair".into(), "upsmash".into()]).unwrap();
    assert!(chain.start_save_on);
    let store = Store::<Chain>::new(root.join(MACROS_DIR));
    let id = store.save_new(&chain).unwrap();
    let loaded = store.load(&id).unwrap();
    assert_eq!(loaded.recordings, vec!["nair".to_owned(), "upsmash".to_owned()]);
    let value: Value = serde_json::from_slice(&std::fs::read(root.join(MACROS_DIR).join(format!("{id}.json"))).unwrap()).unwrap();
    assert_eq!(value["kind"], "chain");
    assert_eq!(value["recordings"], json!(["nair", "upsmash"]));
    assert!(value.get("start_save").is_none());

    let recording = MacroRecording::new("Nair", "13.0.5", start(8), frame(1)).unwrap();
    let recording_id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
    let recordings = Library::new(root.join(MACROS_DIR)).list().unwrap();
    assert!(recordings.entries.iter().all(|entry| entry.id != id));
    assert!(recordings.skipped.iter().all(|skipped| skipped.file != format!("{id}.json")));
    assert!(recordings.entries.iter().any(|entry| entry.id == recording_id));
    let chains = store.list().unwrap();
    assert!(chains.entries.iter().any(|entry| entry.id == id));
    assert!(chains.entries.iter().all(|entry| entry.id != recording_id));
    assert!(chains.skipped.iter().all(|skipped| skipped.file != format!("{recording_id}.json")));

    std::fs::write(root.join(MACROS_DIR).join("broken.json"), b"{").unwrap();
    let recordings = Library::new(root.join(MACROS_DIR)).list().unwrap();
    assert!(recordings.skipped.iter().any(|skipped| skipped.file == "broken.json"));
    let chains = Store::<Chain>::new(root.join(MACROS_DIR)).list().unwrap();
    assert!(chains.skipped.iter().all(|skipped| skipped.file != "broken.json"));
}

#[test]
fn the_start_save_switch_defaults_on_and_off_keeps_the_id() {
    let mut recording = MacroRecording::new("Nair", "13.0.5", start(8), frame(1)).unwrap();
    assert!(recording.start_save.is_none());
    assert!(recording.start_save_on);
    let fresh = serde_json::to_value(&recording).unwrap();
    assert!(fresh.get("start_save").is_none());
    assert!(fresh.get("start_save_on").is_none());

    recording.set_start_save(Some("slot-1")).unwrap();
    assert_eq!(recording.start_save.as_deref(), Some("slot-1"));
    assert!(recording.start_save_on);
    recording.set_start_save_on(false);
    assert!(!recording.start_save_on);
    assert_eq!(recording.start_save.as_deref(), Some("slot-1"));
    let stored = serde_json::to_value(&recording).unwrap();
    assert_eq!(stored["start_save"], "slot-1");
    assert_eq!(stored["start_save_on"], false);
    let loaded: MacroRecording = serde_json::from_value(stored).unwrap();
    assert_eq!(loaded.start_save.as_deref(), Some("slot-1"));
    assert!(!loaded.start_save_on);

    recording.set_start_save(None).unwrap();
    recording.set_start_save_on(false);
    assert!(recording.start_save.is_none());
    assert!(recording.start_save_on);
    assert!(recording.set_start_save(Some("../no")).is_err());
}

#[test]
fn start_save_state_is_linked_deleted_or_none() {
    assert_eq!(start_save_state(None, true, false), StartSaveState::None);
    assert_eq!(start_save_state(Some(""), false, true), StartSaveState::None);
    assert_eq!(start_save_state(Some("slot-1"), true, true), StartSaveState::Linked);
    assert_eq!(start_save_state(Some("slot-1"), false, true), StartSaveState::Linked);
    assert_eq!(start_save_state(Some("slot-1"), true, false), StartSaveState::Deleted);
    assert_eq!(start_save_state(Some("slot-1"), false, false), StartSaveState::Deleted);

    let root = scratch("link");
    let id = Store::<NamedSave>::new(root.join(SAVES_DIR))
        .save_new(&NamedSave::new("Slot 1", saved(7), SavedState::never_saved()).unwrap())
        .unwrap();
    let exists = root.join(SAVES_DIR).join(format!("{id}.json")).is_file();
    assert_eq!(start_save_state(Some(&id), false, exists), StartSaveState::Linked);
    assert_eq!(start_save_state(Some("missing-save"), true, false), StartSaveState::Deleted);
}

#[test]
fn set_active_replaces_the_roles_macro() {
    let mut index = RecordingIndex::fresh();
    index.set_active(Role::Player, Some("alpha"), 1).unwrap();
    index.set_active(Role::Cpu, Some("cpu-1"), 2).unwrap();
    index.set_active(Role::Player, Some("beta"), 3).unwrap();
    assert_eq!(index.active(Role::Player), Some("beta"));
    assert_eq!(index.active(Role::Cpu), Some("cpu-1"));
    assert_eq!(index.hot(Kind::Macro)[0], "beta");
    index.set_active(Role::Player, None, 4).unwrap();
    assert_eq!(index.active(Role::Player), None);
    assert_eq!(index.active(Role::Cpu), Some("cpu-1"));
}

#[test]
fn each_role_has_its_own_loop_count() {
    let root = scratch("loops");
    let mut index = RecordingIndex::fresh();
    assert_eq!(index.loop_count(Role::Player), LoopCount::Finite(1));
    assert_eq!(index.loop_count(Role::Cpu), LoopCount::Finite(1));
    index.set_loop(Role::Player, LoopCount::Infinite);
    index.set_loop(Role::Cpu, LoopCount::try_from(3).unwrap());
    assert_eq!(index.loop_count(Role::Player), LoopCount::Infinite);
    assert_eq!(index.loop_count(Role::Cpu), LoopCount::Finite(3));
    for count in [1, 2, 3, 5, 10] {
        assert_eq!(LoopCount::try_from(count).unwrap(), LoopCount::Finite(count));
    }
    assert!(LoopCount::try_from(4).is_err());
    assert!(LoopCount::try_from(0).is_err());
    index.save(&root).unwrap();
    let loaded = RecordingIndex::load(&root).unwrap();
    assert_eq!(loaded.loop_count(Role::Player), LoopCount::Infinite);
    assert_eq!(loaded.loop_count(Role::Cpu), LoopCount::Finite(3));
    let text = std::fs::read_to_string(root.join(INDEX_FILE)).unwrap();
    assert!(text.contains("infinite"));
    assert!(text.contains("\"cpu\": 3") || text.contains("\"cpu\":3"));
}

#[test]
fn a_sequence_follows_a_renamed_combo_and_skips_a_deleted_one() {
    let root = scratch("sequence");
    let combos = ComboLibrary::new(root.join("combos"));
    let nair = combos.save_new(&combo("Nair")).unwrap();
    let fair = combos.save_new(&combo("Fair")).unwrap();
    let usmash = combos.save_new(&combo("Usmash")).unwrap();
    let mut sequence = Sequence::new("Line", vec![nair.clone(), fair.clone(), usmash.clone()]).unwrap();
    sequence.streak = 2;
    sequence.best = 4;
    let store = Store::<Sequence>::new(root.join(SEQUENCES_DIR));
    let id = store.save_new(&sequence).unwrap();

    let mut renamed = combos.load(&fair).unwrap();
    renamed.name = "Fair renamed".into();
    combos.overwrite(&fair, &renamed).unwrap();
    let open = store.load(&id).unwrap();
    assert_eq!(open.present_ids(&combos), vec![nair.clone(), fair.clone(), usmash.clone()]);
    assert_eq!(combos.load(&fair).unwrap().name, "Fair renamed");

    combos.delete(&usmash).unwrap();
    let open = store.load(&id).unwrap();
    assert_eq!(open.combos, vec![nair.clone(), fair.clone(), usmash]);
    assert_eq!(open.present_ids(&combos), vec![nair, fair]);
    assert_eq!((open.streak, open.best), (2, 4));
}

#[test]
fn old_recording_files_still_load() {
    let root = scratch("old");
    let ordinary = MacroRecording::new("Old", "13.0.5", start(8), frame(1)).unwrap();
    let ordinary_json = serde_json::to_value(&ordinary).unwrap();
    assert!(ordinary_json.get("start_save").is_none());
    assert!(ordinary_json.get("start_save_on").is_none());
    let library = Library::new(root.join(MACROS_DIR));
    let ordinary_id = library.save_new(&ordinary).unwrap();
    let loaded = library.load(&ordinary_id).unwrap();
    assert_eq!(loaded.schema_version, 1);
    assert_eq!(loaded.start_save, None);
    assert!(loaded.start_save_on);

    let mut exact = MacroRecording::new("Old Mii", "13.0.5", StartState::neutral(0x48, 0, 1.0), frame(1)).unwrap();
    exact.set_mii(exact_mii()).unwrap();
    let exact_json = serde_json::to_value(&exact).unwrap();
    assert_eq!(exact_json["schema_version"], 2);
    assert!(exact_json.get("start_save").is_none());
    assert!(exact_json.get("start_save_on").is_none());
    let exact_id = library.save_new(&exact).unwrap();
    let loaded = library.load(&exact_id).unwrap();
    assert_eq!(loaded.schema_version, 2);
    assert!(loaded.mii.is_some());
    assert_eq!(loaded.start_save, None);
    assert!(loaded.start_save_on);
}

fn exact_mii() -> crate::engine::mii::lifetime::Configuration {
    use crate::engine::mii::inventory::{classify, MiiClass, RawSlot, TaggedId, CHARINFO_LEN, TAG_CHARA, TAG_HAT, TAG_OUTFIT};
    let saved = classify(&RawSlot {
        index: 0,
        slot_id: 13231,
        name_len: 1,
        name_units: [65, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        charinfo: [7; CHARINFO_LEN],
        colour: 4,
        voice: 37,
        specials: [1, 0, 0, 0],
        tag_outfit: TaggedId((u64::from(TAG_OUTFIT) << 56) | (1 << 40) | 0x1b_fef7_a5f7),
        tag_hat: TaggedId((u64::from(TAG_HAT) << 56) | 0x0f_a9ef_a5ea),
        tag_chara: TaggedId((u64::from(TAG_CHARA) << 56) | MiiClass::Brawler.chara_hash40()),
    })
    .unwrap();
    crate::engine::mii::lifetime::Configuration::from_saved(&saved).unwrap()
}
