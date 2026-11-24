use serde::{Deserialize, Serialize};

/// One frame of mapped controller input (not a native struct dump).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputFrame {
    pub buttons: u32,
    pub lstick_x: i8,
    pub lstick_y: i8,
    pub rstick_x: i8,
    pub rstick_y: i8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Trim {
    Start,
    End,
    Both,
    #[default]
    Off,
}

impl Trim {
    pub fn window(self, frames: &[InputFrame]) -> core::ops::Range<usize> {
        let start = if matches!(self, Self::Start | Self::Both) {
            frames.iter().position(|frame| !neutral_frame(frame)).unwrap_or(frames.len())
        } else {
            0
        };
        let end = if matches!(self, Self::End | Self::Both) {
            frames.iter().rposition(|frame| !neutral_frame(frame)).map_or(start, |index| index + 1)
        } else {
            frames.len()
        };
        start..end.max(start)
    }

    pub fn apply(self, frames: &mut Vec<InputFrame>) {
        let kept = self.window(frames);
        drop(frames.drain(kept.end..));
        drop(frames.drain(..kept.start));
    }

    pub fn from_row(row: &[u8]) -> Self {
        Self::from_index(row.iter().take(4).position(|value| *value != 0).unwrap_or(3) as u8)
    }

    pub const fn from_index(index: u8) -> Self {
        match index {
            0 => Self::Start,
            1 => Self::End,
            2 => Self::Both,
            _ => Self::Off,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::End => "End",
            Self::Both => "Both",
            Self::Off => "Off",
        }
    }

    pub fn migrate(old: &[u8]) -> Option<[u8; 4]> {
        if old.len() != 2 { return None; }
        Some(if old[0] != 0 { [0, 1, 0, 0] } else { [0, 0, 0, 1] })
    }

    pub fn is_off(&self) -> bool {
        *self == Self::Off
    }
}

pub fn neutral_frame(frame: &InputFrame) -> bool {
    let stick = |value: i8| value.unsigned_abs() < 24;
    (frame.buttons == 0 || frame.buttons == 0x8000)
        && stick(frame.lstick_x)
        && stick(frame.lstick_y)
        && stick(frame.rstick_x)
        && stick(frame.rstick_y)
}
