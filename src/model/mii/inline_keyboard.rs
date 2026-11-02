//! Inline keyboard value validation and callback decisions, without SDK pointers.
//! The native adapter consumes these same values before and after Calc/Finalize.
use super::draft::MAX_NAME_CHARS;
use std::convert::TryFrom;

const MII_UNITS: usize = 10;
const MACRO_UNITS: usize = MAX_NAME_CHARS * 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Limit {
    #[default]
    Mii,
    Macro,
}

impl Limit {
    pub fn units(self) -> usize { match self { Self::Mii => MII_UNITS, Self::Macro => MACRO_UNITS } }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text {
    units: [u16; MACRO_UNITS + 1],
    len: usize,
    cursor: usize,
    limit: Limit,
}

impl Text {
    // Keep the old Mii constructor/ten-unit contract at the compatibility path.
    #[cfg(any(test, feature = "mii-editor-alpha"))]
    pub fn new(units: &[u16], cursor: usize) -> Option<Self> {
        Self::with_limit(units, cursor, Limit::Mii)
    }

    pub fn macro_name(name: &str, cursor: usize) -> Option<Self> {
        Self::with_limit(&name.encode_utf16().collect::<Vec<_>>(), cursor, Limit::Macro)
    }

    fn with_limit(units: &[u16], cursor: usize, limit: Limit) -> Option<Self> {
        if units.len() > limit.units() || cursor > units.len() || units.contains(&0) {
            return None;
        }
        if limit == Limit::Macro {
            let name = String::from_utf16(units).ok()?;
            if name.chars().count() > MAX_NAME_CHARS || name.chars().any(char::is_control)
                || String::from_utf16(&units[..cursor]).is_err() { return None; }
        }
        let mut text = Self { units: [0; MACRO_UNITS + 1], len: units.len(), cursor, limit };
        text.units[..units.len()].copy_from_slice(units);
        Some(text)
    }

    pub fn units(&self) -> &[u16] { &self.units[..self.len] }
    // Mii's native config must still receive exactly 11 units/22 bytes.
    pub fn terminated(&self) -> &[u16] { &self.units[..self.limit.units() + 1] }
    pub fn cursor(&self) -> usize { self.cursor }
    pub fn limit(&self) -> Limit { self.limit }
}

#[derive(Clone, Copy, Debug)]
// One fixed-size callback result, bounded to 96 UTF-16 units. Keeping the text
// inline avoids allocating/freeing in native callbacks or retaining heap owners
// across Finalize; the unused Cancel payload costs only this small stack value.
pub enum Decision {
    Enter(Text),
    Cancel,
}

#[derive(Default)]
pub struct Signals {
    pub limit: Limit,
    pub text: Option<Text>,
    pub decision: Option<Decision>,
    pub initialized: bool,
    pub finished: bool,
    pub invalid: bool,
}

pub fn text_signal(units: &[u16], len: i32, cursor: i32, limit: Limit) -> Option<Text> {
    let len = usize::try_from(len).ok()?;
    let cursor = usize::try_from(cursor).ok()?;
    Text::with_limit(units.get(..len)?, cursor, limit)
}

impl Signals {
    pub fn change(&mut self, units: &[u16], len: i32, cursor: i32) {
        let text = text_signal(units, len, cursor, self.limit);
        self.invalid |= text.is_none();
        if text.is_some() { self.text = text; }
    }

    pub fn enter(&mut self, units: &[u16], len: i32) {
        let text = text_signal(units, len, len, self.limit);
        self.invalid |= text.is_none();
        if let Some(text) = text {
            self.text = Some(text);
            self.decision = Some(if text.limit == Limit::Mii && text.len == 0 {
                Decision::Cancel
            } else { Decision::Enter(text) });
        }
    }

    pub fn cancel(&mut self) { self.decision = Some(Decision::Cancel); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mii_limit_and_native_buffer_size_are_unchanged() {
        let text = Text::new(&[65; 10], 4).unwrap();
        assert_eq!(text.terminated().len(), 11);
        assert_eq!(std::mem::size_of_val(text.terminated()), 22);
        assert!(Text::new(&[65; 11], 0).is_none());
        assert!(Text::new(&[65], 2).is_none());
        assert!(Text::new(&[0], 0).is_none());
    }

    #[test]
    fn macro_limit_counts_scalars_and_preserves_utf16_caret() {
        let name = "😀".repeat(MAX_NAME_CHARS);
        let text = Text::macro_name(&name, 2).unwrap();
        assert_eq!(text.units().len(), 96);
        assert_eq!(text.cursor(), 2);
        assert!(Text::macro_name(&format!("{name}a"), 0).is_none());
        assert!(Text::macro_name(&"a".repeat(49), 0).is_none());
        assert!(Text::macro_name("😀", 1).is_none());
        assert!(text_signal(&[0xd800], 1, 1, Limit::Macro).is_none());
        assert!(text_signal(&[65], -1, 0, Limit::Macro).is_none());
    }
}
