//! The Lobby Stages grid's editing (MENU-DRAFT Fighters & Stages > Stages; SPEC-core-03 §2.5 N5; the carried catalog
//! `engine::stages`). The menu model pages the grid and reports A / X / Y on a tile (`Menu::take_stage_press`); this module applies them
//! to the two kept values the waiting room reads, and says what each tile shows:
//!
//! * the rotation order, kept key `lobby_stages` of the menu snapshot: `[all, rank per catalog stage...]` (`LobbyStageOrder`'s form).
//!   A on All selects every stage; A on a stage adds it at the end of the order, or takes it out (the last one stays); a stage chosen
//!   while All is on becomes the only one (the roster grid's own rank rules, `engine::swap::roster_grid::{select_kind, deselect_kind}`);
//! * the stage preferences, top-level key `stage_preferences` (`stage_menu::Preferences`): Y cycles the focused stage's form among the
//!   forms the game has for it (`Form::next`); X switches Rotation / Direct; in Direct, A picks the stage (and its form) to go to, which
//!   the caller requests from the practice stage (`practice_stage_request`) when the menu closes;
//! * the Change Stage grid (`change`, `decorate_change`): the same tiles without All, always Direct.
//!
//! Until the badge panes exist the order and the form are written into the tile's own label (`3. Battlefield  [BF]`).

use core::fmt::Write;

use serde_json::Value;

use crate::engine::stages::lobby_stages::{native_forms, LobbyStageOrder, STAGES};
use crate::engine::stages::stage_forms::{Form, Selection};
use crate::engine::stages::stage_menu::{Mode, Preferences};
use crate::engine::swap::roster_grid::{deselect_kind, rank, select_kind};
use crate::model::frame::{Frame, Text};
use crate::model::menu::input::Button;

pub const ORDER_KEY: &str = "lobby_stages";
pub const PREFERENCES_KEY: &str = "stage_preferences";

/// What a press asks of the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Nothing,
    /// The values changed: write them back (`order_value`, `preferences_value`).
    Changed,
    /// Direct mode: go to this stage now (the waiting room's practice stage), and the values changed.
    Request(Selection),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StageEditor {
    /// `[all, rank of catalog stage 0, ...]`.
    order: Vec<u8>,
    preferences: Preferences,
}

impl StageEditor {
    /// From the kept values (a missing or malformed one is the native order / the default preferences, as the readers take them).
    pub fn read(order: Option<&Value>, preferences: Option<&Value>) -> StageEditor {
        let typed: LobbyStageOrder = order.and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
        let order = match serde_json::to_value(typed) {
            Ok(Value::Array(values)) => values.iter().map(|v| v.as_u64().unwrap_or(0) as u8).collect(),
            _ => vec![0; STAGES.len() + 1],
        };
        let preferences = preferences.and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
        StageEditor { order, preferences }
    }

    pub fn order_value(&self) -> Value {
        Value::Array(self.order.iter().map(|v| Value::from(*v)).collect())
    }

    pub fn preferences_value(&self) -> Value {
        serde_json::to_value(self.preferences).unwrap_or(Value::Null)
    }

    pub fn mode(&self) -> Mode {
        self.preferences.mode
    }

    fn all(&self) -> bool {
        self.order.first().is_some_and(|v| *v != 0)
    }

    /// The catalog index of grid tile `tile` (tile 0 is All).
    fn stage_of(tile: usize) -> Option<usize> {
        tile.checked_sub(1).filter(|i| *i < STAGES.len())
    }

    fn form_of(&self, index: usize) -> Form {
        self.preferences.rotation_forms.get(index).unwrap_or_default()
    }

    /// One press on `tile`.
    pub fn press(&mut self, tile: usize, button: Button) -> Effect {
        match button {
            Button::X => {
                self.preferences.mode = match self.preferences.mode {
                    Mode::Rotation => Mode::Direct,
                    Mode::Direct => Mode::Rotation,
                };
                Effect::Changed
            }
            Button::Y => {
                let Some(index) = Self::stage_of(tile) else { return Effect::Nothing };
                let form = self.form_of(index);
                let next = form.next(native_forms(STAGES[index].0));
                if next == form || self.preferences.rotation_forms.set(index, next).is_err() {
                    return Effect::Nothing;
                }
                if self.preferences.direct_stage == STAGES[index].0 {
                    self.preferences.direct_form = next;
                }
                Effect::Changed
            }
            Button::A => match (self.preferences.mode, Self::stage_of(tile)) {
                (Mode::Rotation, None) => {
                    if self.all() {
                        return Effect::Nothing;
                    }
                    self.order.iter_mut().for_each(|v| *v = 0);
                    self.order[0] = 1;
                    Effect::Changed
                }
                (Mode::Rotation, Some(index)) => {
                    let changed = if rank(&self.order, index).is_some() { deselect_kind(&mut self.order, index) } else { select_kind(&mut self.order, index) };
                    if changed { Effect::Changed } else { Effect::Nothing }
                }
                (Mode::Direct, None) => Effect::Nothing,
                (Mode::Direct, Some(index)) => {
                    let selection = Selection { stage: STAGES[index].0, form: self.form_of(index) };
                    self.preferences.direct_stage = selection.stage;
                    self.preferences.direct_form = selection.form;
                    Effect::Request(selection)
                }
            },
            _ => Effect::Nothing,
        }
    }

    /// One press on the Change Stage grid (`GridKind::StageChange`, tiles numbered as here): always Direct. Y cycles the stage's
    /// form as on Lobby Stages; A asks to go to the stage with its form. X (the mode switch) does nothing here.
    pub fn change(&mut self, tile: usize, button: Button) -> Effect {
        match (button, Self::stage_of(tile)) {
            (Button::Y, Some(_)) => self.press(tile, button),
            (Button::A, Some(index)) => {
                let selection = Selection { stage: STAGES[index].0, form: self.form_of(index) };
                self.preferences.direct_stage = selection.stage;
                self.preferences.direct_form = selection.form;
                Effect::Request(selection)
            }
            _ => Effect::Nothing,
        }
    }

    /// The Change Stage grid's tiles (each stage with its form tag), help and footer.
    pub fn decorate_change(&self, frame: &mut Frame, focused: usize) {
        for i in 0..usize::from(frame.tile_count).min(frame.tiles.len()) {
            let Some(index) = Self::stage_of(usize::from(frame.tile_n[i])) else { continue };
            if let Some(tag) = self.form_tag(index) {
                let mut label = frame.tiles[i];
                let _ = write!(label, " [{tag}]");
                frame.tiles[i] = label;
            }
        }
        frame.help[1].clear();
        let _ = frame.help[1].write_str("A goes to the stage now and closes the menu. Y picks its form first.");
        frame.hint.clear();
        if let Some(index) = Self::stage_of(focused) {
            let _ = write!(frame.hint, "Form: {}   ", form_name(self.form_of(index)));
        }
        let _ = frame.hint.write_str("A change stage   Y form   L/R page   B back");
    }

    /// The form tag of a stage that has more than one form (`N`, `Ω`, `BF`); none for a single-form stage.
    fn form_tag(&self, index: usize) -> Option<&'static str> {
        let forms = native_forms(STAGES[index].0);
        (forms.iter().filter(|f| f.is_some()).count() > 1).then(|| match self.form_of(index) {
            Form::Normal => "N",
            Form::Omega => "Ω",
            Form::Battlefield => "BF",
        })
    }

    /// Write the order, the direct pick and the form tags into the visible tiles of the frame `screens::grid_frame` built (its
    /// `tile_n`: 0 = All, n = catalog stage n - 1), and the mode and the focused stage's form help into the help panel.
    pub fn decorate(&self, frame: &mut Frame, focused: usize) {
        for i in 0..usize::from(frame.tile_count).min(frame.tiles.len()) {
            let n = usize::from(frame.tile_n[i]);
            let name = frame.tiles[i];
            let mut label = Text::<28>::new();
            match Self::stage_of(n) {
                None if n == 0 => {
                    let _ = write!(label, "{}{}", if self.all() && self.mode() == Mode::Rotation { "* " } else { "" }, name.string_lossy());
                }
                None => continue,
                Some(index) => {
                    let marker = match self.mode() {
                        Mode::Rotation if !self.all() => rank(&self.order, index).map(|r| (r, ". ")),
                        Mode::Direct if self.preferences.direct_stage == STAGES[index].0 => Some((0, "> ")),
                        _ => None,
                    };
                    match marker {
                        Some((0, mark)) => {
                            let _ = label.write_str(mark);
                        }
                        Some((r, mark)) => {
                            let _ = write!(label, "{r}{mark}");
                        }
                        None => {}
                    }
                    let _ = label.write_str(&name.string_lossy());
                    if let Some(tag) = self.form_tag(index) {
                        let _ = write!(label, " [{tag}]");
                    }
                }
            }
            frame.tiles[i] = label;
        }
        frame.help[1].clear();
        let _ = frame.help[1].write_str(match self.mode() {
            Mode::Rotation => "Rotation: A adds or removes a stage (the number is its place), All takes every stage.",
            Mode::Direct => "Direct: A goes to this stage now and closes the menu.",
        });
        frame.hint.clear();
        let form = Self::stage_of(focused).map(|i| form_name(self.form_of(i)));
        let _ = write!(
            frame.hint,
            "{}A {}   Y form   X {}   L/R page   B back",
            form.map_or(String::new(), |f| format!("Form: {f}   ")),
            if self.mode() == Mode::Rotation { "add / remove" } else { "go there" },
            if self.mode() == Mode::Rotation { "direct" } else { "rotation" }
        );
    }
}

fn form_name(form: Form) -> &'static str {
    match form {
        Form::Normal => "Normal",
        Form::Omega => "Omega",
        Form::Battlefield => "Battlefield",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_of(stage: u32) -> usize {
        STAGES.iter().position(|(id, _)| *id == stage).unwrap()
    }

    fn ranks(e: &StageEditor) -> Vec<(usize, u8)> {
        e.order[1..].iter().enumerate().filter(|(_, r)| **r != 0).map(|(i, r)| (i, *r)).collect()
    }

    #[test]
    fn a_missing_order_reads_as_the_native_order_and_writes_back_in_the_files_form() {
        let e = StageEditor::read(None, None);
        assert_eq!(e.order.len(), STAGES.len() + 1);
        assert_eq!(ranks(&e), vec![(0, 1)]);
        let back: LobbyStageOrder = serde_json::from_value(e.order_value()).unwrap();
        assert_eq!(back, LobbyStageOrder::NATIVE);
        assert_eq!(e.mode(), Mode::Rotation);
    }

    #[test]
    fn a_adds_at_the_end_removes_and_compacts_and_keeps_the_last_stage() {
        let mut e = StageEditor::read(None, None);
        assert_eq!(e.press(4, Button::A), Effect::Changed);
        assert_eq!(e.press(2, Button::A), Effect::Changed);
        assert_eq!(ranks(&e), vec![(0, 1), (1, 3), (3, 2)]);
        assert_eq!(e.press(1, Button::A), Effect::Changed, "the first stage leaves; the others move up");
        assert_eq!(ranks(&e), vec![(1, 2), (3, 1)]);
        e.press(4, Button::A);
        assert_eq!(e.press(2, Button::A), Effect::Nothing, "the last stage stays");
        let typed: LobbyStageOrder = serde_json::from_value(e.order_value()).unwrap();
        assert_eq!(typed.stages(), vec![STAGES[1].0]);
    }

    #[test]
    fn all_takes_every_stage_and_a_stage_chosen_from_all_becomes_the_only_one() {
        let mut e = StageEditor::read(None, None);
        assert_eq!(e.press(0, Button::A), Effect::Changed);
        assert!(e.all() && ranks(&e).is_empty());
        let typed: LobbyStageOrder = serde_json::from_value(e.order_value()).unwrap();
        assert_eq!(typed.stages().len(), STAGES.len());
        assert_eq!(e.press(0, Button::A), Effect::Nothing);
        e.press(6, Button::A);
        assert!(!e.all());
        assert_eq!(ranks(&e), vec![(5, 1)]);
    }

    #[test]
    fn y_cycles_only_the_forms_the_game_has_and_direct_requests_the_stage_with_its_form() {
        let mut e = StageEditor::read(None, None);
        let battlefield = index_of(0);
        // Battlefield has Normal and Omega only (native_forms(0) = [0, 1, None]).
        assert_eq!(e.press(battlefield + 1, Button::Y), Effect::Changed);
        assert_eq!(e.form_of(battlefield), Form::Omega);
        e.press(battlefield + 1, Button::Y);
        assert_eq!(e.form_of(battlefield), Form::Normal);
        // A single-form stage keeps Normal.
        let single = (0..STAGES.len()).find(|i| native_forms(STAGES[*i].0).iter().filter(|f| f.is_some()).count() == 1).unwrap();
        assert_eq!(e.press(single + 1, Button::Y), Effect::Nothing);
        assert_eq!(e.press(0, Button::X), Effect::Changed);
        assert_eq!(e.mode(), Mode::Direct);
        e.press(battlefield + 1, Button::Y);
        assert_eq!(e.press(battlefield + 1, Button::A), Effect::Request(Selection { stage: 0, form: Form::Omega }));
        assert_eq!((e.preferences.direct_stage, e.preferences.direct_form), (0, Form::Omega));
        let prefs: Preferences = serde_json::from_value(e.preferences_value()).unwrap();
        assert_eq!(prefs.mode, Mode::Direct);
        assert_eq!(e.press(0, Button::A), Effect::Nothing, "All is not a stage to go to");
    }

    #[test]
    fn change_stage_goes_to_the_stage_with_the_form_y_picked_and_keeps_the_rotation() {
        let mut e = StageEditor::read(None, None);
        let before = e.order_value();
        let battlefield = index_of(0) + 1;
        assert_eq!(e.change(battlefield, Button::A), Effect::Request(Selection { stage: 0, form: Form::Normal }));
        assert_eq!(e.change(battlefield, Button::Y), Effect::Changed, "Y first picks the form");
        assert_eq!(e.change(battlefield, Button::A), Effect::Request(Selection { stage: 0, form: Form::Omega }));
        assert_eq!((e.preferences.direct_stage, e.preferences.direct_form), (0, Form::Omega));
        assert_eq!(e.change(battlefield, Button::X), Effect::Nothing, "no mode switch on Change Stage");
        assert_eq!(e.mode(), Mode::Rotation, "the Lobby Stages mode is untouched");
        assert_eq!(e.change(0, Button::A), Effect::Nothing, "no All tile to go to");
        assert_eq!(e.change(STAGES.len() + 1, Button::A), Effect::Nothing, "past the catalog");
        assert_eq!(e.order_value(), before, "the rotation order is untouched");
    }

    #[test]
    fn change_stage_tiles_show_the_form_tag_and_the_footer_shows_the_controls() {
        let mut e = StageEditor::read(None, None);
        let battlefield = index_of(0) + 1;
        e.change(battlefield, Button::Y);
        let mut f = Frame::new();
        f.tile_count = 2;
        f.tile_n[0] = battlefield as u16;
        f.tile_n[1] = (index_of(0) + 2) as u16;
        let _ = f.tiles[0].write_str(STAGES[battlefield - 1].1);
        e.decorate_change(&mut f, battlefield);
        assert!(f.tiles[0].string_lossy().ends_with(" [Ω]"), "{:?}", f.tiles[0]);
        assert!(f.hint.string_lossy().starts_with("Form: Omega   A change stage   Y form"), "{:?}", f.hint);
        assert!(f.help[1].string_lossy().contains("closes the menu"));
    }

    #[test]
    fn the_tiles_carry_the_order_the_direct_pick_and_the_form_tags() {
        let mut e = StageEditor::read(None, None);
        e.press(4, Button::A);
        let mut f = Frame::new();
        f.tile_count = 5;
        for i in 0..5 {
            f.tile_n[i] = i as u16;
            let _ = f.tiles[i].write_str(if i == 0 { "All" } else { STAGES[i - 1].1 });
        }
        e.decorate(&mut f, 1);
        assert_eq!(f.tiles[0].string_lossy(), "All");
        assert!(f.tiles[1].string_lossy().starts_with("1. "), "{:?}", f.tiles[1]);
        assert!(f.tiles[4].string_lossy().starts_with("2. "), "{:?}", f.tiles[4]);
        assert!(f.hint.string_lossy().contains("add / remove"));
        e.press(0, Button::A);
        e.decorate(&mut f, 0);
        assert!(f.tiles[0].string_lossy().starts_with("* All"), "{:?}", f.tiles[0]);
    }
}
