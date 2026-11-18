//! rc.14 Break the Targets (`artifacts/rc14-eggsercise/SPEC.md` §V2.5, acceptance rows 26-33; 41-43 as btt-5 redrew them, `artifacts/rc14-btt-layouts/DESIGN.md`; rows 31, 41 and 42
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
        ("This stage's saved layouts, the five used last first.", "Placing: ZL + Left/Right picks, ZL + Down saves, ZL + Up loads.")
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
    assert_eq!(s.text(), "0/10   A place  B undo  X start");
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
    assert_eq!((s.btt.layout().len(), s.text()), (2, "2/10   A place  B undo  X start".to_owned()));
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

/// Row 26's overlap rule is STRICT at two radii: a target exactly `APART` (8.0) from another is accepted, one under 8 is
/// refused with "Too close to another target". The Eden rows (`eden/rc14-btt-layouts/btt_rows.py`) place targets with held
/// stick macros (2 units a frame), so this boundary is what a one-frame jitter of a macro crosses.
fn btt_overlap_rule_is_strict_at_apart() {
    assert_eq!(APART, 8.0, "two target radii");
    // A second target `dx` right and `dy` up of the first by D-pad presses (a unit each): (accepted, the notice).
    let second = |dx: u32, dy: u32| {
        let mut s = Sim::placing();
        s.tap(&[A]);
        for _ in 0..dx {
            s.tap(&[DPAD_RIGHT]);
        }
        for _ in 0..dy {
            s.tap(&[DPAD_UP]);
        }
        s.tap(&[A]);
        (s.btt.layout().len() == 2, s.text())
    };
    let refused = "Too close to another target".to_owned();
    let placed = "2/10   A place  B undo  X start".to_owned();
    assert_eq!(second(7, 0), (false, refused.clone()), "7.0 apart, along x");
    assert_eq!(second(8, 0), (true, placed.clone()), "exactly 8.0 apart, along x: accepted");
    assert_eq!(second(0, 7), (false, refused.clone()), "7.0 apart, along y");
    assert_eq!(second(0, 8), (true, placed.clone()), "exactly 8.0 apart, along y: accepted");
    // A distance, not a per-axis gap: 6 and 5 is 7.81, 5 and 5 is 7.07; 6 and 6 is 8.49.
    assert_eq!(second(6, 5), (false, refused.clone()), "7.81 apart");
    assert_eq!(second(5, 5), (false, refused), "7.07 apart");
    assert_eq!(second(6, 6), (true, placed.clone()), "8.49 apart");
    // The stick moves 2 a frame: 4 frames is the 8.0 the Eden rows aim at, 3 is 6.0 (one frame short).
    for (frames, ok) in [(3, false), (4, true), (5, true)] {
        let mut s = Sim::placing();
        s.tap(&[A]);
        s.push([1.0, 0.0], frames);
        s.tap(&[A]);
        assert_eq!(s.btt.layout().len() == 2, ok, "{frames} stick frames on: {}", s.text());
    }
    // The rule is also what lets a third target sit 8 on from the second: a chain of exactly-8 gaps is accepted.
    let mut s = Sim::placing();
    for _ in 0..3 {
        s.tap(&[A]);
        s.push([1.0, 0.0], 4);
    }
    assert_eq!(s.btt.layout(), [[0.0, 15.0], [8.0, 15.0], [16.0, 15.0]]);
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
/// Battlefield's fixed camera (`btt_cursor_clamp`), and a view of the same camera moving: its canvas origin `lift` canvas units
/// lower, so the placing box it gives starts high above the ground (Eden rc14-int3-rr2-f1: the mode took the camera and the
/// cursor, untouched, went from (-47, 15) to (-40.1, 117.6)).
fn fixed_view() -> View {
    View([6.89, 6.80], [0.0, -172.26], [-170.0, 170.0])
}

fn moving_view(lift: f32) -> View {
    View([6.89, 6.80], [0.0, -172.26 - lift], [-170.0, 170.0])
}

/// Placing on Battlefield with the fixed camera settled (three layout ticks of the same view): the cursor's start.
fn placing_settled() -> (Sim, [f32; 2]) {
    let mut s = Sim::placing();
    s.btt.sense(Some([-170.0, 170.0, 130.0, -80.0]), 0.0);
    for _ in 0..3 {
        s.btt.see(Some(fixed_view()));
    }
    s.btt.close_menu(true, true);
    let start = s.btt.cursor().unwrap();
    (s, start)
}

/// A camera change alone never moves the cursor from a spot inside the settled view: through a camera that moves (a new view
/// every layout tick, boxes that leave the start out) and back to the settled one, polls with no input keep the start.
fn btt_cursor_stays_through_a_camera_change() {
    let (mut s, start) = placing_settled();
    assert_eq!(start, [0.0, 15.0]);
    for k in 0..8 {
        s.btt.see(Some(moving_view(900.0 - 40.0 * k as f32)));
        s.idle(1);
        assert_eq!(s.btt.cursor(), Some(start), "the camera moving (tick {k}) dragged the cursor");
    }
    for _ in 0..4 {
        s.btt.see(Some(fixed_view()));
        s.idle(1);
    }
    assert_eq!(s.btt.cursor(), Some(start), "inside the settled box: never moved");
}

/// The cursor never stays off screen: a start the settled view leaves out is not moved while the camera changes, and is clamped
/// into the box once the camera has settled (the same view three ticks running); the stick still clamps at once.
fn btt_cursor_off_screen_is_clamped_once_settled() {
    let (mut s, start) = placing_settled();
    let range = Some([-170.0, 170.0, 130.0, -80.0]);
    let far = moving_view(900.0);
    let b = bounds(range, Some(far), 0.0);
    assert!(start[1] < b[1][0], "the far view leaves the start out: {start:?} {b:?}");
    for _ in 0..2 {
        s.btt.see(Some(far));
        s.idle(1);
        assert_eq!(s.btt.cursor(), Some(start), "not before the camera has settled");
    }
    s.btt.see(Some(far));
    s.idle(1);
    let [x, y] = s.btt.cursor().unwrap();
    assert_eq!((x, y), (start[0], b[1][0]), "clamped once settled: onto the box's bottom edge, x kept");
    s.btt.see(Some(far));
    s.idle(3);
    assert_eq!(s.btt.cursor(), Some([x, y]), "settled: no further change");
    // A new camera move, then the stick: a moved cursor is clamped into the box of the moment, as before.
    s.btt.see(Some(moving_view(400.0)));
    s.push([0.0, -1.0], 200);
    let now = bounds(range, Some(moving_view(400.0)), 0.0);
    assert_eq!(s.btt.cursor().unwrap()[1], now[1][0], "the stick clamps at once");
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
    assert_eq!((s.btt.phase(), s.text()), (Phase::Ready, "0/3 0:00.00".to_owned()));
    assert!(!s.frame(&[], [0.0; 2]), "Ready gives P1's controller back");
    for _ in 0..200 {
        pass(&mut s.btt, &[]);
        s.frame(&[], [0.0; 2]);
    }
    assert_eq!((s.btt.phase(), s.text()), (Phase::Ready, "0/3 0:00.00".to_owned()), "the clock waits");
    // A stick inside the dead zone is not an input; a button is.
    s.frame(&[], [0.1, 0.1]);
    assert_eq!(s.btt.phase(), Phase::Ready);
    s.frame(&[X], [0.0; 2]);
    assert_eq!(s.btt.phase(), Phase::Running);
    for _ in 0..60 {
        pass(&mut s.btt, &[]);
    }
    assert_eq!(s.text(), "0/3 0:01.00", "a second of game frames, in hundredths");
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
    assert_eq!(s.text(), "1/3 0:00.51", "31 frames in");
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
    assert_eq!((s.btt.phase(), s.text(), count(&s.btt, EggPicture::Whole)), (Phase::Ready, "0/3 0:00.00".to_owned(), 3));
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
    assert_eq!(s.text(), "0/3 0:00.00");
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
    assert_eq!(s.text(), "1/3 0:00.35");
    s.btt.reset();
    assert_eq!((s.btt.phase(), s.text(), count(&s.btt, EggPicture::Whole)), (Phase::Ready, "0/3 0:00.00".to_owned(), 3));
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

/// A saved layout as the writer publishes it: file `id`, `name`, its targets.
fn hot(id: &str, name: &str, at: &[[f32; 2]]) -> Hot {
    let layout = Layout { schema_version: 1, name: name.into(), stage: 10, used: 1, targets: at.to_vec() };
    five(&[(id.into(), layout)], 1)[0].expect("one")
}

/// The hot five the writer publishes: `list`, newest first.
fn five_of(list: &[Hot]) -> Five {
    let mut out = [None; HOT];
    for (d, h) in out.iter_mut().zip(list) {
        *d = Some(*h);
    }
    out
}

/// Three saved layouts on stage 10, newest first, as the writer publishes them.
fn three() -> Five {
    five_of(&[
        hot("layout-3", "Layout 3 · Battlefield", &[[0.0, 15.0]]),
        hot("ledge-trap", "Ledge trap", &[[-30.0, 21.0], [10.0, 21.0]]),
        hot("layout-1", "Layout 1 · Battlefield", &[[5.0, 5.0], [6.0, 40.0], [7.0, 60.0]]),
    ])
}

/// Row 41 (btt-5): the Save chord asks the writer for a new file every time, with the targets to a tenth: never a question,
/// never a replace. Its answer chooses the new file, first of the five. No targets is refused.
fn layout_save_makes_a_new_file_never_overwrites() {
    let mut s = Sim::placing();
    s.chord(true);
    assert_eq!((s.text(), s.btt.take_ask().is_none()), ("Place at least one target".to_owned(), true), "nothing to save");
    for x in [-20.0, 0.0, 20.0] {
        s.place_at(x, 15.0);
    }
    for _ in 0..2 {
        s.chord(true);
        match s.btt.take_ask() {
            Some(Ask::Save(10, 3, at)) => assert_eq!(at[..3], [[-20.0, 15.0], [0.0, 15.0], [20.0, 15.0]]),
            other => panic!("a new file each time: {other:?}"),
        }
    }
    s.btt.adopt(three(), true);
    assert_eq!(s.text(), "Saved: Layout 3 · Battlefield");
    s.idle(130);
    assert_eq!(s.text(), "3/10   1/3 Layout 3 · Battlefield: 1", "the new file is chosen, first of the five");
}

/// Row 42 (btt-5): the Load chord puts the chosen layout's targets back, still Placing, and uses its file (the writer moves
/// it to the top: the choice follows it). Load on the Saved layouts screen puts a layout in place the same way.
fn layout_load_puts_the_targets_back() {
    let mut s = Sim::placing();
    s.place_at(0.0, 15.0);
    s.btt.adopt(three(), false);
    s.tap(&[ZL, DPAD_RIGHT]);
    s.tap(&[ZL, DPAD_RIGHT]);
    assert_eq!(s.text(), "1/10   2/3 Ledge trap: 2");
    s.chord(false);
    assert_eq!((s.btt.layout(), s.text()), (&[[-30.0, 21.0], [10.0, 21.0]][..], "Loaded: Ledge trap".to_owned()));
    assert!(matches!(s.btt.take_ask(), Some(Ask::Use(id)) if id.as_str() == "ledge-trap"));
    assert_eq!(s.btt.phase(), Phase::Placing, "still in Placing, to edit or start");
    // The writer's answer: the used file is now the newest; the choice stays on it.
    let f = *s.btt.five();
    s.btt.adopt(five_of(&[f[1].unwrap(), f[0].unwrap(), f[2].unwrap()]), false);
    s.idle(130);
    assert_eq!(s.text(), "2/10   1/3 Ledge trap: 2");
    s.tap(&[X]);
    assert_eq!(s.btt.phase(), Phase::Ready);
    // The Saved layouts screen's Load (the menu worker posts the layout).
    let mut m = Sim::placing();
    m.btt.apply(hot("wide-line", "Wide line", &[[1.0, 2.0], [3.0, 4.0]]));
    assert_eq!((m.btt.layout(), m.text()), (&[[1.0, 2.0], [3.0, 4.0]][..], "Loaded: Wide line".to_owned()));
}

/// Row 43 (btt-5): ZL + Left / Right step through this stage's hot five (the first step shows the newest) and the notice names
/// the chosen one; ZL + Y twice deletes it and the five close up; with none saved each chord says so; outside Placing the
/// chords are the normal save states.
fn layout_chords_step_the_hot_five() {
    let mut s = Sim::placing();
    s.place_at(0.0, 15.0);
    s.tap(&[ZL, DPAD_RIGHT]);
    assert_eq!(s.text(), "No saved layouts on this stage yet");
    s.idle(130);
    s.chord(false);
    assert_eq!((s.text(), s.btt.layout().len()), ("No saved layouts on this stage yet".to_owned(), 1));
    s.tap(&[ZL, Y]);
    assert_eq!((s.text(), s.btt.take_ask().is_none()), ("No saved layouts on this stage yet".to_owned(), true));
    s.btt.adopt(three(), false);
    s.idle(130);
    assert_eq!(s.text(), "1/10   A place  B undo  X start", "nothing chosen yet");
    let mut seen = vec![];
    for k in [DPAD_RIGHT, DPAD_RIGHT, DPAD_RIGHT, DPAD_RIGHT, DPAD_LEFT, DPAD_LEFT] {
        s.tap(&[ZL, k]);
        seen.push(s.text().split("   ").nth(1).unwrap_or_default().to_owned());
    }
    assert_eq!(seen, ["1/3 Layout 3 · Battlefield: 1", "2/3 Ledge trap: 2", "3/3 Layout 1 · Battlefield: 3", "1/3 Layout 3 · Battlefield: 1", "3/3 Layout 1 · Battlefield: 3", "2/3 Ledge trap: 2"]);
    // ZL + Y asks, the second deletes the chosen one; the five close up at once.
    s.tap(&[ZL, Y]);
    assert_eq!((s.text(), s.btt.take_ask().is_none()), ("ZL + Y again to delete Ledge trap".to_owned(), true));
    s.tap(&[ZL, Y]);
    assert_eq!(s.text(), "Deleted: Ledge trap");
    assert!(matches!(s.btt.take_ask(), Some(Ask::Delete(id)) if id.as_str() == "ledge-trap"));
    let names: Vec<_> = s.btt.five().iter().flatten().map(|h| h.name.as_str().to_owned()).collect();
    assert_eq!(names, ["Layout 3 · Battlefield", "Layout 1 · Battlefield"]);
    // ZL + D-pad is not the cursor, and the layout placed stays.
    assert_eq!((s.btt.layout().len(), s.btt.cursor()), (1, Some([0.0, 15.0])));
    // A new stage has its own five (the writer lists it), nothing chosen.
    s.btt.enter(20);
    assert!(s.btt.five().iter().all(Option::is_none));
    // Outside Placing the save-state chords are not ours: nothing is asked, and the controller stays the game's.
    let mut r = ready_with_three();
    r.btt.adopt(three(), false);
    let pad = Pad { held: bits(&[ZL, DPAD_DOWN]), pressed: bits(&[ZL, DPAD_DOWN]), save: true, ..Pad::default() };
    assert!(!r.btt.feed(pad), "not hidden from the game");
    assert!(r.btt.take_ask().is_none());
    assert_eq!(r.btt.phase(), Phase::Running, "a press while Ready is the first input");
    let mut off = Btt::IDLE;
    assert!(!off.feed(pad));
    assert_eq!(off.phase(), Phase::Off);
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
    assert!(s.btt.notice().is_some(), "the Placing text, after a pass");
    assert!(s.btt.notice().is_none(), "reported once");
    s.place_at(0.0, 15.0);
    s.btt.pass(true, |_| false);
    assert!(s.btt.notice().is_some(), "a target placed");
    s.tap(&[X]);
    s.btt.pass(true, |_| false);
    assert!(s.btt.notice().is_some(), "Ready");
    s.tap(&[A]);
    pass(&mut s.btt, &[]);
    assert!(s.btt.notice().is_some(), "the clock moved");
    pass(&mut s.btt, &[hit_at([0.0, 15.0])]);
    assert_eq!((s.btt.phase(), s.btt.notice().is_some()), (Phase::Done, true), "the run ended on that pass");
}

/// The runtime asks `notice` on every pass, Off included, and clears the notice when the text is empty: turning the mode Off
/// reports the empty text once, and turning it back On to the same Ready text reports that text again (the Off cleared it).
fn btt_notice_reposts_after_off() {
    let mut s = ready_with_three();
    s.btt.pass(true, |_| false);
    let text = s.text();
    assert_eq!((s.btt.notice().is_some(), text.as_str()), (true, "0/3 0:00.00"), "Ready, posted");
    s.btt.close_menu(false, false);
    assert_eq!((s.btt.phase(), s.btt.notice().is_some(), s.text()), (Phase::Off, true, String::new()), "Off: the empty text, once");
    assert!(s.btt.notice().is_none(), "Off, nothing new");
    s.btt.close_menu(true, false);
    assert_eq!((s.btt.phase(), s.btt.notice().is_some(), s.text()), (Phase::Ready, true, text), "On again: posted again");
}

/// The clock ticks once a frame however many of P1's fighters pass, and follows a swap.
fn btt_clock_keeper() {
    let mut b = Btt::IDLE;
    assert_eq!([b.keeper(1), b.keeper(2), b.keeper(1), b.keeper(2)], [true, false, true, false], "P1 then Nana: one tick a frame");
    assert_eq!([b.keeper(3), b.keeper(3), b.keeper(3)], [false, true, true], "a swapped-in fighter takes it after two missed passes");
}

fn result_clock_matches_btt_done_boundary_matrix() {
    use crate::model::practice_games::result_tick;
    // Expected values are the old BTT boundary behavior, including zero and a paused last frame.
    for (left, tick, after, complete) in [
        (0, false, 0, true), (0, true, 0, true), (1, false, 1, false), (1, true, 0, true),
        (2, false, 2, false), (2, true, 1, false), (180, false, 180, false), (180, true, 179, false),
        (u16::MAX, false, u16::MAX, false), (u16::MAX, true, u16::MAX - 1, false),
    ] {
        let mut shared = left;
        assert_eq!((result_tick(&mut shared, tick), shared), (complete, after));
        let mut s = ready_with_three();
        run_three(&mut s, 3);
        s.btt.left = left;
        assert_eq!(s.btt.pass(tick, |_| panic!("Done must not test targets")), 0);
        assert_eq!(s.btt.phase(), if complete { Phase::Ready } else { Phase::Done });
        assert_eq!(s.btt.left, after);
    }
}

fn btt_shared_countdown_holds_pauses_and_finishes_once() {
    let mut s = ready_with_three();
    run_three(&mut s, 3);
    assert_eq!(END_FRAMES, 180);
    assert!(s.btt.notice().is_some());
    for elapsed in 0..179 {
        s.btt.pass(false, |_| panic!("paused Done must not test targets"));
        assert_eq!((s.btt.phase(), s.btt.left), (Phase::Done, 180 - elapsed));
        assert!(s.btt.notice().is_none());
        s.btt.pass(true, |_| false);
    }
    assert_eq!((s.btt.phase(), s.btt.left), (Phase::Done, 1));
    for _ in 0..7 { s.btt.pass(false, |_| false); }
    assert_eq!(s.btt.phase(), Phase::Done);
    s.btt.pass(true, |_| false);
    assert_eq!((s.btt.phase(), s.btt.left), (Phase::Ready, 0));
    assert!(s.btt.notice().is_some(), "Ready is posted once");
    for tick in [false, true, true, false] {
        s.btt.pass(tick, |_| panic!("Ready must not test targets"));
        assert_eq!(s.btt.phase(), Phase::Ready);
        assert!(s.btt.notice().is_none());
    }
}

fn btt_unfinished_attempt_does_not_create_best() {
    use crate::model::practice_games::best_time;
    let mut s = ready_with_three();
    let layout = key(s.btt.layout());
    assert_eq!(best_time(&s.btt.best, layout), None);
    s.tap(&[A]);
    pass(&mut s.btt, &[hit_at([0.0, 15.0])]);
    s.btt.reset();
    assert_eq!(best_time(&s.btt.best, layout), None, "an incomplete run is not a scored attempt");
    run_three(&mut s, 30);
    assert_eq!(best_time(&s.btt.best, layout), Some(30));
    countdown(&mut s.btt);
    s.tap(&[A]);
    for _ in 0..5 { pass(&mut s.btt, &[]); }
    s.btt.close_menu(false, false);
    assert_eq!(best_time(&s.btt.best, layout), Some(30), "a faster cancelled attempt cannot replace a completed best");
}

fn btt_best_pool_retains_sixteen_fifo_layouts() {
    use crate::model::practice_games::best_time;
    fn finish(b: &mut Btt, x: f32, frames: u32) {
        let mut at = [[0.0; 2]; MAX];
        at[0] = [x, 15.0];
        b.apply(Hot { id: FixedPath::new(), name: FixedPath::new(), n: 1, at });
        b.close_menu(true, false);
        b.feed(Pad { pressed: bit(A), ..Pad::default() });
        for _ in 0..frames { b.pass(true, |_| false); }
        assert_eq!(b.pass(false, |_| true), 1);
        assert_eq!(b.phase(), Phase::Done);
    }
    let mut b = Btt::IDLE;
    assert_eq!(BESTS, 16);
    for x in 0..16 { finish(&mut b, x as f32, x + 10); }
    let first = key(&[[0.0, 15.0]]);
    let second = key(&[[1.0, 15.0]]);
    assert_eq!(best_time(&b.best, first), Some(10));
    finish(&mut b, 0.0, 1);
    assert_eq!(best_time(&b.best, first), Some(1));
    finish(&mut b, 16.0, 26);
    assert_eq!(b.best.iter().flatten().count(), 16);
    assert_eq!(best_time(&b.best, first), None, "improving the oldest layout does not make it newest");
    assert_eq!(best_time(&b.best, second), Some(11));
}

fn btt_compact_hud_geometry() {
    use crate::model::panes::{self as p, cb, iv, Kind};
    for (strip, bar) in [(false, false), (true, false), (true, true)] {
        let [x, y, w, h] = p::btt_hud_box("10/10 0:07.12", strip, bar);
        assert_eq!([x, y, w, h], [-936.0, 504.0, 276.0, 48.0]);
        assert!(x + w <= -iv::STRIP_W / 2.0 - 12.0, "left gutter clears the default strip by 8px at 1280w");
        assert!(y - h / 2.0 > iv::STRIP_Y + cb::SEQ_BAR_Y + cb::BAR_H / 2.0, "the helper bar is below the compact run box");
        let [_, py, _, ph] = p::btt_hud_box("3/10   A place  B undo  X start", strip, bar);
        if bar { assert!(py + ph / 2.0 <= iv::STRIP_Y + cb::SEQ_BAR_Y - cb::BAR_H / 2.0 - 8.0); }
        else if strip { assert!(py + ph / 2.0 <= iv::STRIP_Y - iv::STRIP_H / 2.0 - 8.0); }
    }
    assert_eq!(p::spec(p::IDX_BTT + 2).parent, p::name_of(p::IDX_BTT));
    assert!(matches!(p::spec(p::IDX_BTT + 3).kind, Kind::Text { chars: 96, .. }));
}

fn btt_compact_hud_text() {
    let mut s = Sim::placing();
    assert_eq!(s.text(), "0/10   A place  B undo  X start");
    s.tap(&[A]);
    s.tap(&[X]);
    assert_eq!(s.text(), "0/1 0:00.00");
    s.tap(&[A]);
    pass(&mut s.btt, &[]);
    assert_eq!(s.text(), "0/1 0:00.01");
}

fn btt_projectiles_use_owned_native_hits() {
    use std::cell::Cell;
    let calls = Cell::new(0);
    let own = |kind, ordinal| {
        calls.set(calls.get() + 1);
        if kind == 2 && ordinal < 2 { 0x1_0000 + ordinal as usize * 8 } else { 0 }
    };
    let read = |article| {
        let mut hits = [Hit::default(); crate::model::hitbox::SLOTS as usize];
        // Native capsule endpoints and radius, with no padding. P1's second article supplies the hit.
        hits[0] = Hit([20.0, 15.0, 0.0], [24.0, 15.0, 0.0], 1.0, 1.0);
        Some((hits, usize::from(article == 0x1_0008)))
    };
    assert!(!projectile_breaks(false, [22.0, 15.0], std::iter::from_fn(|| panic!("Off polls no pool")), own, |_| panic!("Off reads no article")));
    assert_eq!(calls.get(), 0);
    assert!(projectile_breaks(true, [22.0, 15.0], [], own, read));
    assert!(!projectile_breaks(true, [22.0, 15.0 + eggs::RADIUS + 1.001], [], own, read), "outside the native radius is a miss");
    assert!(!projectile_breaks(true, [22.0, 15.0], [], |_, _| 0, |_| panic!("another owner's article never enters the reader")));
    assert!(!projectile_breaks(true, [22.0, 15.0], [], |_, n| if n == 0 { 3 } else { 0 }, |_| panic!("misaligned article")));
}

fn btt_fired_bullet_survives_an_empty_current_ring() {
    use crate::engine::seams::article_pool::read as projectile_pool;
    // Trace2 Fox: pool slot1 is active kind1/id10010000, absent from the current ring.
    // Native constructor33b98b0 and post-ATTACK holder snapshot prove this inline layout.
    let base = 0x8000_0000;
    let mut owner = [0usize; 0xa0 / 8];
    let mut module = [0usize; 0x248 / 8];
    let mut holder = [0usize; 0x238 / 8];
    let mut article = [base + 0x5186a58];
    owner[0x98 / 8] = module.as_ptr() as usize;
    module[0] = base + 0x4f6a230;
    module[0x240 / 8] = holder.as_ptr() as usize;
    holder[0] = base + 0x4f626e8;
    holder[1] = 64;
    holder[2] = 2;
    holder[3] = holder.as_ptr() as usize + 0x20;
    holder[5] = article.as_ptr() as usize; // slot0 empty, slot1 the fired bullet.
    let owner_at = owner.as_ptr() as usize;
    let pool = || unsafe { projectile_pool(owner_at, base) };
    let read = |_: usize| {
        let mut hits = [Hit::default(); crate::model::hitbox::SLOTS as usize];
        hits[0] = Hit([20.0, 15.0, 0.0], [24.0, 15.0, 0.0], 1.0, 3.0);
        Some((hits, 1))
    };
    assert!(!projectile_breaks(true, [22.0, 15.0], [], |_, _| 0, read), "the old ring-only census misses");
    assert!(projectile_breaks(true, [22.0, 15.0], pool().into_iter().flatten(), |_, _| 0, read));
    assert!(!projectile_breaks(true, [22.0, 15.0], pool().into_iter().flatten(), |_, _| 0, |_| None), "inactive pooled objects supply no attack");
    assert!(!projectile_breaks(true, [22.0, 15.0], pool().into_iter().flatten(), |_, _| 0, |at| {
        let (mut hits, n) = read(at).unwrap();
        hits[0].3 = 0.0;
        Some((hits, n))
    }), "a zero-damage windbox does not break targets");
    for field in [0, 1, 2, 3] {
        let saved = holder[field];
        holder[field] = match field { 2 => 65, _ => 0 };
        assert!(unsafe { projectile_pool(owner.as_ptr() as usize, base) }.iter().all(Option::is_none), "reject malformed holder field {field}");
        holder[field] = saved;
    }
    article[0] ^= 1;
    assert_ne!(article[0], base + 0x5186a58);
    assert!(pool().iter().all(Option::is_none), "unknown article type is never handed to the native ID getter");
    article[0] ^= 1;
    assert_eq!(article[0], base + 0x5186a58);
    assert!(pool().iter().any(Option::is_some));
    module[0] ^= 1;
    assert_ne!(module[0], base + 0x4f6a230);
    assert!(pool().iter().all(Option::is_none), "wrong ArticleModule type is never followed");
}

/// Frozen ffd2f74a cursor arithmetic, independent of the extracted helper (including its addition order).
fn btt_placing_motion_matches_original_bytes() {
    fn original(p: &Pad, was: u16, age: &mut u8) -> ([f32; 2], [f32; 2]) {
        let down = |k: u8| p.held & bit(k) != 0;
        let pad = p.held & DPAD;
        *age = age.saturating_add(1) * u8::from(pad != 0 && pad == was & DPAD);
        let repeat = *age == 0 || *age >= 15 && *age % 2 == 1;
        let step = f32::from(pad != 0 && !down(ZL) && !down(B) && repeat);
        let unit = [[DPAD_RIGHT, DPAD_LEFT], [DPAD_UP, DPAD_DOWN]].map(|[hi, lo]| step * (f32::from(down(hi)) - f32::from(down(lo))));
        let tilt = [0, 1].map(|i| p.stick[i] * 2.0 * f32::from(p.stick[i].abs() >= 0.3));
        (tilt, unit)
    }
    let bytes = |v: [f32; 2]| v.map(f32::to_bits);
    for mask in 0..16 {
        let pad = [DPAD_RIGHT, DPAD_LEFT, DPAD_UP, DPAD_DOWN].into_iter().enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0).fold(0, |held, (_, key)| held | bit(key));
        for modifiers in [0, bit(B), bit(ZL), bit(B) | bit(ZL)] {
            for was in [0, pad, DPAD] {
                for age in 0..=u8::MAX {
                    for stick in [[0.0, 0.0], [1.0, -1.0], [-0.3, 0.3], [-0.29999, 0.29999], [-0.0, 0.3], [0.2, -0.4]] {
                        let p = Pad { held: pad | modifiers, stick, ..Pad::default() };
                        let (mut expected_age, mut parts_age, mut motion_age) = (age, age, age);
                        let (tilt, unit) = original(&p, was, &mut expected_age);
                        let (actual_tilt, actual_unit) = placing_parts(&p, was, &mut parts_age);
                        let motion = placing_motion(&p, was, &mut motion_age);
                        assert_eq!((parts_age, motion_age), (expected_age, expected_age));
                        assert_eq!((bytes(actual_tilt), bytes(actual_unit)), (bytes(tilt), bytes(unit)));
                        assert_eq!(bytes(motion), bytes([0, 1].map(|i| tilt[i] + unit[i])));
                        for cursor in [-0.0, 0.0, 10.1, 100.0] {
                            assert_eq!(bytes([0, 1].map(|i| cursor + actual_tilt[i] + actual_unit[i])),
                                bytes([0, 1].map(|i| cursor + tilt[i] + unit[i])));
                        }
                    }
                }
            }
        }
    }
}

/// The cases of this module in one table.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("btt_rows_and_help", btt_rows_and_help as fn()),
        ("btt_placing_inputs", btt_placing_inputs as fn()),
        ("btt_overlap_rule_is_strict_at_apart", btt_overlap_rule_is_strict_at_apart as fn()),
        ("btt_cursor_clamp", btt_cursor_clamp as fn()),
        ("btt_cursor_stays_through_a_camera_change", btt_cursor_stays_through_a_camera_change as fn()),
        ("btt_cursor_off_screen_is_clamped_once_settled", btt_cursor_off_screen_is_clamped_once_settled as fn()),
        ("btt_ready_waits_for_input", btt_ready_waits_for_input as fn()),
        ("btt_run_and_best", btt_run_and_best as fn()),
        ("btt_restart", btt_restart as fn()),
        ("btt_layout_per_stage", btt_layout_per_stage as fn()),
        ("layout_save_makes_a_new_file_never_overwrites", layout_save_makes_a_new_file_never_overwrites as fn()),
        ("layout_load_puts_the_targets_back", layout_load_puts_the_targets_back as fn()),
        ("layout_chords_step_the_hot_five", layout_chords_step_the_hot_five as fn()),
        ("btt_pictures", btt_pictures as fn()),
        ("btt_notice_reports_each_change", btt_notice_reports_each_change as fn()),
        ("btt_notice_reposts_after_off", btt_notice_reposts_after_off as fn()),
        ("btt_clock_keeper", btt_clock_keeper as fn()),
        ("result_clock_matches_btt_done_boundary_matrix", result_clock_matches_btt_done_boundary_matrix as fn()),
        ("btt_shared_countdown_holds_pauses_and_finishes_once", btt_shared_countdown_holds_pauses_and_finishes_once as fn()),
        ("btt_unfinished_attempt_does_not_create_best", btt_unfinished_attempt_does_not_create_best as fn()),
        ("btt_best_pool_retains_sixteen_fifo_layouts", btt_best_pool_retains_sixteen_fifo_layouts as fn()),
        ("btt_compact_hud_geometry", btt_compact_hud_geometry as fn()),
        ("btt_compact_hud_text", btt_compact_hud_text as fn()),
        ("btt_placing_motion_matches_original_bytes", btt_placing_motion_matches_original_bytes as fn()),
        ("btt_projectiles_use_owned_native_hits", btt_projectiles_use_owned_native_hits as fn()),
        ("btt_fired_bullet_survives_an_empty_current_ring", btt_fired_bullet_survives_an_empty_current_ring as fn()),
    ]);
}
}
