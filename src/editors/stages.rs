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

use crate::engine::stages::custom_stages::{tile_ref, CustomStage, StageOrder, StageRef};
use crate::engine::stages::lobby_stages::{native_forms, LobbyStageOrder, STAGES};
use crate::engine::stages::stage_forms::{Form, Selection};
use crate::engine::stages::stage_menu::{Mode, Preferences};
use crate::model::frame::{Frame, Text};
use crate::model::menu::input::Button;

pub const ORDER_KEY: &str = "lobby_stages";
/// rc.14: the order with custom stages, by save file id (`custom_stages::StageOrder`); `lobby_stages` keeps its catalog projection.
pub const STAGE_ORDER_KEY: &str = "lobby_stage_order";
pub const PREFERENCES_KEY: &str = "stage_preferences";

/// What a press asks of the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Nothing,
    /// The values changed: write them back (`order_value`, `preferences_value`).
    Changed,
    /// Direct mode: go to this stage now (the waiting room's practice stage), and the values changed.
    Request(Selection),
    /// rc.14: go to this Stage Builder stage now (its save file id), and the values changed.
    RequestCustom(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StageEditor {
    /// The order, catalog and custom stages (`lobby_stage_order`, else read from `lobby_stages`).
    order: StageOrder,
    preferences: Preferences,
    /// The save's Stage Builder stages now: the tiles after the catalog.
    customs: Vec<CustomStage>,
}

impl StageEditor {
    /// From the kept values (a missing or malformed one is the native order / the default preferences, as the readers take them).
    pub fn read(order: Option<&Value>, preferences: Option<&Value>) -> StageEditor {
        Self::read_with(None, order, preferences, None)
    }

    /// rc.14: with the order that names custom stages and the save's Stage Builder stages now. Custom stages the save no
    /// longer holds leave the order (`StageOrder::prune`), so the grid and every list agree with Stage Builder. `None`
    /// while the save has not been read: no custom tiles, and the order keeps its custom stages.
    pub fn read_with(stage_order: Option<&Value>, order: Option<&Value>, preferences: Option<&Value>,
        customs: Option<&[CustomStage]>) -> StageEditor {
        let legacy: LobbyStageOrder = order.and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
        let mut order = StageOrder::read(stage_order, legacy);
        if let Some(customs) = customs {
            order.prune(customs);
            order.remember(customs);
        }
        let preferences = preferences.and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
        StageEditor { order, preferences, customs: customs.unwrap_or_default().to_vec() }
    }

    /// The legacy `lobby_stages` value: the catalog projection of the order.
    pub fn order_value(&self) -> Value {
        serde_json::to_value(self.order.legacy()).unwrap_or(Value::Null)
    }

    /// The `lobby_stage_order` value.
    pub fn stage_order_value(&self) -> Value {
        serde_json::to_value(&self.order).unwrap_or(Value::Null)
    }

    pub fn preferences_value(&self) -> Value {
        serde_json::to_value(self.preferences).unwrap_or(Value::Null)
    }

    pub fn mode(&self) -> Mode {
        self.preferences.mode
    }

    fn all(&self) -> bool {
        self.order.all
    }

    /// The stage of grid tile `tile`: a catalog stage, then the save's custom stages (tile 0 is All).
    fn ref_of(&self, tile: usize) -> Option<StageRef> {
        tile_ref(tile, &self.customs)
    }

    fn custom_name(&self, id: u32) -> &str {
        self.customs.iter().find(|s| s.id == id).map_or("", |s| s.name.as_str())
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
            Button::A => match (self.preferences.mode, self.ref_of(tile)) {
                (Mode::Rotation, None) if tile == 0 => {
                    if self.order.select_all() { Effect::Changed } else { Effect::Nothing }
                }
                (Mode::Rotation, Some(stage)) => {
                    if self.order.toggle(stage) {
                        self.order.remember(&self.customs);
                        Effect::Changed
                    } else {
                        Effect::Nothing
                    }
                }
                (Mode::Direct, Some(stage)) => self.request(stage),
                _ => Effect::Nothing,
            },
            _ => Effect::Nothing,
        }
    }

    /// Go to a stage (a catalog stage with its form, or a custom stage): the Direct choice is remembered and the stage
    /// requested.
    fn request(&mut self, stage: StageRef) -> Effect {
        match stage {
            StageRef::Catalog(id) => {
                let Some(index) = STAGES.iter().position(|(s, _)| *s == id) else { return Effect::Nothing };
                let selection = Selection { stage: id, form: self.form_of(index) };
                self.preferences.direct_stage = selection.stage;
                self.preferences.direct_form = selection.form;
                self.order.direct_custom = None;
                self.order.remember(&self.customs);
                Effect::Request(selection)
            }
            StageRef::Custom(id) => {
                self.order.direct_custom = Some(id);
                self.order.remember(&self.customs);
                Effect::RequestCustom(id)
            }
        }
    }

    /// One press on the Change Stage grid (`GridKind::StageChange`, tiles numbered as here): always Direct. Y cycles the stage's
    /// form as on Lobby Stages; A asks to go to the stage with its form. X (the mode switch) does nothing here.
    pub fn change(&mut self, tile: usize, button: Button) -> Effect {
        match (button, self.ref_of(tile)) {
            (Button::Y, Some(StageRef::Catalog(_))) => self.press(tile, button),
            (Button::A, Some(stage)) => self.request(stage),
            _ => Effect::Nothing,
        }
    }

    /// The Change Stage grid's tiles (each stage with its form tag), help and footer.
    pub fn decorate_change(&self, frame: &mut Frame, focused: usize) {
        for i in 0..usize::from(frame.tile_count).min(frame.tiles.len()) {
            if let Some(StageRef::Custom(id)) = self.ref_of(usize::from(frame.tile_n[i])) {
                frame.tiles[i] = custom_label(self.custom_name(id), "");
                continue;
            }
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
            let stage = self.ref_of(n);
            let marker = stage.and_then(|stage| match (self.mode(), stage) {
                (Mode::Rotation, _) if !self.all() => self.order.rank(stage).map(|r| (r, ". ")),
                (Mode::Direct, StageRef::Custom(id)) if self.order.direct_custom == Some(id) => Some((0, "> ")),
                (Mode::Direct, StageRef::Catalog(id))
                    if self.order.direct_custom.is_none() && self.preferences.direct_stage == id => Some((0, "> ")),
                _ => None,
            });
            if let Some(StageRef::Custom(id)) = stage {
                let mark = match marker {
                    Some((0, mark)) => mark.to_owned(),
                    Some((r, mark)) => format!("{r}{mark}"),
                    None => String::new(),
                };
                frame.tiles[i] = custom_label(self.custom_name(id), &mark);
                continue;
            }
            match Self::stage_of(n) {
                None if n == 0 => _ = write!(label, "{}{}", if self.all() && self.mode() == Mode::Rotation { "* " } else { "" }, name.string_lossy()),
                None => continue,
                Some(index) => {
                    match marker {
                        Some((0, mark)) => _ = label.write_str(mark),
                        Some((r, mark)) => _ = write!(label, "{r}{mark}"),
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

/// A custom stage's tile label: its place or the Direct mark, then its name (cut to the tile).
fn custom_label(name: &str, mark: &str) -> Text<28> {
    let mut label = Text::<28>::new();
    let _ = label.write_str(mark);
    for c in name.chars() {
        if label.write_char(c).is_err() {
            break;
        }
    }
    label
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
    lib_unit_tests! {
    use super::*;

    fn index_of(stage: u32) -> usize {
        STAGES.iter().position(|(id, _)| *id == stage).unwrap()
    }

    fn saved_stage_selections_keep_their_stages() {
        let mut legacy = vec![0u8; STAGES.len() + 1];
        legacy[1] = 1;
        legacy[index_of(3) + 1] = 2;
        let legacy = serde_json::Value::Array(legacy.into_iter().map(serde_json::Value::from).collect());
        let modern = serde_json::json!({
            "all": false,
            "entries": [{"stage": 311}, {"stage": 3}]
        });
        let mut editor = StageEditor::read_with(Some(&modern), Some(&legacy), None, None);
        assert_eq!(editor.order_value(), legacy);
        assert_eq!(editor.stage_order_value(), modern);
        assert_eq!(editor.change(2, Button::A), Effect::Request(Selection { stage: 308, form: Form::Normal }));
        assert_eq!(editor.order_value(), legacy, "the legacy catalog ranks do not move");
        assert_eq!(editor.stage_order_value(), modern, "the persisted stage IDs do not move");
    }

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

    fn customs() -> Vec<crate::engine::stages::custom_stages::CustomStage> {
        use crate::engine::stages::custom_stages::CustomStage;
        vec![CustomStage { id: 7, name: "Ledge Lab".into() }, CustomStage { id: 12, name: "Flat".into() },
             CustomStage { id: 40, name: "Walls".into() }]
    }

    fn custom_stages_are_the_last_tiles_of_both_grids_and_deleting_one_leaves_every_list() {
        let first = STAGES.len() + 1;
        let mut e = StageEditor::read_with(None, None, None, Some(&customs()));
        // Rotation: A on the first custom tile adds it as number 2 (after the Waiting Room); Y gives it no form.
        assert_eq!(e.press(first, Button::A), Effect::Changed);
        assert_eq!(e.press(first + 2, Button::A), Effect::Changed);
        assert_eq!(e.press(first, Button::Y), Effect::Nothing, "a custom stage has one form");
        let mut f = Frame::new();
        f.tile_count = 3;
        for i in 0..3 {
            f.tile_n[i] = (first + i) as u16;
        }
        e.decorate(&mut f, first);
        assert_eq!(f.tiles[0].string_lossy(), "2. Ledge Lab");
        assert_eq!(f.tiles[1].string_lossy(), "Flat");
        assert_eq!(f.tiles[2].string_lossy(), "3. Walls");
        // The values the menu keeps: the legacy projection keeps the catalog part; the new key names the custom stages.
        let order = e.stage_order_value();
        assert_eq!(e.order_value(), serde_json::to_value(LobbyStageOrder::NATIVE).unwrap());
        // Stage 7 deleted in Stage Builder: one tile fewer, and it leaves the order; Walls moves up to number 2.
        let after: Vec<_> = customs().into_iter().filter(|s| s.id != 7).collect();
        let e = StageEditor::read_with(Some(&order), Some(&e.order_value()), None, Some(&after));
        let mut f = Frame::new();
        f.tile_count = 2;
        f.tile_n[0] = first as u16;
        f.tile_n[1] = (first + 1) as u16;
        e.decorate(&mut f, first);
        assert_eq!(f.tiles[0].string_lossy(), "Flat");
        assert_eq!(f.tiles[1].string_lossy(), "2. Walls");
        // Before the save has been read (None) nothing is pruned: the order keeps its custom stages.
        let unknown = StageEditor::read_with(Some(&order), Some(&e.order_value()), None, None);
        assert_eq!(unknown.stage_order_value(), order);
        // Direct and Change Stage go to a custom stage by its save file id; a catalog pick clears it again.
        let mut e = StageEditor::read_with(Some(&order), None, None, Some(&customs()));
        assert_eq!(e.press(0, Button::X), Effect::Changed);
        assert_eq!(e.press(first + 1, Button::A), Effect::RequestCustom(12));
        let mut f = Frame::new();
        f.tile_count = 1;
        f.tile_n[0] = (first + 1) as u16;
        e.decorate(&mut f, first + 1);
        assert_eq!(f.tiles[0].string_lossy(), "> Flat");
        assert_eq!(e.change(first + 2, Button::A), Effect::RequestCustom(40));
        assert_eq!(e.change(first + 2, Button::Y), Effect::Nothing);
        assert_eq!(e.change(first + 3, Button::A), Effect::Nothing, "past the last custom stage");
        assert_eq!(e.change(index_of(0) + 1, Button::A), Effect::Request(Selection { stage: 0, form: Form::Normal }));
        let mut f = Frame::new();
        f.tile_count = 1;
        f.tile_n[0] = (first + 2) as u16;
        e.decorate(&mut f, first + 2);
        assert_eq!(f.tiles[0].string_lossy(), "Walls", "the catalog pick took the Direct mark");
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("saved_stage_selections_keep_their_stages", saved_stage_selections_keep_their_stages as fn()),
            ("y_cycles_only_the_forms_the_game_has_and_direct_requests_the_stage_with_its_form", y_cycles_only_the_forms_the_game_has_and_direct_requests_the_stage_with_its_form as fn()),
            ("change_stage_goes_to_the_stage_with_the_form_y_picked_and_keeps_the_rotation", change_stage_goes_to_the_stage_with_the_form_y_picked_and_keeps_the_rotation as fn()),
            ("change_stage_tiles_show_the_form_tag_and_the_footer_shows_the_controls", change_stage_tiles_show_the_form_tag_and_the_footer_shows_the_controls as fn()),
            ("the_tiles_carry_the_order_the_direct_pick_and_the_form_tags", the_tiles_carry_the_order_the_direct_pick_and_the_form_tags as fn()),
            ("custom_stages_are_the_last_tiles_of_both_grids_and_deleting_one_leaves_every_list", custom_stages_are_the_last_tiles_of_both_grids_and_deleting_one_leaves_every_list as fn()),
        ]);
    }
    }
}
