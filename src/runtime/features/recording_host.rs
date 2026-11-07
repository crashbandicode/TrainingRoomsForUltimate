//! Switch side of the recording worker. Decisions live in `engine::rr_host`, which the host tests compile.

use std::path::Path;

use crate::engine::library::LegacySlot;
use crate::engine::macros::InputFrame;
use crate::engine::rr_host::{self, Tick};
use crate::model::input_delay::MappedInputs;

const NAME_TOKEN: u64 = 1 << 62;
const NOTICE_ID: u8 = 0x2C;
const NOTICE_COLOR: [u8; 4] = [255, 255, 255, 255];

pub fn on_chords(record: bool, play_player: bool, play_cpu: bool) {
    rr_host::on_chords(record, play_player, play_cpu);
}

pub fn note_mash() {
    rr_host::note_mash();
}

pub fn push_capture(side: u8, frame: InputFrame) {
    rr_host::push_capture(side, frame);
}

pub(crate) fn capture_role() -> u8 {
    rr_host::capture_role()
}

pub(crate) fn take_cpu_frame() -> Option<InputFrame> {
    rr_host::take_cpu_frame()
}

/// # Safety
/// `out` is null or a live mapped-input record for this poll.
pub unsafe fn on_player_poll(out: *mut MappedInputs) {
    rr_host::on_player_poll(out);
}

pub fn service() {
    let root = Path::new(crate::model::storage::ROOT);
    if !root.is_dir() {
        let _ = std::fs::create_dir_all(root);
        if !root.is_dir() {
            return;
        }
    }
    take_mismatch_answer();
    let player = super::states::live_saved(0);
    let cpu = super::states::live_saved(1);
    if player.fighter_kind >= 0 {
        rr_host::set_role_fighter(crate::engine::library::Role::Player, player.fighter_kind);
    }
    if cpu.fighter_kind >= 0 {
        rr_host::set_role_fighter(crate::engine::library::Role::Cpu, cpu.fighter_kind);
    }
    rr_host::set_strip_sample(super::combo::strip_sample());
    if player.fighter_kind >= 0 {
        rr_host::set_fighter_label(&crate::runtime::macros::named_macros::fighter_label(player.fighter_kind));
    }
    let online = keyboard_closed();
    // Migration waits for the old slot image. A record, a play, or a menu effect does not.
    if !rr_host::migrated() {
        if let Some(slots) = super::recorder::copied_slots() {
            let tick = rr_host::service_at(root, &slots, &rr_host::today(), player, cpu, online);
            if rr_host::migrated() {
                super::recorder::release_slot_image();
            }
            apply(&tick);
            return;
        }
    }
    if !rr_host::pending() {
        return;
    }
    let tick = rr_host::service_pending(root, player, cpu, online);
    apply(&tick);
}

fn take_mismatch_answer() {
    let Some(choice) = crate::runtime::macros::named_macros::take_recording_fighter_choice() else {
        return;
    };
    let answer = match choice {
        Some(crate::engine::macro_ui::macro_panel::FighterChoice::Switch) => {
            rr_host::MismatchChoice::Switch
        }
        Some(crate::engine::macro_ui::macro_panel::FighterChoice::Keep) => rr_host::MismatchChoice::Keep,
        None => rr_host::MismatchChoice::Cancel,
    };
    rr_host::answer_mismatch(answer);
}

fn apply(tick: &Tick) {
    if let Some(ask) = &tick.mismatch {
        let recorded = if ask.recorded_kind >= 0 {
            crate::runtime::macros::named_macros::fighter_label(ask.recorded_kind)
        } else {
            ask.recorded_label.clone()
        };
        let current = if ask.current_kind >= 0 {
            crate::runtime::macros::named_macros::fighter_label(ask.current_kind)
        } else {
            ask.current_label.clone()
        };
        let _ = crate::runtime::macros::named_macros::open_recording_fighter_prompt(
            crate::engine::macro_ui::macro_panel::FighterOrigin::Recording,
            recorded,
            current,
        );
    }
    if let Some(kind) = tick.switch_kind {
        let requested = if tick.switch_player {
            crate::runtime::roster::character_roster::request_player_kind(kind as u32)
        } else {
            crate::runtime::roster::character_roster::request_cpu_kind(kind as u32)
        };
        let _ = requested;
    }
    if let Some(name) = &tick.switch_name {
        if let Some(kind) = (0..128u32).find(|kind| {
            crate::runtime::macros::named_macros::fighter_label(*kind as i32) == name.as_str()
        }) {
            let _ = crate::runtime::roster::character_roster::request_player_kind(kind);
        }
    }
    if let Some(id) = &tick.warp {
        let root = std::path::Path::new(crate::model::storage::ROOT);
        let saves = crate::model::store::Store::<crate::engine::library::NamedSave>::new(
            root.join(crate::engine::library::SAVES_DIR),
        );
        if let Ok(save) = saves.load(id) {
            super::states::request_named_load([save.player, save.cpu]);
        }
    }
    for text in &tick.notices {
        super::notices::upsert(NOTICE_ID, "Recording", text, 240, NOTICE_COLOR);
    }
    if let Some(offer) = &tick.keyboard {
        crate::runtime::macros::named_macros::offer_external_name_titled(
            NAME_TOKEN,
            offer.header,
            &offer.draft,
            "Name",
        );
    }
    emit(&tick.diags);
}

fn emit(lines: &[String]) {
    #[cfg(feature = "diag")]
    {
        for line in lines {
            crate::runtime::diag_log::diagnostic(format_args!("[PracticeRecording] {line}"));
        }
    }
    #[cfg(not(feature = "diag"))]
    let _ = lines;
}

pub(crate) fn keyboard_is_closed() -> bool {
    keyboard_closed()
}

pub(crate) fn take_typed_name() -> Option<String> {
    crate::runtime::macros::named_macros::take_confirmed_name_for(NAME_TOKEN)
}

fn keyboard_closed() -> bool {
    let mode = crate::runtime::lobby::game_mode::current()
        .map(|mode| mode.mode)
        .unwrap_or(0);
    !crate::model::scene::combo_name_keyboard(crate::runtime::ui::facts::fact_waiting_scene(), mode)
}
