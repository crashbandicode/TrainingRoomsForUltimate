//! Switch side of the recording worker. Decisions live in `engine::rr_host`, which the host tests compile.

use std::path::Path;

use crate::engine::library::LegacySlot;
use crate::engine::macros::InputFrame;
use crate::engine::rr_host::{self, Tick};
use crate::model::input_delay::MappedInputs;
use crate::model::state::record::SavedState;

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
        return;
    }
    if rr_host::migrated() && !rr_host::pending() {
        return;
    }
    let empty: [Option<LegacySlot>; 5] = [None, None, None, None, None];
    let slots = if rr_host::migrated() {
        empty
    } else {
        match super::recorder::copied_slots() {
            Some(slots) => slots,
            None => return,
        }
    };
    let tick = rr_host::service_at(
        root,
        &slots,
        &rr_host::today(),
        SavedState::never_saved(),
        SavedState::never_saved(),
        keyboard_closed(),
    );
    if rr_host::migrated() {
        super::recorder::release_slot_image();
    }
    apply(&tick);
}

fn apply(tick: &Tick) {
    if tick.warp.is_some() {
        super::states::request_load();
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

fn keyboard_closed() -> bool {
    let mode = crate::runtime::lobby::game_mode::current()
        .map(|mode| mode.mode)
        .unwrap_or(0);
    !crate::model::scene::combo_name_keyboard(crate::runtime::ui::facts::fact_waiting_scene(), mode)
}
