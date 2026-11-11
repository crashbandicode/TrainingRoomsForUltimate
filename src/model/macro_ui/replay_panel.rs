//! Controller-operated selection over captured native replay frames. The preview
//! shows the actual recorded controls/state; it is not a rewound game simulation.
use {crate::engine::macros::timestamp, crate::engine::macros::FrameWindow, crate::engine::macros::MacroRecording, crate::engine::macros::ReplaySession, crate::engine::macros::Step};
use crate::engine::macro_ui::macro_panel::{Pad, View};

pub enum Effect {
    None,
    Close,
    Export(Box<MacroRecording>),
    Error(String),
}

pub struct ReplayEditor {
    session: ReplaySession,
    previous: Option<ReplaySession>,
    previous_selected: bool,
    track: usize,
    window: FrameWindow,
    clean_statuses: Vec<i32>,
    cursor: usize,
    step: Step,
    custom_seconds: u32,
}

impl ReplayEditor {
    pub fn new(mut session: ReplaySession, clean_statuses: Vec<i32>) -> Result<Self, String> {
        session.tracks.retain(|t| !t.frames.is_empty());
        session.tracks.sort_by_key(|t| t.entry);
        let window = session.tracks.first().ok_or("Play a saved replay in Vault first")?.window()?;
        Ok(Self { session, previous: None, previous_selected: false,
            track: 0, window, clean_statuses, cursor: 2,
            step: Step::Frame, custom_seconds: 10 })
    }

    /// Keep the preceding observation selectable when Vault automatically loops.
    pub fn with_previous(mut self, previous: Option<ReplaySession>) -> Self {
        self.previous = previous.and_then(|mut session| {
            session.tracks.retain(|track| !track.frames.is_empty());
            session.tracks.sort_by_key(|track| track.entry);
            (!session.tracks.is_empty()).then_some(session)
        });
        self
    }

    pub fn view(&self) -> View {
        let track = &self.session.tracks[self.track];
        let frame = &track.frames[(self.window.cursor() as usize).min(track.frames.len() - 1)];
        let mut labels = ["Prev fighter", "Next fighter", "Back", "Forward",
            "Step: 1 frame", "Step: 1 sec", "Step: 5 sec", "Step: N secs",
            "N - 1", "N + 1", "Mark In", "Mark Out", "Go to In", "Go to Out",
            "In: prior idle/fall", "Name / Export", "Replay start", "Replay end", "", "Cancel",
            "", "", "", "", "", "", "", "", "", "", "", ""]
            .map(String::from).to_vec();
        labels[7] = format!("Step: {} secs", self.custom_seconds);
        if self.previous.is_some() {
            labels[18] = if self.previous_selected { "Use latest capture" } else { "Use previous capture" }.into();
        }
        labels[20] = format!("Frame {}/{}", self.window.cursor(), self.window.total());
        labels[21] = timestamp(self.window.cursor(), self.window.fps());
        labels[22] = format!("P{} / kind {}", track.entry + 1, frame.start.fighter_kind);
        labels[23] = format!("Status {}", frame.start.status);
        labels[24] = format!("Buttons {:X}", frame.input.buttons);
        labels[25] = format!("Stick {},{}", frame.input.lstick_x, frame.input.lstick_y);
        labels[26] = format!("Pos {:.1},{:.1}", frame.x, frame.y);
        labels[27] = format!("Speed {:.2},{:.2}", frame.start.speed_x, frame.start.speed_y);
        labels[28] = format!("In {}", self.window.start());
        labels[29] = format!("Out {}", self.window.end());
        labels[30] = format!("Length {}f", self.window.selected_frames());
        labels[31] = "Input/state preview".into();
        let step = match self.step { Step::Frame => "1 frame".into(), Step::Seconds(n) => format!("{n} secs") };
        let warning = if self.session.truncated { "Capture ended early/10-min limit. " } else { "" };
        let analog = if track.quantized_analog { "Analog rounded to controller steps. " } else { "" };
        View { title: "Replay Window".into(), labels, selected: self.cursor,
            footer: format!("Step {step}. In inclusive, Out exclusive. A selects, B closes.\n{warning}{analog}Mid-action starts need earlier idle/fall lead-in.") }
    }

    pub fn on_pad(&mut self, pad: Pad) -> Effect {
        match pad {
            Pad::Left => self.cursor = self.cursor / 4 * 4 + (self.cursor + 3) % 4,
            Pad::Right => self.cursor = self.cursor / 4 * 4 + (self.cursor + 1) % 4,
            Pad::Up => self.cursor = (self.cursor + 16) % 20,
            Pad::Down => self.cursor = (self.cursor + 4) % 20,
            Pad::Back => return Effect::Close,
            Pad::Extra => { self.window.forward(Step::Frame); }
            Pad::Confirm => return self.confirm(),
            Pad::Rename => {}
        }
        Effect::None
    }

    fn confirm(&mut self) -> Effect {
        match self.cursor {
            0 | 1 => {
                let count = self.session.tracks.len();
                self.track = (self.track + if self.cursor == 0 { count - 1 } else { 1 }) % count;
                // A fighter change keeps exact boundary times where possible.
                let mut window = self.session.tracks[self.track].window().unwrap();
                window.set_out(self.window.end());
                window.set_in(self.window.start());
                window.seek(self.window.cursor());
                self.window = window;
            }
            2 => { self.window.back(self.step); }
            3 => { self.window.forward(self.step); }
            4 => self.step = Step::Frame,
            5 => self.step = Step::SECOND,
            6 => self.step = Step::FIVE_SECONDS,
            7 => self.step = Step::Seconds(self.custom_seconds),
            8 | 9 => {
                self.custom_seconds = if self.cursor == 8 { self.custom_seconds.saturating_sub(1).max(1) }
                    else { (self.custom_seconds + 1).min(600) };
                self.step = Step::Seconds(self.custom_seconds);
            }
            10 => { self.window.set_in(self.window.cursor()); }
            11 => { self.window.set_out(self.window.cursor()); }
            12 => { self.window.seek(self.window.start()); }
            13 => { self.window.seek(self.window.end()); }
            14 => {
                if let Some(frame) = self.session.tracks[self.track]
                    .previous_clean_start(self.window.cursor(), &self.clean_statuses)
                {
                    self.window.set_in(frame);
                    self.window.seek(self.window.start());
                } else {
                    return Effect::Error("No earlier idle/fall frame was captured".into());
                }
            }
            15 => return match self.session.tracks[self.track].export(&self.window,
                &format!("Replay P{}", self.session.tracks[self.track].entry + 1), "13.0.5", &self.clean_statuses)
            {
                Ok(recording) => Effect::Export(Box::new(recording)),
                Err(error) => Effect::Error(error),
            },
            16 => { self.window.seek(0); }
            17 => { self.window.seek(self.window.total()); }
            18 => {
                if let Some(previous) = &mut self.previous {
                    std::mem::swap(&mut self.session, previous);
                    self.previous_selected = !self.previous_selected;
                    self.track = 0;
                    self.window = self.session.tracks[0].window().unwrap();
                }
            }
            19 => return Effect::Close,
            _ => {}
        }
        Effect::None
    }
}

