//! rc.14 Break the Targets (`artifacts/rc14-eggsercise/SPEC.md` §V2.5, acceptance rows 26-33, 41-43 and 45 (file errors); rows 31, 41 and 42
//! also have an Eden row).

lib_unit_tests! {
use super::*;
use crate::model::chords::logical::{A, B, DPAD_DOWN, DPAD_LEFT, DPAD_RIGHT, DPAD_UP, L, X, Y, ZL};
use crate::model::eggs::{self, Hit, View};
use crate::model::menu::table::{Control, GROUPS, ROWS};
use crate::model::panes::EggPicture;

/// How many of `picture` the mode shows now.
fn count(b: &Btt, picture: EggPicture) -> usize {
    b.look().iter().flatten().filter(|l| l.1.picture == picture).count()
}

fn bits(keys: &[u8]) -> u16 {
    keys.iter().fold(0, |set, k| set | 1 << k)
}

/// The fallback placing box on a ground at 0: x ±90, y 0 to 80.
fn fallback() -> [[f32; 2]; 2] {
    bounds(None, None, 0.0)
}

/// One P1 controller driving the model a poll at a time: `pressed` is the edge of `held`.
struct Sim {
    btt: Btt,
    was: u16,
}

impl Sim {
    /// Stage 10, the menu closed with the mode On and Place targets chosen.
    fn placing() -> Sim {
        let mut btt = Btt::IDLE;
        btt.enter(10);
        btt.close_menu(true, true);
        Sim { btt, was: 0 }
    }

    fn frame(&mut self, keys: &[u8], stick: [f32; 2]) -> bool {
        let held = bits(keys);
        let pad = Pad { stick, held, pressed: held & !self.was, ..Pad::default() };
        self.was = held;
        self.btt.feed(pad)
    }

    /// Buttons down together for one poll, then up.
    fn tap(&mut self, keys: &[u8]) {
        self.frame(keys, [0.0; 2]);
        self.frame(&[], [0.0; 2]);
    }

    fn idle(&mut self, frames: u32) {
        for _ in 0..frames {
            self.frame(&[], [0.0; 2]);
        }
    }

    /// The stick held for `frames` polls, then let go.
    fn push(&mut self, stick: [f32; 2], frames: u32) {
        for _ in 0..frames {
            self.frame(&[], stick);
        }
        self.frame(&[], [0.0; 2]);
    }

    /// The Save State Save or Load chord (ZL + D-pad Down / Up by default) passing on one poll.
    fn chord(&mut self, save: bool) {
        let held = bits(&[ZL, if save { DPAD_DOWN } else { DPAD_UP }]);
        let pad = Pad { held, pressed: held & !self.was, save, load: !save, ..Pad::default() };
        self.was = held;
        self.btt.feed(pad);
        self.frame(&[], [0.0; 2]);
    }

    /// ZL + D-pad Right (or Left): the next (or the last) slot.
    fn slot(&mut self, right: bool) {
        self.tap(&[ZL, if right { DPAD_RIGHT } else { DPAD_LEFT }]);
    }

    /// The cursor moved to (`x`, `y`) with the stick, then A.
    fn place_at(&mut self, x: f32, y: f32) {
        let [cx, cy] = self.btt.cursor().unwrap();
        self.push([(x - cx).signum(), 0.0], ((x - cx).abs() / 2.0).round() as u32);
        self.push([0.0, (y - cy).signum()], ((y - cy).abs() / 2.0).round() as u32);
        self.tap(&[A]);
    }

    fn text(&self) -> String {
        let mut text = String::new();
        self.btt.write_notice(&mut text).unwrap();
        text
    }
}

fn hit_at(at: [f32; 2]) -> Hit {
    Hit([at[0], at[1], 0.0], [at[0], at[1], 0.0], 3.0, 0.0)
}

/// Three targets, 20 apart, started: Ready.
fn ready_with_three() -> Sim {
    let mut s = Sim::placing();
    for x in [-20.0, 0.0, 20.0] {
        s.place_at(x, 15.0);
    }
    s.tap(&[X]);
    s
}

/// One P1 pass: the clock ticks, `hits` against the targets, rule Any attack; the spots that broke.
fn pass(btt: &mut Btt, hits: &[Hit]) -> Vec<[f32; 2]> {
    let mut broke = vec![];
    btt.pass(true, |at| {
        let hit = hits.iter().any(|h| eggs::breaks(eggs::ANY, false, h, at));
        hit.then(|| broke.push(at));
        hit
    });
    broke
}

/// Three seconds of Done: the run is over and Ready is back.
fn countdown(btt: &mut Btt) {
    for _ in 0..180 {
        pass(btt, &[]);
    }
    assert_eq!(btt.phase(), Phase::Ready);
}

/// A whole run of three targets in `frames` game frames: the first input, then the targets on the last three passes.
fn run_three(s: &mut Sim, frames: u32) {
    s.tap(&[A]);
    assert_eq!(s.btt.phase(), Phase::Running);
    for _ in 0..frames - 3 {
        pass(&mut s.btt, &[]);
    }
    for x in [-20.0, 0.0, 20.0] {
        assert_eq!(pass(&mut s.btt, &[hit_at([x, 15.0])]), [[x, 15.0]]);
    }
}

fn btt_rows_and_help() {
    let at = |id: &str| ROWS.iter().position(|r| r.id == id).unwrap_or_else(|| panic!("row {id}"));
    let (on, place, layouts) = (at(ROW), at(ROW_PLACE), at(ROW_LAYOUTS));
    // V2.7: in Practice games, after the Eggs-ercise rows, in V2.5's order.
    let g = usize::from(ROWS[on].group);
    assert_eq!(GROUPS[g].title, "Practice games");
    assert!(ROWS[place].group == ROWS[on].group && ROWS[layouts].group == ROWS[on].group);
    assert!(at(eggs::ROW_SPEED) < on && on < place && place < layouts);
    assert_eq!((ROWS[on].title, ROWS[place].title, ROWS[layouts].title), ("Break the Targets", "Place targets", "Saved layouts"));
    assert!(matches!(ROWS[on].control, Control::Dropdown(["Off", "On"])));
    assert_eq!(
        ROWS[on].help_lines(),
        ("Place targets, then break them all against the clock.", "Closing the menu with no targets opens the placing view.")
    );
    assert_eq!(
        ROWS[place].help_lines(),
        ("Close the menu into the placing view, stage zoomed out.", "Stick moves, A places, B removes the last, X starts.")
    );
    assert_eq!(
        ROWS[layouts].help_lines(),
        ("This stage's 5 saved target layouts: load or delete.", "While placing: ZL + Down saves, ZL + Up loads the slot.")
    );
    for r in [&ROWS[on], &ROWS[place], &ROWS[layouts]] {
        let (a, b) = r.help_lines();
        assert!(a.chars().count() <= 67 && b.chars().count() <= 67, "{}", r.title);
    }
    let def = crate::model::factory::DEFS.iter().find(|d| d.id == ROW).expect("factory row");
    assert_eq!(def.default, &[1, 0][..], "Off by default");
}

/// Row 26: A places at the cursor; an 11th or an overlapping target is refused; B on release removes the last; B + D-pad Up
/// removes nothing; Y twice clears.
fn btt_placing_inputs() {
    let mut s = Sim::placing();
    assert_eq!((s.btt.phase(), s.btt.cursor()), (Phase::Placing, Some([0.0, 15.0])), "the stage centre, one fighter up");
    assert_eq!(s.text(), "Place targets 0/10   A place  B undo  X start");
    assert!(s.frame(&[], [0.0; 2]), "Placing keeps P1's controller from the game");
    s.tap(&[A]);
    assert_eq!(s.btt.layout(), [[0.0, 15.0]]);
    // The same spot again overlaps.
    s.tap(&[A]);
    assert_eq!((s.btt.layout().len(), s.text()), (1, "Too close to another target".to_owned()));
    // 12 units on is clear, and the stick moves the cursor 2 a frame at full tilt.
    s.push([1.0, 0.0], 6);
    assert_eq!(s.btt.cursor(), Some([12.0, 15.0]));
    s.tap(&[A]);
    assert_eq!((s.btt.layout().len(), s.text()), (2, "Place targets 2/10   A place  B undo  X start".to_owned()));
    for (x, y) in [(-80.0, 15.0), (-40.0, 15.0), (40.0, 15.0), (80.0, 15.0), (-80.0, 55.0), (-40.0, 55.0), (0.0, 55.0), (40.0, 55.0)] {
        s.place_at(x, y);
    }
    assert_eq!(s.btt.layout().len(), 10);
    s.push([1.0, 0.0], 6);
    s.tap(&[A]);
    assert_eq!((s.btt.layout().len(), s.text()), (10, "10 targets: B removes the last".to_owned()), "an 11th is refused");
    // B acts on release.
    s.frame(&[B], [0.0; 2]);
    assert_eq!(s.btt.layout().len(), 10, "not on press");
    s.frame(&[], [0.0; 2]);
    assert_eq!(s.btt.layout().len(), 9);
    // B + D-pad Up (the menu's chord) removes nothing; neither does B with another button pressed while it is held.
    s.frame(&[B, DPAD_UP], [0.0; 2]);
    s.frame(&[], [0.0; 2]);
    assert_eq!(s.btt.layout().len(), 9);
    s.frame(&[B], [0.0; 2]);
    s.frame(&[B, L], [0.0; 2]);
    s.frame(&[], [0.0; 2]);
    assert_eq!(s.btt.layout().len(), 9, "B then another button is not an undo");
    // Y twice within a second clears; a second Y after the question lapsed asks again.
    s.tap(&[Y]);
    assert_eq!((s.btt.layout().len(), s.text()), (9, "Y again to clear all".to_owned()));
    s.idle(70);
    s.tap(&[Y]);
    assert_eq!(s.btt.layout().len(), 9);
    s.tap(&[Y]);
    assert_eq!(s.btt.layout().len(), 0);
    // The D-pad: a unit a press; held, it repeats after 15 frames and then every 2.
    let x0 = s.btt.cursor().unwrap()[0];
    s.tap(&[DPAD_RIGHT]);
    assert_eq!(s.btt.cursor().unwrap()[0], x0 + 1.0);
    for _ in 0..20 {
        s.frame(&[DPAD_RIGHT], [0.0; 2]);
    }
    s.frame(&[], [0.0; 2]);
    assert_eq!(s.btt.cursor().unwrap()[0], x0 + 1.0 + 4.0, "frames 0, 15, 17 and 19");
    // A stick inside the dead zone does not move it.
    let at = s.btt.cursor();
    s.push([0.1, -0.1], 30);
    assert_eq!(s.btt.cursor(), at);
}

/// Row 27: the cursor stays in the placing box at full tilt in every direction; no stage read: ±90 and 0 to 80 above the
/// ground; with the stage's camera range it is cut to what the camera shows.
fn btt_cursor_clamp() {
    assert_eq!(fallback(), [[-90.0, 90.0], [0.0, 80.0]]);
    assert_eq!(bounds(None, None, 12.0)[1], [12.0, 92.0], "above the ground");
    // Battlefield in the fixed camera (Eden d2-btt-cam-1): 6.89 x and 6.80 y canvas units a world unit, world (0, 0) at
    // canvas (0, -172.26); its camera range [-170, 170, 130, -80]. On screen: x ±139.3, y -54.1 to 104.7.
    let view = View([6.89, 6.80], [0.0, -172.26], [-170.0, 170.0]);
    let range = Some([-170.0, 170.0, 130.0, -80.0]);
    let b = bounds(range, Some(view), 0.0);
    let near = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 0.1 && (a[1] - b[1]).abs() < 0.1;
    assert!(near(b[0], [-135.3, 135.3]), "the view less a target's radius, inside the range: {:?}", b[0]);
    assert!(near(b[1], [-10.0, 100.7]), "10 under the ground up to the view's top: {:?}", b[1]);
    // A view wider than the range leaves the range.
    let wide = bounds(Some([-100.0, 100.0, 60.0, -80.0]), Some(View([2.0, 2.0], [0.0, 0.0], [-100.0, 100.0])), 0.0);
    assert_eq!(wide, [[-100.0, 100.0], [-10.0, 60.0]]);
    for (range, view) in [(None, None), (range, Some(view))] {
        let (mut s, b) = (Sim::placing(), bounds(range, view, 0.0));
        s.btt.sense(range, 0.0);
        s.btt.see(view);
        s.btt.close_menu(true, true);
        for stick in [[1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0], [1.0, 1.0], [-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0]] {
            s.push(stick, 300);
            let [x, y] = s.btt.cursor().unwrap();
            assert!(x >= b[0][0] && x <= b[0][1] && y >= b[1][0] && y <= b[1][1], "{stick:?} left the box: {x} {y}");
        }
    }
}
/// Row 28: X with no targets is refused; with one or more it goes to Ready at 0:00.00; the clock starts on the first input,
/// not before.
fn btt_ready_waits_for_input() {
    let mut s = Sim::placing();
    s.tap(&[X]);
    assert_eq!((s.btt.phase(), s.text()), (Phase::Placing, "Place at least one target".to_owned()));
    for x in [-20.0, 0.0, 20.0] {
        s.place_at(x, 15.0);
    }
    s.tap(&[X]);
    assert_eq!((s.btt.phase(), s.text()), (Phase::Ready, "Targets 0/3   0:00.00".to_owned()));
    assert!(!s.frame(&[], [0.0; 2]), "Ready gives P1's controller back");
    for _ in 0..200 {
        pass(&mut s.btt, &[]);
        s.frame(&[], [0.0; 2]);
    }
    assert_eq!((s.btt.phase(), s.text()), (Phase::Ready, "Targets 0/3   0:00.00".to_owned()), "the clock waits");
    // A stick inside the dead zone is not an input; a button is.
    s.frame(&[], [0.1, 0.1]);
    assert_eq!(s.btt.phase(), Phase::Ready);
    s.frame(&[X], [0.0; 2]);
    assert_eq!(s.btt.phase(), Phase::Running);
    for _ in 0..60 {
        pass(&mut s.btt, &[]);
    }
    assert_eq!(s.text(), "Targets 0/3   0:01.00", "a second of game frames, in hundredths");
    // A stick out of its dead zone is an input, but one still tilted from the cursor when X was pressed is not.
    let mut s = Sim::placing();
    s.place_at(0.0, 15.0);
    s.frame(&[X], [1.0, 0.0]);
    s.frame(&[], [1.0, 0.0]);
    assert_eq!(s.btt.phase(), Phase::Ready, "the tilt was there before");
    s.frame(&[], [0.0; 2]);
    s.frame(&[], [1.0, 0.0]);
    assert_eq!(s.btt.phase(), Phase::Running);
}

/// Row 29: breaking the last target stops the clock at that frame; the best is per layout; Ready again
/// with every target back.
fn btt_run_and_best() {
    let mut s = ready_with_three();
    s.tap(&[A]);
    for _ in 0..30 {
        pass(&mut s.btt, &[]);
    }
    assert_eq!(pass(&mut s.btt, &[hit_at([-20.0, 15.0])]), [[-20.0, 15.0]]);
    assert_eq!(s.text(), "Targets 1/3   0:00.51", "31 frames in");
    // A hit that misses breaks nothing; a hit on two targets breaks both and tells the runtime twice (the hitlag).
    assert!(pass(&mut s.btt, &[hit_at([5.0, 40.0])]).is_empty());
    let both = [Hit([0.0, 15.0, 0.0], [20.0, 15.0, 0.0], 3.0, 0.0)];
    assert_eq!(pass(&mut s.btt, &both), [[0.0, 15.0], [20.0, 15.0]]);
    // The clock stopped on the break frame: 33 frames, 0:00.55, and nothing counts on.
    assert_eq!((s.btt.phase(), s.text()), (Phase::Done, "Done! 3 in 0:00.55 (best 0:00.55)".to_owned()));
    for _ in 0..100 {
        pass(&mut s.btt, &[]);
        assert_eq!((s.btt.phase(), s.text()), (Phase::Done, "Done! 3 in 0:00.55 (best 0:00.55)".to_owned()));
    }
    // After 3 s: Ready again, every target back, clock 0.
    for _ in 0..80 {
        pass(&mut s.btt, &[]);
    }
    assert_eq!((s.btt.phase(), s.text(), count(&s.btt, EggPicture::Whole)), (Phase::Ready, "Targets 0/3   0:00.00".to_owned(), 3));
    // A faster run on the same layout replaces the best; a slower one does not.
    run_three(&mut s, 30);
    assert_eq!(s.text(), "Done! 3 in 0:00.50 (best 0:00.50)");
    countdown(&mut s.btt);
    run_three(&mut s, 90);
    assert_eq!(s.text(), "Done! 3 in 0:01.50 (best 0:00.50)");
    countdown(&mut s.btt);
    // A pause (no tick) holds the clock.
    s.tap(&[A]);
    for _ in 0..50 {
        s.btt.pass(false, |_| false);
    }
    assert_eq!(s.text(), "Targets 0/3   0:00.00");
    // Changing the layout starts a new best: a fourth target shows only its own time.
    s.btt.close_menu(true, true);
    s.place_at(40.0, 15.0);
    s.tap(&[X]);
    s.tap(&[A]);
    for _ in 0..40 {
        pass(&mut s.btt, &[]);
    }
    let all: Vec<Hit> = [-20.0, 0.0, 20.0, 40.0].map(|x| hit_at([x, 15.0])).to_vec();
    pass(&mut s.btt, &all);
    assert_eq!(s.text(), "Done! 4 in 0:00.68 (best 0:00.68)", "41 frames; the three-target best is not this layout's");
}

/// Row 30: L+R+A (the game's reset) and a menu close return to Ready with every target; the best is kept.
fn btt_restart() {
    let mut s = ready_with_three();
    s.tap(&[A]);
    for _ in 0..20 {
        pass(&mut s.btt, &[]);
    }
    pass(&mut s.btt, &[hit_at([0.0, 15.0])]);
    assert_eq!(s.text(), "Targets 1/3   0:00.35");
    s.btt.reset();
    assert_eq!((s.btt.phase(), s.text(), count(&s.btt, EggPicture::Whole)), (Phase::Ready, "Targets 0/3   0:00.00".to_owned(), 3));
    // A finished run's best survives both.
    run_three(&mut s, 30);
    s.btt.reset();
    s.btt.close_menu(true, false);
    assert_eq!(s.btt.phase(), Phase::Ready, "the menu closed with the mode On and a layout");
    run_three(&mut s, 60);
    assert_eq!(s.text(), "Done! 3 in 0:01.00 (best 0:00.50)");
    // Place targets closes the menu into Placing and keeps the layout; closing it again opens Ready; Off stops.
    s.btt.close_menu(true, true);
    assert_eq!((s.btt.phase(), s.btt.layout().len()), (Phase::Placing, 3));
    s.btt.reset();
    assert_eq!(s.btt.phase(), Phase::Placing, "the game's reset does not leave Placing");
    s.btt.close_menu(true, false);
    assert_eq!(s.btt.phase(), Phase::Ready, "the menu closed from Placing with a layout");
    s.btt.close_menu(false, false);
    assert_eq!(s.btt.phase(), Phase::Off);
    let mut empty = Sim::placing();
    empty.btt.close_menu(false, false);
    empty.btt.close_menu(true, false);
    assert_eq!(empty.btt.phase(), Phase::Placing, "no targets placed");
}

/// Row 32: the layout is kept per stage for the session; another stage starts empty.
fn btt_layout_per_stage() {
    let mut s = Sim::placing();
    s.place_at(-20.0, 15.0);
    s.place_at(20.0, 15.0);
    assert!(s.btt.enter(20), "a stage change asks for that stage's file");
    assert!(!s.btt.enter(20), "the same stage again asks for nothing");
    assert_eq!((s.btt.phase(), s.btt.layout().len()), (Phase::Placing, 0), "another stage starts empty, in Placing");
    s.place_at(6.0, 31.0);
    assert!(s.btt.enter(10));
    assert_eq!((s.btt.phase(), s.btt.layout()), (Phase::Ready, &[[-20.0, 15.0], [20.0, 15.0]][..]), "back on stage 10");
    s.btt.enter(20);
    assert_eq!(s.btt.layout().len(), 1);
    // Eight stages are remembered; the oldest go after that.
    for stage in 100..110 {
        s.btt.enter(stage);
        s.place_at(0.0, 15.0);
    }
    s.btt.enter(20);
    assert_eq!(s.btt.layout().len(), 0, "stage 20 fell out of the eight");
    s.btt.enter(109);
    assert_eq!(s.btt.layout().len(), 1);
    // Off keeps the layout too.
    s.btt.close_menu(false, false);
    s.btt.close_menu(true, false);
    assert_eq!((s.btt.phase(), s.btt.layout().len()), (Phase::Ready, 1));
}

/// Row 33: turning one mode On sets the other Off, and back; both On at once (a hand-edited file) keeps Eggs-ercise.
fn btt_modes_exclusive() {
    // [Eggs-ercise, Break the Targets], before and after.
    for (was, now, want) in [
        ([false, false], [false, true], [false, true]),
        ([false, true], [true, true], [true, false]),
        ([true, false], [true, true], [false, true]),
        ([true, true], [true, true], [true, false]),
        ([true, false], [true, false], [true, false]),
        ([false, true], [false, false], [false, false]),
    ] {
        assert_eq!(exclusive(was, now), want, "{was:?} -> {now:?}");
    }
}

/// Row 41: Save writes the slot with the targets; a full slot asks first and replaces only on a second Save within a
/// second; no targets is refused.
fn btt_layout_save() {
    let mut s = Sim::placing();
    s.chord(true);
    assert_eq!((s.text(), s.btt.take_write()), ("Place at least one target".to_owned(), false), "nothing to save");
    for x in [-20.0, 0.0, 20.0] {
        s.place_at(x, 15.0);
    }
    // ZL + Right chooses Slot 2, and the notice says what it holds.
    s.slot(true);
    assert_eq!(s.text(), "Place targets 3/10   Slot 2: empty");
    s.chord(true);
    assert_eq!((s.text(), s.btt.take_write()), ("Slot 2 · Saved".to_owned(), true));
    let slot = s.btt.slots()[1].expect("saved");
    assert_eq!((slot.n, slot.at[..3].to_vec()), (3, vec![[-20.0, 15.0], [0.0, 15.0], [20.0, 15.0]]));
    assert!(s.btt.slots()[0].is_none() && s.btt.slots()[2].is_none());
    // The file: stage, version and five slots, null where empty; it parses back to the same slots.
    let text = render(s.btt.slots(), 10);
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!((json["version"].as_u64(), json["stage"].as_i64()), (Some(1), Some(10)));
    assert_eq!(json["slots"].as_array().unwrap().len(), 5);
    assert!(json["slots"][0].is_null() && json["slots"][1]["n"] == 3);
    assert_eq!(parse(&text, 10), Some(*s.btt.slots()));
    // A full slot asks first; a second Save within a second replaces it; one after the question lapsed asks again.
    s.chord(true);
    assert_eq!((s.text(), s.btt.take_write()), ("Slot 2 has 3 targets: save again to replace".to_owned(), false));
    s.idle(70);
    s.chord(true);
    assert!(!s.btt.take_write(), "the question had lapsed");
    s.chord(true);
    assert_eq!((s.text(), s.btt.take_write()), ("Slot 2 · Saved".to_owned(), true));
    s.place_at(40.0, 15.0);
    s.chord(true);
    s.chord(true);
    assert_eq!((s.btt.slots()[1].unwrap().n, s.btt.take_write()), (4, true));
}

/// Row 42: Load puts the slot's targets back in Placing after a reboot of the game; an empty slot says so.
fn btt_layout_load() {
    let mut s = Sim::placing();
    for x in [-30.0, 10.0] {
        s.place_at(x, 21.0);
    }
    s.slot(true);
    s.chord(true);
    let file = render(s.btt.slots(), 10);
    // A fresh boot: nothing in memory but the file.
    let mut fresh = Sim::placing();
    fresh.btt = Btt::IDLE;
    fresh.btt.enter(10);
    fresh.btt.adopt(parse(&file, 10).unwrap(), false);
    fresh.btt.close_menu(true, true);
    assert_eq!((fresh.btt.phase(), fresh.btt.layout().len()), (Phase::Placing, 0));
    fresh.slot(true);
    assert_eq!(fresh.text(), "Place targets 0/10   Slot 2: 2 saved");
    fresh.chord(false);
    assert_eq!((fresh.btt.layout(), fresh.text()), (&[[-30.0, 21.0], [10.0, 21.0]][..], "Slot 2 · Loaded".to_owned()));
    assert_eq!(fresh.btt.phase(), Phase::Placing, "still in Placing, to edit or start");
    fresh.tap(&[X]);
    assert_eq!(fresh.btt.phase(), Phase::Ready);
    // An empty slot says so and changes nothing.
    let mut fresh = Sim::placing();
    fresh.btt.enter(11);
    fresh.place_at(0.0, 15.0);
    fresh.chord(false);
    assert_eq!((fresh.btt.layout().len(), fresh.text()), (1, "Slot 1 is empty".to_owned()));
}

/// Row 43: ZL + Y twice deletes; ZL + Left / Right chooses the slot and the notice shows it; outside Placing the chords are
/// the normal save states.
fn btt_layout_controls() {
    let mut s = Sim::placing();
    s.place_at(0.0, 15.0);
    for _ in 0..3 {
        s.slot(true);
    }
    assert_eq!(s.text(), "Place targets 1/10   Slot 4: empty", "three Rights from Slot 1");
    s.slot(false);
    assert_eq!(s.text(), "Place targets 1/10   Slot 3: empty");
    s.chord(true);
    s.btt.take_write();
    s.tap(&[ZL, Y]);
    assert_eq!((s.text(), s.btt.slots()[2].is_some()), ("ZL + Y again to delete Slot 3".to_owned(), true));
    s.tap(&[ZL, Y]);
    assert_eq!((s.text(), s.btt.slots()[2].is_some(), s.btt.take_write()), ("Slot 3 · Deleted".to_owned(), false, true));
    // The slots are a ring.
    for want in ["Slot 4: empty", "Slot 5: empty", "Slot 1: empty"] {
        s.slot(true);
        assert!(s.text().ends_with(want), "{} vs {want}", s.text());
    }
    s.slot(false);
    assert!(s.text().ends_with("Slot 5: empty"));
    // ZL + D-pad is not the cursor, and ZL + Y on an empty slot does not clear the layout.
    s.tap(&[ZL, Y]);
    assert_eq!((s.btt.layout().len(), s.btt.cursor()), (1, Some([0.0, 15.0])));
    // Outside Placing the save-state chords are not ours: nothing is taken or written, and the controller stays the game's.
    let mut r = ready_with_three();
    let pad = Pad { held: bits(&[ZL, DPAD_DOWN]), pressed: bits(&[ZL, DPAD_DOWN]), save: true, ..Pad::default() };
    assert!(!r.btt.feed(pad), "not hidden from the game");
    assert!(!r.btt.take_write());
    assert_eq!(r.btt.phase(), Phase::Running, "a press while Ready is the first input");
    let mut off = Btt::IDLE;
    assert!(!off.feed(pad));
    assert_eq!(off.phase(), Phase::Off);
}

/// Row 45 (file errors): a file that cannot be read is ignored with the notice and left alone until a Save; another stage's
/// file is not touched.
fn btt_layout_file_errors() {
    for text in [
        "",
        "{",
        "[]",
        "{\"version\":1}",
        "{\"version\":2,\"stage\":10,\"slots\":[null,null,null,null,null]}",
        "{\"version\":1,\"stage\":11,\"slots\":[null,null,null,null,null]}",
        "{\"version\":1,\"stage\":10,\"slots\":[{\"n\":2,\"at\":[[1,2],[3]]},null,null,null,null]}",
        "{\"version\":1,\"stage\":10,\"slots\":[{\"n\":11,\"at\":[[0,0],[0,0],[0,0],[0,0],[0,0],[0,0],[0,0],[0,0],[0,0],[0,0]]},null,null,null,null]}",
    ] {
        assert_eq!(parse(text, 10), None, "{text}");
    }
    let dir = std::env::temp_dir().join(format!("tr-btt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(read(&dir, 10), ([None; SLOTS], false), "no file: empty slots, nothing wrong");
    let mut s = Sim::placing();
    s.place_at(0.0, 15.0);
    s.chord(true);
    s.btt.take_write();
    write(&dir, 10, s.btt.slots()).unwrap();
    write(&dir, 20, s.btt.slots()).unwrap();
    assert_eq!(path(&dir, 10), dir.join("targets").join("10.json"));
    assert_eq!(read(&dir, 10), (*s.btt.slots(), false), "a round trip through the file");
    // A corrupt file: unreadable, the slots empty, and the bytes stay as they are until a Save writes them.
    let bad = path(&dir, 10);
    std::fs::write(&bad, "{ not json").unwrap();
    let (slots, unreadable) = read(&dir, 10);
    assert!(unreadable);
    let mut fresh = Sim::placing();
    fresh.btt = Btt::IDLE;
    fresh.btt.enter(10);
    fresh.btt.adopt(slots, unreadable);
    fresh.btt.close_menu(true, true);
    assert_eq!(fresh.text(), "Target layouts for this stage couldn't be read");
    fresh.place_at(0.0, 15.0);
    fresh.slot(true);
    fresh.slot(false);
    assert!(!fresh.btt.take_write(), "reading and choosing write nothing");
    assert_eq!(std::fs::read_to_string(&bad).unwrap(), "{ not json", "left alone");
    fresh.chord(true);
    assert!(fresh.btt.take_write(), "a Save replaces it");
    write(&dir, 10, fresh.btt.slots()).unwrap();
    assert!(!read(&dir, 10).1);
    // Only that stage's file was written.
    assert_eq!(read(&dir, 20), (*s.btt.slots(), false));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The pictures (V2.1 ii, V2.5; the pane pools of `panes::EGG_POOLS`): the target for each unbroken one, the cursor only while
/// Placing, and a break animation (Eggs-ercise's, at most two) that fades over `BURST_TICKS` ticks and holds when the clock does.
fn btt_pictures() {
    let mut s = Sim::placing();
    assert_eq!((count(&s.btt, EggPicture::Cursor), count(&s.btt, EggPicture::Whole)), (1, 0), "the cursor alone");
    for x in [-20.0, 0.0, 20.0] {
        s.place_at(x, 15.0);
    }
    assert_eq!((count(&s.btt, EggPicture::Cursor), count(&s.btt, EggPicture::Whole)), (1, 3));
    s.tap(&[X]);
    s.tap(&[A]);
    assert_eq!((count(&s.btt, EggPicture::Cursor), count(&s.btt, EggPicture::Whole)), (0, 3), "no cursor once started");
    pass(&mut s.btt, &[hit_at([0.0, 15.0])]);
    let seen = s.btt.look().into_iter().flatten().collect::<Vec<_>>();
    assert_eq!(seen.iter().filter(|l| l.1.picture == EggPicture::Whole).count(), 2, "the broken target is gone");
    assert_eq!(seen.iter().find(|l| l.1.picture == EggPicture::Cracked).map(|l| (l.0, l.1.alpha)), Some(([0.0, 15.0], 255)), "its break starts full");
    s.btt.pass(false, |_| false);
    assert_eq!(count(&s.btt, EggPicture::Cracked), 1, "a pause holds it");
    for _ in 0..eggs::BURST_TICKS {
        pass(&mut s.btt, &[]);
    }
    assert_eq!(count(&s.btt, EggPicture::Cracked), 0, "gone after BURST_TICKS ticks");
    // Two breaks at once show two; the run's end leaves the last one to finish.
    pass(&mut s.btt, &[hit_at([-20.0, 15.0]), hit_at([20.0, 15.0])]);
    assert_eq!((s.btt.phase(), count(&s.btt, EggPicture::Cracked), count(&s.btt, EggPicture::Whole)), (Phase::Done, 2, 0));
}

/// The runtime posts the notice when `notice` says it changed, after the pass: the pass must not have used the change up (Eden
/// btt-2: the pass returned it and the runtime dropped it, so no notice and no `[Btt]` line ever showed).
fn btt_notice_reports_each_change() {
    let mut s = Sim::placing();
    s.btt.pass(true, |_| false);
    assert_eq!(s.btt.notice(), Some(u16::MAX), "the Placing text, after a pass");
    assert_eq!(s.btt.notice(), None, "reported once");
    s.place_at(0.0, 15.0);
    s.btt.pass(true, |_| false);
    assert_eq!(s.btt.notice(), Some(u16::MAX), "a target placed");
    s.tap(&[X]);
    s.btt.pass(true, |_| false);
    assert_eq!(s.btt.notice(), Some(u16::MAX), "Ready");
    s.tap(&[A]);
    pass(&mut s.btt, &[]);
    assert_eq!(s.btt.notice(), Some(u16::MAX), "the clock moved");
    pass(&mut s.btt, &[hit_at([0.0, 15.0])]);
    assert_eq!((s.btt.phase(), s.btt.notice()), (Phase::Done, Some(u16::MAX)), "the run ended on that pass");
}

/// The runtime asks `notice` on every pass, Off included, and clears the notice when the text is empty: turning the mode Off
/// reports the empty text once, and turning it back On to the same Ready text reports that text again (the Off cleared it).
fn btt_notice_reposts_after_off() {
    let mut s = ready_with_three();
    s.btt.pass(true, |_| false);
    let text = s.text();
    assert_eq!((s.btt.notice(), text.as_str()), (Some(u16::MAX), "Targets 0/3   0:00.00"), "Ready, posted");
    s.btt.close_menu(false, false);
    assert_eq!((s.btt.phase(), s.btt.notice(), s.text()), (Phase::Off, Some(u16::MAX), String::new()), "Off: the empty text, once");
    assert_eq!(s.btt.notice(), None, "Off, nothing new");
    s.btt.close_menu(true, false);
    assert_eq!((s.btt.phase(), s.btt.notice(), s.text()), (Phase::Ready, Some(u16::MAX), text), "On again: posted again");
}

/// The clock ticks once a frame however many of P1's fighters pass, and follows a swap.
fn btt_clock_keeper() {
    let mut b = Btt::IDLE;
    assert_eq!([b.keeper(1), b.keeper(2), b.keeper(1), b.keeper(2)], [true, false, true, false], "P1 then Nana: one tick a frame");
    assert_eq!([b.keeper(3), b.keeper(3), b.keeper(3)], [false, true, true], "a swapped-in fighter takes it after two missed passes");
}

/// The cases of this module in one table.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("btt_rows_and_help", btt_rows_and_help as fn()),
        ("btt_placing_inputs", btt_placing_inputs as fn()),
        ("btt_cursor_clamp", btt_cursor_clamp as fn()),
        ("btt_ready_waits_for_input", btt_ready_waits_for_input as fn()),
        ("btt_run_and_best", btt_run_and_best as fn()),
        ("btt_restart", btt_restart as fn()),
        ("btt_layout_per_stage", btt_layout_per_stage as fn()),
        ("btt_modes_exclusive", btt_modes_exclusive as fn()),
        ("btt_layout_save", btt_layout_save as fn()),
        ("btt_layout_load", btt_layout_load as fn()),
        ("btt_layout_controls", btt_layout_controls as fn()),
        ("btt_layout_file_errors", btt_layout_file_errors as fn()),
        ("btt_pictures", btt_pictures as fn()),
        ("btt_notice_reports_each_change", btt_notice_reports_each_change as fn()),
        ("btt_notice_reposts_after_off", btt_notice_reposts_after_off as fn()),
        ("btt_clock_keeper", btt_clock_keeper as fn()),
    ]);
}
}
