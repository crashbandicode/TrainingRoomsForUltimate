//! Eden-only control of the named-macro layer (diag build): the menu's Named library, Save slot to library and Replay window screens are
//! package 5's to draw, so the rows that prove the layer (a slot saved to the library, a named macro loaded and played, a capture listed and
//! cropped) are driven by one request file, `sd:/ultimate/TrainingRooms/diag-macro.req`, one command per line, read by a small thread and
//! executed on the game thread (the CPU body's frame), with one `[NamedMacroDiag]` result line per command.
//!
//! Commands: `save_slot <n> <name>` (export slot n, offer it to the panel, confirm the name: the keyboard's confirmed path),
//! `open_library <n>`, `pad <left|right|up|down|confirm|back|extra>`, `view`, `status`, `name <text>`, `replay_window`,
//! `clip <in|out|play|restart|prev|next|capture|leadin|confirm|cancel|nudge N>`, `list_captures`, `restart_capture`.

use std::sync::atomic::{AtomicBool, Ordering};

use hos_lock::Mutex;

use crate::engine::macro_ui::clip_panel::ClipAction;
use crate::engine::macro_ui::macro_panel::Pad;
use crate::model::storage::FixedPath;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::macros::{named_macros, replay_capture, replay_store};

const MAX_TEXT: usize = 96;

/// One parsed command, small and `Copy` so the hand-off allocates nothing.
#[derive(Clone, Copy)]
struct Command {
    kind: u8,
    number: i32,
    text: [u8; MAX_TEXT],
    len: usize,
}

static PENDING: Mutex<Option<Command>> = Mutex::new(None);
static STARTED: AtomicBool = AtomicBool::new(false);
/// The layer's step in progress (`macros::step`) and the count of CPU-body frames the layer finished: the watchdog's two facts.
pub(crate) static STEP: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);
static FRAMES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// One CPU-body frame of the layer finished.
pub(crate) fn frame_done() {
    FRAMES.fetch_add(1, Ordering::Relaxed);
}

const SAVE_SLOT: u8 = 1;
const OPEN_LIBRARY: u8 = 2;
const PAD: u8 = 3;
const VIEW: u8 = 4;
const STATUS: u8 = 5;
const NAME: u8 = 6;
const REPLAY_WINDOW: u8 = 7;
const CLIP: u8 = 8;
const LIST_CAPTURES: u8 = 9;
const EXPORT_SLOT: u8 = 10;

fn parse(line: &str) -> Option<Command> {
    let mut words = line.split_whitespace();
    let verb = words.next()?;
    let mut command = Command { kind: 0, number: 0, text: [0; MAX_TEXT], len: 0 };
    let set_text = |text: &str, command: &mut Command| {
        let bytes = text.as_bytes();
        let n = bytes.len().min(MAX_TEXT);
        command.text[..n].copy_from_slice(&bytes[..n]);
        command.len = n;
    };
    match verb {
        "save_slot" => {
            command.kind = SAVE_SLOT;
            command.number = words.next()?.parse().ok()?;
            let rest: Vec<&str> = words.collect();
            set_text(&rest.join(" "), &mut command);
        }
        "export_slot" => {
            command.kind = EXPORT_SLOT;
            command.number = words.next()?.parse().ok()?;
        }
        "open_library" => {
            command.kind = OPEN_LIBRARY;
            command.number = words.next()?.parse().ok()?;
        }
        "pad" | "clip" => {
            command.kind = if verb == "pad" { PAD } else { CLIP };
            set_text(words.next()?, &mut command);
            command.number = words.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        }
        "view" => command.kind = VIEW,
        "status" => command.kind = STATUS,
        "name" => {
            command.kind = NAME;
            let rest: Vec<&str> = words.collect();
            set_text(&rest.join(" "), &mut command);
        }
        "replay_window" => command.kind = REPLAY_WINDOW,
        "list_captures" => command.kind = LIST_CAPTURES,
        _ => return None,
    }
    Some(command)
}

/// The thread that reads the request file (the only thing it does): the commands run on the game thread.
pub(crate) fn start() {
    if STARTED.swap(true, Ordering::AcqRel) {
        return;
    }
    let _ = crate::runtime::threads::spawn(crate::model::thread_stacks::ThreadId::MacroDiag, || {
        // The stack this thread runs on (its size is the stack table's row), so a fault report (an SP below a stack bottom) can be
        // attributed to a thread.
        let here = 0u8;
        diagnostic(format_args!("[NamedMacroDiag] thread tr-macro-diag stack_near={:p}", &here));
        let (mut last, mut still, mut told) = (0u64, 0u32, false);
        loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
        // Watchdog: the layer's frames stopped while they were counting (a stalled game thread): say once where.
        let now = FRAMES.load(Ordering::Relaxed);
        if now != 0 && now == last {
            still += 1;
            if still == 10 && !told {
                told = true;
                diagnostic(format_args!("[NamedMacroDiag] stall frames={now} step={}", STEP.load(Ordering::Relaxed)));
            }
        } else {
            still = 0;
            told = false;
        }
        last = now;
        let Some(path) = FixedPath::<128>::in_root("diag-macro.req") else { continue };
        let Ok(text) = std::fs::read_to_string(path.as_str()) else { continue };
        let _ = std::fs::remove_file(path.as_str());
        for line in text.lines() {
            let Some(command) = parse(line.trim()) else {
                diagnostic(format_args!("[NamedMacroDiag] unknown_command line={line:?}"));
                continue;
            };
            // Wait for the game thread to take the previous command.
            for _ in 0..100 {
                if PENDING.lock().is_none() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            *PENDING.lock() = Some(command);
        }
        }
    });
}

fn pad_of(name: &str) -> Option<Pad> {
    Some(match name {
        "left" => Pad::Left,
        "right" => Pad::Right,
        "up" => Pad::Up,
        "down" => Pad::Down,
        "confirm" => Pad::Confirm,
        "back" => Pad::Back,
        "extra" => Pad::Extra,
        _ => return None,
    })
}

fn clip_of(name: &str, number: i32) -> Option<ClipAction> {
    Some(match name {
        "in" => ClipAction::SelectIn,
        "out" => ClipAction::SelectOut,
        "play" => ClipAction::TogglePlay,
        "restart" => ClipAction::Restart,
        "prev" => ClipAction::PreviousFighter,
        "next" => ClipAction::NextFighter,
        "capture" => ClipAction::ToggleCapture,
        "leadin" => ClipAction::UseLeadIn,
        "confirm" => ClipAction::Confirm,
        "cancel" => ClipAction::Cancel,
        "nudge" => ClipAction::Nudge(number),
        _ => return None,
    })
}

/// Run the pending command, if any (the CPU body's frame, the game thread). A command is an event (a save, a panel step): its allocations
/// are the per-load kind of the hook counter (hook_stats::per_load), apart from the steady-state ones that must stay 0.
pub(crate) fn run_pending() {
    let Some(command) = PENDING.lock().take() else { return };
    crate::model::hook_stats::per_load(|| run(command));
}

fn run(command: Command) {
    let text = std::str::from_utf8(&command.text[..command.len]).unwrap_or("");
    match command.kind {
        SAVE_SLOT => {
            named_macros::export_record_slot(usize::try_from(command.number).ok());
            named_macros::diag_confirm_name(text);
            diagnostic(format_args!("[NamedMacroDiag] save_slot slot={} name={text:?}", command.number));
        }
        EXPORT_SLOT => {
            named_macros::diag_export_slot(usize::try_from(command.number).unwrap_or(0));
            diagnostic(format_args!("[NamedMacroDiag] export_slot slot={}", command.number));
        }
        OPEN_LIBRARY => {
            named_macros::open_library(usize::try_from(command.number).ok());
            diagnostic(format_args!("[NamedMacroDiag] open_library slot={}", command.number));
        }
        PAD => match pad_of(text) {
            Some(pad) => {
                named_macros::on_pad(pad);
                diagnostic(format_args!("[NamedMacroDiag] pad {text}"));
            }
            None => diagnostic(format_args!("[NamedMacroDiag] unknown_pad {text:?}")),
        },
        CLIP => match clip_of(text, command.number) {
            Some(action) => {
                replay_capture::on_action(action);
                diagnostic(format_args!("[NamedMacroDiag] clip {text} {}", command.number));
            }
            None => diagnostic(format_args!("[NamedMacroDiag] unknown_clip {text:?}")),
        },
        NAME => {
            named_macros::diag_confirm_name(text);
            diagnostic(format_args!("[NamedMacroDiag] name {text:?}"));
        }
        REPLAY_WINDOW => {
            replay_capture::open_editor();
            diagnostic(format_args!("[NamedMacroDiag] replay_window opened"));
        }
        VIEW => match named_macros::panel_view() {
            Some(view) => diagnostic(format_args!(
                "[NamedMacroDiag] view title={:?} footer={:?} selected={} labels={:?}",
                view.title, view.footer, view.selected, view.labels
            )),
            None => diagnostic(format_args!("[NamedMacroDiag] view none visible={}", named_macros::panel_visible())),
        },
        LIST_CAPTURES => match replay_store::archive().list() {
            Ok(listing) => {
                diagnostic(format_args!("[NamedMacroDiag] captures count={} skipped={}", listing.entries.len(), listing.skipped.len()));
                for entry in &listing.entries {
                    diagnostic(format_args!("[NamedMacroDiag] capture id={} name={:?} frames={} truncated={}", entry.id, entry.replay_name, entry.frames, entry.truncated));
                }
            }
            Err(error) => diagnostic(format_args!("[NamedMacroDiag] captures error={error}")),
        },
        STATUS => {
            diagnostic(format_args!(
                "[NamedMacroDiag] status named_source={} panel={} browser={} editor={} capture_armed={} capture_ready={} recorder={:?} cpu_kind={:?}",
                named_macros::source_is_named(),
                named_macros::panel_visible(),
                replay_capture::browser_visible(),
                replay_capture::panel_visible(),
                replay_store::toggle().is_armed(),
                replay_store::toggle().is_ready(),
                crate::runtime::features::recorder::phase(),
                crate::runtime::features::recorder::cpu_kind()
            ));
        }
        _ => {}
    }
}
