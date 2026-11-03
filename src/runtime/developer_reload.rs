//! File work for the empty developer configuration, on the existing menu worker.
use crate::model::developer_reload::{Counts, Queue, Reader, OPERATIONS_PER_TURN};
use crate::runtime::{published, ui};

static REQUESTS: Queue<{ u32::MAX }> = Queue::new();
const PATH: &str = "sd:/ultimate/TrainingRooms/dev.json";

pub fn publish(snapshot: &crate::model::settings::Snapshot) {
    use crate::model::developer_ini::{control_enabled, Control};
    REQUESTS.set_enabled(control_enabled(snapshot, published::developer_page(), Control::ReloadConfig));
    ui::worker::wake();
}

/// Existing P1 mapping hook, before menu/chord processing. Only qualified raw input is read; no mutation.
pub unsafe fn on_poll(slot: i64, controller: usize) {
    if !REQUESTS.enabled() || slot != 0 || controller == 0 || controller % 4 != 0 { return; }
    let buttons = ((controller + crate::model::chords::field::CURRENT_BUTTONS) as *const u32).read();
    if crate::model::developer_reload::qualifies(slot, crate::runtime::scene::is_training(), buttons)
        && REQUESTS.request() {
        ui::worker::wake();
    }
}

/// Owned exclusively by the existing menu worker. No game pointers are retained.
#[derive(Default)]
pub struct Worker {
    reader: Reader,
    last_report: u64,
    reported: Counts,
}

impl Worker {
    // Keep the read buffer and diagnostics out of the long-lived menu worker
    // frame: unrelated menu/editor calls must not carry this turn's stack cost.
    #[inline(never)]
    pub fn step(&mut self) -> bool {
        let counts = self.reader.drain(&REQUESTS, PATH);
        let now = crate::runtime::clock::uptime_ms();
        let pending = counts.pending != 0;
        if counts != self.reported && (now.saturating_sub(self.last_report) >= 1000
            || counts.dropped != self.reported.dropped) {
            crate::runtime::diag_log::diagnostic(format_args!(
                "[TrainingRooms] developer_reload requested={} completed={} pending={} dropped={} errors={} bytes={} lag={}",
                counts.requested, counts.completed, counts.pending, counts.dropped, self.reader.errors, self.reader.bytes,
                counts.pending > OPERATIONS_PER_TURN as u64,
            ));
            self.reported = counts;
            self.last_report = now;
        }
        pending || counts != self.reported
    }
}
