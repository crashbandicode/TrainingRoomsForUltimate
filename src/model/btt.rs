//! Break the Targets (rc.14, `artifacts/rc14-eggsercise/SPEC.md` §V2.5): place up to ten targets on the stage, zoomed out, then
//! break them all against the clock. The target, its hit test (`eggs::breaks`) and its break animation are Eggs-ercise's; this
//! module is the state: Placing (the cursor and the layout), Ready, Running and Done, the placing box, the session's bests,
//! the layout kept per stage, and the five saved layouts of a stage. Pure and host-tested (`btt_tests.rs`); the runtime half
//! feeds it P1's poll and pass and draws `marks`. Nothing here allocates except the file functions, which the menu worker runs.

use core::fmt::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::chords::logical::{A, B, DPAD_DOWN, DPAD_LEFT, DPAD_RIGHT, DPAD_UP, X, Y, ZL};
use super::eggs::{self, Bursts, View, END_FRAMES, RADIUS};
use super::hitbox_orbs::CANVAS_HALF;
use super::panes::{EggLook, EggPicture, EGG_WHOLE};
use super::storage::FixedPath;

pub const ROW: &str = "btt";
pub const ROW_PLACE: &str = "btt_place";
pub const ROW_LAYOUTS: &str = "btt_layouts";
pub const HEADER: &str = "Break the Targets";
/// The standing notice's id (Eggs-ercise has 0x2E; the two modes are never on together, but each clears only its own).
pub const NOTICE_ID: u8 = 0x2F;
pub const MAX: usize = 10;
pub const SLOTS: usize = 5;
/// The pictures `look` returns: the targets, the break animations, the cursor.
pub const LOOKS: usize = MAX + 3;
/// The cursor's units a frame at full tilt, and the stick's dead zone (which is also what starts a run).
const SPEED: f32 = 2.0;
const DEAD: f32 = 0.3;
/// Frames a question stands (a second) or a message shows.
const ASK: u16 = 60;
const SHOW: u16 = 120;
/// Two targets nearer than this overlap.
const APART: f32 = 2.0 * RADIUS;
/// Stages whose layout the session remembers, and the bests it keeps.
const KEPT: usize = 8;
const BESTS: usize = 16;
const DPAD: u16 = 1 << DPAD_UP | 1 << DPAD_DOWN | 1 << DPAD_LEFT | 1 << DPAD_RIGHT;

const fn bit(button: u8) -> u16 {
    1 << button
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Off,
    Placing,
    Ready,
    Running,
    Done,
}

/// One P1 poll in logical buttons (`chords::logical`): the left stick, the buttons held and newly pressed, and whether the
/// Save State Save or Load chord passed (the configured chords, so a moved chord is used here too).
#[derive(Clone, Copy, Debug, Default)]
pub struct Pad {
    pub stick: [f32; 2],
    pub held: u16,
    pub pressed: u16,
    pub save: bool,
    pub load: bool,
}

/// The placing box `[x, y]` in the fighters' plane: the stage's camera range `[l, r, t, b]` cut to what the camera shows (`view`)
/// less a target's radius, from 10 under the ground up to the range's top. No stage read: ±90 and 0 to 80 above the ground.
pub fn bounds(range: Option<[f32; 4]>, view: Option<View>, ground: f32) -> [[f32; 2]; 2] {
    let mut b = match range {
        Some([l, r, t, _]) => [[l, r], [ground - 10.0, t]],
        None => [[-90.0, 90.0], [ground, ground + 80.0]],
    };
    if let Some(View(s, o, _)) = view {
        for (i, r) in b.iter_mut().enumerate() {
            *r = [r[0].max((-CANVAS_HALF[i] - o[i]) / s[i] + RADIUS), r[1].min((CANVAS_HALF[i] - o[i]) / s[i] - RADIUS)];
            r[1] = r[1].max(r[0]);
        }
    }
    b
}
/// One saved layout: its first `n` targets, to a tenth.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slot {
    pub n: u8,
    pub at: [[f32; 2]; MAX],
}

pub type Slots = [Option<Slot>; SLOTS];

/// `targets/<stage>.json`: a version, the stage, and the five slots (`null` when empty).
#[derive(Serialize, Deserialize)]
struct File {
    version: u8,
    stage: i32,
    slots: Slots,
}

pub fn render(slots: &Slots, stage: i32) -> String {
    serde_json::to_string(&File { version: 1, stage, slots: *slots }).unwrap_or_default()
}

/// The slots of a stage's file; `None` when it is not this version, not this stage's, or has more than ten targets in a slot.
pub fn parse(text: &str, stage: i32) -> Option<Slots> {
    let file: File = serde_json::from_str(text).ok()?;
    (file.version == 1 && file.stage == stage && file.slots.iter().flatten().all(|s| usize::from(s.n) <= MAX)).then_some(file.slots)
}
pub fn path(root: &Path, stage: i32) -> PathBuf {
    root.join("targets").join(format!("{stage}.json"))
}

/// A stage's slots and whether its file could not be read (a missing file is not an error). The menu worker's.
pub fn read(root: &Path, stage: i32) -> (Slots, bool) {
    let path = path(root, stage);
    match std::fs::read_to_string(&path) {
        Ok(text) => parse(&text, stage).map_or(([None; SLOTS], true), |slots| (slots, false)),
        // Whether the file is there decides, not the error's kind: on the console a missing file's error is not `NotFound`
        // (Eden btt-2 and btt-4-1: `unreadable=true` on every stage without a file).
        Err(_) => ([None; SLOTS], path.exists()),
    }
}

/// Replace a stage's file; only that stage's. The menu worker's.
pub fn write(root: &Path, stage: i32, slots: &Slots) -> std::io::Result<()> {
    std::fs::create_dir_all(root.join("targets"))?;
    super::store::write_replace(&path(root, stage), render(slots, stage).as_bytes())
}

/// Whether `[Eggs-ercise, Break the Targets]` are On (any option but Off). The two cannot run together: the menu turns Off the
/// one not turned On last, and a settings file with both On (hand-edited) keeps Eggs-ercise.
pub fn on(rows: &super::settings::Snapshot) -> [bool; 2] {
    ["eggsercise", ROW].map(|id| rows.get(id).is_some_and(|r| r.iter().skip(1).any(|v| *v != 0)))
}

/// The layout's identity for the session's best: its target positions to half a unit, in no order.
fn key(at: &[[f32; 2]]) -> u64 {
    let h = |v: f32, m: u64| ((v * 2.0).round() as i64 as u64).wrapping_mul(m);
    at.iter().fold(at.len() as u64, |k, p| k.wrapping_add(h(p[0], 0x9E37_79B9_7F4A_7C15) ^ h(p[1], 0xC2B2_AE3D_27D4_EB4F)))
}

/// The mode's state. Fixed storage, no allocation: a game thread owns it.
pub struct Btt {
    phase: Phase,
    /// The current layout, which of its targets this run has broken, and the cursor.
    at: [[f32; 2]; MAX],
    n: usize,
    broken: u16,
    bursts: Bursts,
    cursor: [f32; 2],
    /// The clock in game frames, and the frames left of Done.
    frames: u32,
    left: u16,
    /// The last poll's buttons, the D-pad's repeat counter, whether B is still a clean undo, and whether the stick has been
    /// still since Ready began.
    was: u16,
    rep: u8,
    clean: bool,
    still: bool,
    /// The message showing (or the question standing) and the frames it has left.
    msg: (FixedPath<64>, u16),
    /// The slot chosen with ZL + Left / Right.
    pick: Option<usize>,
    slots: Slots,
    bad: bool,
    write: bool,
    stage: i32,
    /// What the passes and the layout tick last saw: the stage's camera range, P1's height, the camera's map; and the ground
    /// (P1's height when Placing opened).
    range: Option<[f32; 4]>,
    view: Option<View>,
    here: f32,
    ground: f32,
    kept: [(i32, [[f32; 2]; MAX], usize); KEPT],
    /// The session's best per layout (its key, frames), and the one of the run just done.
    best: [(u64, u32); BESTS],
    top: u32,
    shown: FixedPath<96>,
    /// The clock keeper (`keeper`) and its rivals' passes since its last.
    owner: (usize, u8),
}

impl Btt {
    pub const IDLE: Btt = Btt {
        phase: Phase::Off,
        at: [[0.0; 2]; MAX],
        n: 0,
        broken: 0,
        bursts: [None; 2],
        cursor: [0.0; 2],
        frames: 0,
        left: 0,
        was: 0,
        rep: 0,
        clean: false,
        still: false,
        msg: (FixedPath::new(), 0),
        pick: None,
        slots: [None; SLOTS],
        bad: false,
        write: false,
        stage: i32::MIN,
        range: None,
        view: None,
        here: 0.0,
        ground: 0.0,
        kept: [(i32::MIN, [[0.0; 2]; MAX], 0); KEPT],
        best: [(0, 0); BESTS],
        top: 0,
        shown: FixedPath::new(),
        owner: (0, 2),
    };

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn layout(&self) -> &[[f32; 2]] {
        &self.at[..self.n]
    }

    pub fn cursor(&self) -> Option<[f32; 2]> {
        (self.phase == Phase::Placing).then_some(self.cursor)
    }

    /// What to draw: each unbroken target, the break animations, the cursor (the pools of `panes::EGG_POOLS`).
    pub fn look(&self) -> [Option<([f32; 2], EggLook)>; LOOKS] {
        let live = (0..self.n).filter(|i| self.broken >> i & 1 == 0).map(|i| (self.at[i], EGG_WHOLE));
        let cursor = self.cursor().map(|c| (c, EggLook { picture: EggPicture::Cursor, ..EGG_WHOLE }));
        let mut out = [None; LOOKS];
        out.iter_mut().zip(live.chain(self.bursts.into_iter().flatten().map(eggs::burst)).chain(cursor)).for_each(|(slot, pic)| *slot = Some(pic));
        out
    }

    pub fn slots(&self) -> &Slots {
        &self.slots
    }

    /// Whether the stage's file must be written (a Save or a Delete); clears the flag.
    pub fn take_write(&mut self) -> bool {
        core::mem::take(&mut self.write)
    }

    /// Whether `fighter`'s pass ticks the clock: the first to pass keeps it, and another takes it after the keeper missed two
    /// passes in a row (a swap replaced it); Ice Climbers' Nana passes too and does not tick it twice.
    pub fn keeper(&mut self, fighter: usize) -> bool {
        self.owner.1 += 1;
        if self.owner.0 == fighter || self.owner.1 > 2 {
            self.owner = (fighter, 0);
        }
        self.owner.1 == 0
    }

    /// Say `args` (a message, or a question for a second); true when that question was standing: it is answered.
    fn say(&mut self, ask: bool, args: fmt::Arguments) -> bool {
        let mut text = FixedPath::<64>::new();
        let _ = text.write_fmt(args);
        let yes = ask && self.msg.1 > 0 && self.msg.0.as_str() == text.as_str();
        self.msg = [(text, [SHOW, ASK][usize::from(ask)]), (FixedPath::new(), 0)][usize::from(yes)];
        yes
    }

    /// The stage's file was read (the menu worker): its slots, and whether it could not be.
    pub fn adopt(&mut self, slots: Slots, unreadable: bool) {
        (self.slots, self.bad) = (slots, unreadable);
        if unreadable && self.phase == Phase::Placing {
            self.say(false, format_args!("Target layouts for this stage couldn't be read"));
        }
    }

    /// Every pass: the stage's camera range and P1's height. Every layout tick: the camera's map.
    pub fn sense(&mut self, range: Option<[f32; 4]>, here: f32) {
        (self.range, self.here) = (range, here);
    }

    pub fn see(&mut self, view: Option<View>) {
        self.view = view;
    }

    fn start(&mut self) {
        self.ground = self.here;
        let [x, y] = bounds(self.range, self.view, self.ground);
        (self.phase, self.broken, self.bursts) = (Phase::Placing, 0, [None; 2]);
        self.cursor = [(x[0] + x[1]) / 2.0, (self.ground + 15.0).clamp(y[0], y[1])];
        self.adopt(self.slots, self.bad);
    }

    fn ready(&mut self) {
        (self.phase, self.broken, self.bursts, self.frames, self.left, self.still) = (Phase::Ready, 0, [None; 2], 0, 0, false);
    }

    /// The stage in play (every pass): a new one keeps this stage's layout for the session and takes its own, empty if it has
    /// none, and opens Ready or Placing as the layout says. True when it changed: the stage's file is to be read.
    pub fn enter(&mut self, stage: i32) -> bool {
        if stage == self.stage {
            return false;
        }
        let old = self.kept.iter().position(|k| k.0 == self.stage);
        let i = old.unwrap_or_else(|| {
            self.kept.rotate_left(1);
            KEPT - 1
        });
        self.kept[i] = (self.stage, self.at, self.n);
        let kept = self.kept.iter().find(|k| k.0 == stage).copied();
        let (_, at, n) = kept.unwrap_or((stage, [[0.0; 2]; MAX], 0));
        (self.stage, self.at, self.n, self.slots) = (stage, at, n, [None; SLOTS]);
        (self.bad, self.pick) = (false, None);
        match self.phase {
            Phase::Off => {}
            _ if n > 0 => self.ready(),
            _ => self.start(),
        }
        true
    }

    /// The menu closed: Off stops; Place targets, or no targets, opens Placing; otherwise Ready, every target back.
    pub fn close_menu(&mut self, on: bool, place: bool) {
        (self.was, self.rep, self.clean, self.msg) = (0, 0, false, (FixedPath::new(), 0));
        match (on, place || self.n == 0) {
            (false, _) => self.phase = Phase::Off,
            (_, true) => self.start(),
            _ => self.ready(),
        }
    }

    /// The game initialised the fighters again (L+R+A, Change Stage): a run starts over, Placing stays.
    pub fn reset(&mut self) {
        if matches!(self.phase, Phase::Ready | Phase::Running | Phase::Done) {
            self.ready();
        }
    }

    /// One P1 poll, before the game maps it: true while the game must not see the controller (Placing). In Ready the first
    /// input starts the clock: a button, or the stick leaving its dead zone after it had been inside it.
    pub fn feed(&mut self, p: Pad) -> bool {
        let was = core::mem::replace(&mut self.was, p.held);
        let hide = self.phase == Phase::Placing;
        match self.phase {
            Phase::Placing => self.placing(p, was),
            Phase::Ready => {
                let tilt = p.stick.iter().any(|v| v.abs() >= DEAD);
                self.still |= !tilt;
                if p.pressed != 0 || tilt && self.still {
                    self.phase = Phase::Running;
                }
            }
            _ => {}
        }
        hide
    }

    fn placing(&mut self, p: Pad, was: u16) {
        let b = bounds(self.range, self.view, self.ground);
        let down = |k: u8| p.held & bit(k) != 0;
        let pressed = |k: u8| p.pressed & bit(k) != 0;
        let (zl, held_b, slot) = (down(ZL), down(B), self.pick.unwrap_or(0));
        self.msg.1 = self.msg.1.saturating_sub(1);
        // The cursor: the stick, and the D-pad a unit a press, repeating from the 15th frame held (not with ZL or B: chords).
        let pad = p.held & DPAD;
        self.rep = self.rep.saturating_add(1) * u8::from(pad != 0 && pad == was & DPAD);
        let repeat = self.rep == 0 || self.rep >= 15 && self.rep % 2 == 1;
        let step = f32::from(pad != 0 && !zl && !held_b && repeat);
        let unit = [[DPAD_RIGHT, DPAD_LEFT], [DPAD_UP, DPAD_DOWN]].map(|[hi, lo]| step * (f32::from(down(hi)) - f32::from(down(lo))));
        for (i, range) in b.into_iter().enumerate() {
            let tilt = p.stick[i] * SPEED * f32::from(p.stick[i].abs() >= DEAD);
            self.cursor[i] = (self.cursor[i] + tilt + unit[i]).clamp(range[0], range[1]);
        }
        // B removes the last target on release, if nothing else was pressed while it was held.
        if pressed(B) || held_b && p.pressed != 0 {
            self.clean = p.pressed == bit(B);
        }
        if was & bit(B) != 0 && !held_b && self.clean {
            self.n = self.n.saturating_sub(1);
        }
        if pressed(A) && !zl {
            self.put();
        }
        if pressed(X) && !zl {
            if self.n == 0 {
                self.say(false, format_args!("Place at least one target"));
            } else {
                self.ready();
            }
        }
        if pressed(Y) && zl && (self.slots[slot].is_none() || self.say(true, format_args!("ZL + Y again to delete Slot {}", slot + 1))) {
            self.erase(slot);
        } else if pressed(Y) && !zl && self.say(true, format_args!("Y again to clear all")) {
            self.n = 0;
        }
        for (k, d) in [(DPAD_RIGHT, 1), (DPAD_LEFT, SLOTS - 1)] {
            if zl && pressed(k) {
                (self.pick, self.msg.1) = (Some((slot + d) % SLOTS), 0);
            }
        }
        if p.save {
            self.save(slot);
        }
        if p.load {
            self.load(slot);
        }
    }

    /// A places a target at the cursor: not an 11th, not on another.
    fn put(&mut self) {
        let c = self.cursor;
        let near = |t: &[f32; 2]| (t[0] - c[0]).hypot(t[1] - c[1]) < APART;
        if self.n == MAX {
            self.say(false, format_args!("{MAX} targets: B removes the last"));
        } else if self.layout().iter().any(near) {
            self.say(false, format_args!("Too close to another target"));
        } else {
            (self.at[self.n], self.n, self.msg.1) = (c, self.n + 1, 0);
        }
    }

    /// The layout to slot `s`; a full slot asks first.
    fn save(&mut self, s: usize) {
        let k = s + 1;
        if self.n == 0 {
            self.say(false, format_args!("Place at least one target"));
        } else if self.slots[s].is_none_or(|old| {
            self.say(true, format_args!("Slot {k} has {} targets: save again to replace", old.n))
        }) {
            let mut at = [[0.0; 2]; MAX];
            for (d, t) in at.iter_mut().zip(self.layout()) {
                *d = t.map(|v| (v * 10.0).round() / 10.0);
            }
            let n = self.n as u8;
            (self.slots[s], self.bad, self.write) = (Some(Slot { n, at }), false, true);
            self.say(false, format_args!("Slot {k} · Saved"));
        }
    }

    /// Slot `s` becomes the layout (still Placing, to edit or start); an empty slot says so.
    pub fn load(&mut self, s: usize) {
        match self.slots[s] {
            Some(slot) => {
                (self.at, self.n) = (slot.at, usize::from(slot.n));
                self.say(false, format_args!("Slot {} · Loaded", s + 1));
            }
            None => _ = self.say(false, format_args!("Slot {} is empty", s + 1)),
        }
    }

    pub fn erase(&mut self, s: usize) {
        if self.slots[s].take().is_some() {
            self.write = true;
            self.say(false, format_args!("Slot {} · Deleted", s + 1));
        } else {
            self.say(false, format_args!("Slot {} is empty", s + 1));
        }
    }

    /// One pass of Player 1 (`tick`: the clock's frame): the clock runs while Running, and `hit(spot)` says whether a live hit of P1's
    /// breaks the unbroken target there (`eggs::breaks`; the caller also owns the hitlag and the break animation). The last target
    /// ends the run on that frame. The number of targets broken; the notice is the caller's to take (`notice`), after the pass.
    pub fn pass(&mut self, tick: bool, mut hit: impl FnMut([f32; 2]) -> bool) -> u8 {
        let was = self.broken;
        eggs::age(&mut self.bursts, tick);
        match self.phase {
            Phase::Running => {
                self.frames += u32::from(tick);
                for i in 0..self.n {
                    if self.broken >> i & 1 == 0 && hit(self.at[i]) {
                        (self.broken, self.bursts) = (self.broken | 1 << i, [Some((self.at[i], 0)), self.bursts[0]]);
                    }
                }
                if self.broken.count_ones() as usize == self.n {
                    self.done();
                }
            }
            Phase::Done => {
                self.left = self.left.saturating_sub(u16::from(tick));
                if self.left == 0 {
                    self.ready();
                }
            }
            _ => {}
        }
        (self.broken & !was).count_ones() as u8
    }

    /// The run is over: the clock stops, and the session's best for this layout.
    fn done(&mut self) {
        (self.phase, self.left) = (Phase::Done, END_FRAMES);
        let k = key(self.layout());
        let i = self.best.iter().position(|b| b.0 == k).unwrap_or_else(|| {
            self.best.rotate_left(1);
            self.best[BESTS - 1] = (k, u32::MAX);
            BESTS - 1
        });
        self.top = self.best[i].1.min(self.frames);
        self.best[i].1 = self.top;
    }

    /// The notice's text when it changed since the last call (a standing notice: until the next change).
    pub fn notice(&mut self) -> Option<FixedPath<96>> {
        let mut text = FixedPath::<96>::new();
        let _ = self.write_notice(&mut text);
        let changed = text.as_str() != self.shown.as_str();
        self.shown = text;
        changed.then_some(text)
    }

    /// The notice (SPEC V2.5): `Place targets 3/10   A place  B undo  X start`, `Targets 4/10   0:07.12`, `Done! 10 in
    /// 0:18.27 (best 0:16.90)`, or a message.
    pub fn write_notice(&self, out: &mut impl Write) -> fmt::Result {
        let clock = |f: u32| (f * 5 / 3 / 6000, f * 5 / 3 / 100 % 60, f * 5 / 3 % 100);
        let ((m, s, c), n, got) = (clock(self.frames), self.n, self.broken.count_ones());
        match self.phase {
            Phase::Off => Ok(()),
            Phase::Placing if self.msg.1 > 0 => out.write_str(self.msg.0.as_str()),
            Phase::Placing => {
                write!(out, "Place targets {n}/{MAX}   ")?;
                match self.pick.map(|s| (s + 1, self.slots[s])) {
                    None => write!(out, "A place  B undo  X start"),
                    Some((s, None)) => write!(out, "Slot {s}: empty"),
                    Some((s, Some(slot))) => write!(out, "Slot {s}: {} saved", slot.n),
                }
            }
            Phase::Ready | Phase::Running => write!(out, "Targets {got}/{n}   {m}:{s:02}.{c:02}"),
            Phase::Done => {
                let (bm, bs, bc) = clock(self.top);
                write!(out, "Done! {n} in {m}:{s:02}.{c:02} (best {bm}:{bs:02}.{bc:02})")
            }
        }
    }
}

/// A line of the Saved layouts list: `Slot 2 · 6 targets` or `Slot 3 · empty`. The plugin has no calendar date to show.
pub fn slot_label(index: usize, slot: Option<&Slot>) -> String {
    match slot {
        None => format!("Slot {} · empty", index + 1),
        Some(s) => format!("Slot {} · {} target{}", index + 1, s.n, if s.n == 1 { "" } else { "s" }),
    }
}

/// The list's footer: its first line (the focused row) and this screen's buttons.
pub fn layouts_footer(library_footer: &str) -> String {
    format!("{}\nA Load · Y Delete · B Return", library_footer.lines().next().unwrap_or(""))
}

/// The camera mode that is Training's Fixed camera (`cpu::tech::CAMERA_MODE_FIXED`).
pub const FIXED: i32 = super::cpu::tech::CAMERA_MODE_FIXED;
/// Frames between two asks while the game keeps taking the camera back (its own Training Camera option, a reset).
pub const RETRY: u32 = 10;

/// A game object's address as the plugin may dereference it: aligned, and not in the first pages.
pub const fn pointer_ok(p: usize) -> bool {
    p >= 0x1_0000 && p & 7 == 0
}

/// The camera's state (`runtime::features::btt_camera`: the stage is shown by the game's own Training Fixed camera, mode 4, in
/// Placing, Ready and Running): the mode the game was in when this mode first took the camera (`saved`, while it holds it) and
/// the frames left before it may ask again.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    saved: Option<i32>,
    cool: u32,
}

impl Camera {
    pub const IDLE: Camera = Camera { saved: None, cool: 0 };

    /// The mode took the camera from the game and has not given it back: the Hide Tech camera hooks stand down.
    pub fn holding(&self) -> bool {
        self.saved.is_some()
    }

    /// One pass of Player 1: `want` the Fixed camera, `mode` the game's now (`None`: no camera manager, the scene is over). It
    /// asks (`Some(mode)`) for the Fixed camera at most once in `RETRY` frames while the game is in another mode, and gives the camera back
    /// at once, to the mode first seen, when it is not wanted. A mode the game was already in (the Fixed camera) is not held.
    pub fn step(&mut self, want: bool, mode: Option<i32>) -> Option<i32> {
        self.cool = self.cool.saturating_sub(1);
        let Some(now) = mode else {
            self.saved = None;
            return None;
        };
        match (want, self.saved) {
            (true, _) if now != FIXED && self.cool == 0 => {
                self.saved.get_or_insert(now);
                self.cool = RETRY;
                Some(FIXED)
            }
            (false, Some(back)) => {
                self.saved = None;
                (now == FIXED).then_some(back)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "btt_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "btt_files_tests.rs"]
mod files_tests;
#[cfg(test)]
#[path = "btt_camera_tests.rs"]
mod camera_tests;
