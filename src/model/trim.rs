use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Trim {
    Start,
    End,
    Both,
    #[default]
    Off,
}

impl Trim {
    pub fn from_row(_row: &[u8]) -> Self {
        Self::Off
    }

    pub fn migrate(_old: &[u8]) -> Option<[u8; 4]> {
        None
    }

    pub fn is_off(&self) -> bool {
        *self == Self::Off
    }
}

impl From<bool> for Trim {
    fn from(on: bool) -> Self {
        if on { Self::End } else { Self::Off }
    }
}
