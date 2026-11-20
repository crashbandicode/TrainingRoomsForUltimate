//! Break the Targets (rc.14, `artifacts/rc14-eggsercise/SPEC.md` §V2.5): place up to ten targets on the stage, zoomed out, then
//! break them all against the clock. The target, its hit test (`eggs::breaks`) and its break animation are Eggs-ercise's; this
//! module is the state: Placing (the cursor and the layout), Ready, Running and Done, the placing box, the session's bests,
//! the layout kept per stage, and the saved layouts (btt-5, `artifacts/rc14-btt-layouts/DESIGN.md`): one file each in
//! `layouts/`, in Recording's convention (the hot five per stage, a new file on every save). Pure and host-tested
//! (`btt_tests.rs`, `btt_files_tests.rs`); the runtime half feeds it P1's poll and pass and draws `marks`. The game thread's half
//! allocates nothing; the file functions run on the writer thread and the menu worker.

use core::fmt::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::chords::logical::{A, B, DPAD_DOWN, DPAD_LEFT, DPAD_RIGHT, DPAD_UP, X, Y, ZL};
use super::eggs::{self, Bursts, View, END_FRAMES, RADIUS};
use super::hitbox_orbs::CANVAS_HALF;
use super::panes::{EggLook, EggPicture, EGG_WHOLE};
use super::practice_games::{record_time, result_tick};
use super::recording_menu::{hot_five, ordered};
use super::storage::FixedPath;
use super::store::{lowest_free, Record, Store, StoreError};

pub const ROW: &str = "btt";
pub const ROW_PLACE: &str = "btt_place";
pub const ROW_LAYOUTS: &str = "btt_layouts";
pub const HEADER: &str = "Break the Targets";
/// At most ten targets in one course.
pub const MAX: usize = 10;
/// The hot five: a stage's saved layouts used last (Recording's convention), which the chords step through.
pub const HOT: usize = 5;
/// The saved layouts' folder, one file each, under the data folder.
pub const DIR: &str = "layouts";
/// The pictures `look` returns: the targets, the break animations, the cursor.
pub const LOOKS: usize = MAX + 3;
/// The cursor's units a frame at full tilt, and the stick's dead zone (which is also what starts a run).
const SPEED: f32 = 2.0;
const DEAD: f32 = 0.3;
/// Layout ticks the placing box must stay the same, after the one that changed it, before the camera counts as settled.
const CALM: u8 = 2;
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

/// A saved layout as the game thread holds it: its file's id, its name (cut to fit) and its first `n` targets.
#[derive(Clone, Copy, Debug)]
pub struct Hot {
    pub id: FixedPath<48>,
    pub name: FixedPath<64>,
    pub n: u8,
    pub at: [[f32; 2]; MAX],
}

/// This stage's hot five, newest first, the empty ones last.
pub type Five = [Option<Hot>; HOT];

/// What the game thread asks of the writer thread, which does the file work: Save (a new file: the stage and the targets), a
/// use of a file (Load), Delete.
#[derive(Clone, Copy, Debug)]
pub enum Ask {
    Save(i32, u8, [[f32; 2]; MAX]),
    Use(FixedPath<48>),
    Delete(FixedPath<48>),
}

/// The placing box `[x, y]` in the fighters' plane: the stage's camera range `[l, r, t, b]` cut to what the camera shows (`view`)
/// less a target's radius, from 10 under the ground up to the range's top. No stage read: ±90 and 0 to 80 above the ground.
pub fn bounds(range: Option<[f32; 4]>, view: Option<View>, ground: f32) -> [[f32; 2]; 2] {
    let mut b = match range {
        Some([l, r, t, _]) => [[l, r], [ground - 10.0, t]],
        None => [[-90.0, 90.0], [ground, ground + 80.0]],
    };
    if let Some(View(s, o, _, _)) = view {
        for (i, r) in b.iter_mut().enumerate() {
            *r = [r[0].max((-CANVAS_HALF[i] - o[i]) / s[i] + RADIUS), r[1].min((CANVAS_HALF[i] - o[i]) / s[i] - RADIUS)];
            r[1] = r[1].max(r[0]);
        }
    }
    b
}
/// A saved layout's file, `layouts/<id>.json`: its name, its stage (the native id), its use number and its 1 to 10 targets.
/// Each save, load, duplicate and rename stamps the file with one more than the folder's highest use: the hot five's order,
/// with no clock (two uses in one second still order).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub schema_version: u32,
    pub name: String,
    pub stage: i32,
    #[serde(default)]
    pub used: u64,
    pub targets: Vec<[f32; 2]>,
}

/// A name's most characters (the keyboard's, as Recording's Rename).
const NAME_CHARS: usize = 48;

impl Record for Layout {
    type Error = StoreError;
    type Summary = (String, Layout);
    const SCHEMA_VERSIONS: &'static [u32] = &[1];
    const MAX_FILE_BYTES: u64 = 4096;

    fn validate(&self) -> Result<(), StoreError> {
        let name = self.name.trim();
        let named = !name.is_empty() && name.chars().count() <= NAME_CHARS && !name.chars().any(char::is_control);
        let targets = (1..=MAX).contains(&self.targets.len()) && self.targets.iter().flatten().all(|v| v.is_finite());
        if self.schema_version == 1 && named && self.stage >= 0 && targets {
            Ok(())
        } else {
            Err(StoreError::Parse(<serde_json::Error as serde::de::Error>::custom("not a target layout")))
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn summary(&self, id: &str) -> (String, Layout) {
        (id.to_owned(), self.clone())
    }
}

/// The saved layouts' folder under the data folder (`root`).
pub fn library(root: &Path) -> Store<Layout> {
    Store::new(root.join(DIR))
}

/// Every saved layout, and how many files could not be read. A missing folder is empty: whether it is there decides, not the
/// error's kind (on the console a missing file's error is not `NotFound`; Eden btt-2 and btt-4-1).
pub fn all(root: &Path) -> (Vec<(String, Layout)>, usize) {
    if !root.join(DIR).exists() {
        return (Vec::new(), 0);
    }
    library(root).list().map_or((Vec::new(), 1), |listing| (listing.entries, listing.skipped.len()))
}

/// This stage's layouts in Recording's order (`recording_menu::hot_five` and `ordered`): the five used last, newest
/// first, then the rest by name; and how many of them are the hot five.
pub fn listed(all: &[(String, Layout)], stage: i32) -> (Vec<(String, Layout)>, usize) {
    let mine: Vec<(String, Layout)> = all.iter().filter(|e| e.1.stage == stage).cloned().collect();
    let hot = hot_five(mine.iter().map(|e| (e.0.as_str(), e.1.used)));
    (ordered(&mine, &hot, |e| &e.0, |e| &e.1.name).into_iter().cloned().collect(), hot.len())
}

/// The game thread's copy of the hot five (the first `hot` of `listed`), in fixed storage.
pub fn five(listed: &[(String, Layout)], hot: usize) -> Five {
    let mut out = [None; HOT];
    for (slot, (id, layout)) in out.iter_mut().zip(listed.iter().take(hot)) {
        let mut h = Hot { id: FixedPath::new(), name: FixedPath::new(), n: layout.targets.len().min(MAX) as u8, at: [[0.0; 2]; MAX] };
        let _ = (h.id.write_str(cut(id, 48)), h.name.write_str(cut(&layout.name, 64)));
        for (d, t) in h.at.iter_mut().zip(&layout.targets) {
            *d = *t;
        }
        *slot = Some(h);
    }
    out
}

/// `s` cut to at most `max` bytes, on a character's edge.
fn cut(s: &str, max: usize) -> &str {
    let mut end = s.len().min(max);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

fn next_use(all: &[(String, Layout)]) -> u64 {
    all.iter().map(|e| e.1.used).max().unwrap_or(0) + 1
}

/// Save: `targets` on `stage` as a new file (never over another), named `Layout N · <stage>` with the lowest N this stage's
/// files leave free, and used last. Its id.
pub fn save_new(root: &Path, stage: i32, stage_name: &str, targets: &[[f32; 2]]) -> Result<String, StoreError> {
    let (all, _) = all(root);
    let names = all.iter().filter(|e| e.1.stage == stage).map(|e| e.1.name.as_str());
    let name = lowest_free(|n| format!("Layout {n} · {stage_name}"), names);
    library(root).save_new(&Layout { schema_version: 1, name, stage, used: next_use(&all), targets: targets.to_vec() })
}

/// A use of the file `id` (a load, or a rename to `name`): it becomes the newest of its stage's hot five, in its own file.
pub fn use_file(root: &Path, id: &str, name: Option<&str>) -> Result<(), StoreError> {
    let (all, lib) = (all(root).0, library(root));
    let mut layout = lib.load(id)?;
    if let Some(name) = name {
        layout.name = name.trim().to_owned();
    }
    layout.used = next_use(&all);
    lib.overwrite(id, &layout)
}

/// Duplicate: a copy of `id` as a new file, `Layout N · <stage>`, used last. The copy's id.
pub fn duplicate(root: &Path, id: &str, stage_name: &str) -> Result<String, StoreError> {
    let layout = library(root).load(id)?;
    save_new(root, layout.stage, stage_name, &layout.targets)
}

/// A count and its noun: `1 target`, `6 targets`, `1 file`.
fn counted(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

pub fn targets_label(n: usize) -> String {
    counted(n, "target", "targets")
}

/// The Saved layouts list's two footer lines: the layout under the cursor, then the keys (and the files skipped, if any).
pub fn list_footer(rows: &[(String, Layout)], cursor: usize, skipped: usize) -> String {
    let first = rows.get(cursor).map_or_else(
        || "No saved layouts on this stage yet. ZL + Down saves one while placing".to_owned(),
        |(_, l)| format!("{} · {}", l.name, targets_label(l.targets.len())),
    );
    let note = if skipped > 0 { format!("   {} skipped", counted(skipped, "file", "files")) } else { String::new() };
    format!("{first}\nA open   B back{note}")
}

/// How `projectile_breaks` searches: not at all, until the first qualifying overlap (a break only), or for the deepest (a grade).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Search {
    Off,
    First,
    Deepest,
}

/// Only P1's owned pool and ArticleModule supply objects; Off performs no discovery or hit reads. An article's hit counts as
/// `Breaks on` (`rule`, with `aerial` Player 1's own status) allows that hit (rc.15, both practice games: docs/DECISIONS.md).
pub fn projectile_breaks(
    search: Search, rule: u8, aerial: bool, spot: [f32; 2], pool: impl IntoIterator<Item = usize>, get: impl Fn(i32, i32) -> usize + Copy,
    read: impl FnMut(usize) -> Option<([eggs::Hit; super::hitbox::SLOTS as usize], usize)>,
) -> Option<f32> {
    projectile_contact(search, rule, aerial, spot, pool, get, read).map(|(_, depth)| depth)
}

/// The same overlap, with the winning article for attributing the scored hit.
pub fn projectile_contact(
    search: Search, rule: u8, aerial: bool, spot: [f32; 2], pool: impl IntoIterator<Item = usize>, get: impl Fn(i32, i32) -> usize + Copy,
    mut read: impl FnMut(usize) -> Option<([eggs::Hit; super::hitbox::SLOTS as usize], usize)>,
) -> Option<(usize, f32)> {
    if search == Search::Off {
        return None;
    }
    let mut best = None;
    for article in pool.into_iter().chain(super::hitbox::article_instances(true, get)) {
        if !pointer_ok(article) {
            continue;
        }
        let Some((hits, n)) = read(article) else { continue };
        for hit in &hits[..n] {
            if hit.3 <= 0.0 || !eggs::counts(rule, aerial, hit.3) {
                continue;
            }
            if let Some(depth) = eggs::depth(hit, spot) {
                match search {
                    Search::Off => return None,
                    Search::First => return Some((article, depth)),
                    Search::Deepest if best.is_none_or(|(_, previous)| depth > previous) => best = Some((article, depth)),
                    Search::Deepest => {}
                }
            }
        }
    }
    best
}

/// Shared placing controls, without stage bounds or projection.
pub fn placing_motion(p: &Pad, was: u16, repeat_age: &mut u8) -> [f32; 2] {
    let (tilt, unit) = placing_parts(p, was, repeat_age);
    [0, 1].map(|i| tilt[i] + unit[i])
}

// Keep the two terms so BTT retains its original (cursor + tilt) + unit rounding.
fn placing_parts(p: &Pad, was: u16, repeat_age: &mut u8) -> ([f32; 2], [f32; 2]) {
    let down = |k: u8| p.held & bit(k) != 0;
    let pad = p.held & DPAD;
    *repeat_age = repeat_age.saturating_add(1) * u8::from(pad != 0 && pad == was & DPAD);
    let repeat = *repeat_age == 0 || *repeat_age >= 15 && *repeat_age % 2 == 1;
    let step = f32::from(pad != 0 && !down(ZL) && !down(B) && repeat);
    let unit = [[DPAD_RIGHT, DPAD_LEFT], [DPAD_UP, DPAD_DOWN]].map(|[hi, lo]| step * (f32::from(down(hi)) - f32::from(down(lo))));
    let tilt = [0, 1].map(|i| p.stick[i] * SPEED * f32::from(p.stick[i].abs() >= DEAD));
    (tilt, unit)
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
    msg: (FixedPath<96>, u16),
    /// The saved layout chosen with ZL + Left / Right (an index into `five`), this stage's hot five, and the ask for the writer.
    pick: Option<usize>,
    five: Five,
    ask: Option<Ask>,
    stage: i32,
    /// What the passes and the layout tick last saw: the stage's camera range, P1's height, the camera's map; and the ground
    /// (P1's height when Placing opened).
    range: Option<[f32; 4]>,
    view: Option<View>,
    here: f32,
    ground: f32,
    /// The placing box the last layout tick's view gave, and for how many ticks running it has stayed the same (`see`).
    seen: [[f32; 2]; 2],
    calm: u8,
    kept: [(i32, [[f32; 2]; MAX], usize); KEPT],
    /// The session's best per layout (its key, frames), and the one of the run just done.
    best: [Option<(u64, u32)>; BESTS],
    top: u32,
    pub(crate) streak: super::hit_streak::Streak,
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
        five: [None; HOT],
        ask: None,
        stage: i32::MIN,
        range: None,
        view: None,
        here: 0.0,
        ground: 0.0,
        seen: [[0.0; 2]; 2],
        calm: 0,
        kept: [(i32::MIN, [[0.0; 2]; MAX], 0); KEPT],
        best: [None; BESTS],
        top: 0,
        streak: super::hit_streak::Streak::EMPTY,
        shown: FixedPath::new(),
        owner: (0, 2),
    };

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn layout(&self) -> &[[f32; 2]] {
        &self.at[..self.n]
    }

    /// Each breakable slot keeps its index as other targets are broken.
    pub fn lockon_spots(&self) -> [Option<[f32; 2]>; MAX] {
        core::array::from_fn(|i| {
            (self.phase == Phase::Running && i < self.n && self.broken >> i & 1 == 0)
                .then_some(self.at[i])
        })
    }

    pub fn cursor(&self) -> Option<[f32; 2]> {
        (self.phase == Phase::Placing).then_some(self.cursor)
    }

    /// What to draw: each unbroken target, the break animations, the cursor (the pools of `panes::EGG_POOLS`).
    pub fn look(&self) -> [Option<([f32; 2], EggLook)>; LOOKS] {
        let mut pointer = EGG_WHOLE;
        pointer.picture = EggPicture::Cursor;
        let live = (0..self.n).filter(|i| self.broken >> i & 1 == 0);
        let whole = live.map(|i| (self.at[i], EGG_WHOLE));
        let bursts = self.bursts.into_iter().flatten().map(eggs::burst);
        let cursor = self.cursor().map(|c| (c, pointer));
        let mut out = [None; LOOKS];
        for (slot, pic) in out.iter_mut().zip(whole.chain(bursts).chain(cursor)) {
            *slot = Some(pic);
        }
        out
    }

    pub fn five(&self) -> &Five {
        &self.five
    }

    /// What the writer thread is to do (a Save, a Load's use, a Delete), once.
    pub fn take_ask(&mut self) -> Option<Ask> {
        self.ask.take()
    }

    fn count(&self) -> usize {
        self.five.iter().flatten().count()
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
        let mut text = FixedPath::<96>::new();
        let _ = text.write_fmt(args);
        let yes = ask && self.msg.1 > 0 && self.msg.0.as_str() == text.as_str();
        self.msg = [(text, [SHOW, ASK][usize::from(ask)]), (FixedPath::new(), 0)][usize::from(yes)];
        yes
    }

    /// The writer thread listed this stage's layouts: its hot five. The chosen layout stays chosen while it is in the five;
    /// `saved` answers a Save, whose new file is the first of the five and is chosen.
    pub fn adopt(&mut self, five: Five, saved: bool) {
        let chosen = self.pick.and_then(|p| self.five[p]).map(|h| h.id);
        self.five = five;
        let same = chosen.and_then(|id| five.iter().position(|h| h.is_some_and(|h| h.id.as_str() == id.as_str())));
        let count = self.count();
        self.pick = if saved { Some(0) } else { same.or(self.pick.map(|p| p.min(count.saturating_sub(1)))) }.filter(|p| *p < count);
        if let Some(first) = five[0].filter(|_| saved) {
            self.say(false, format_args!("Saved: {}", first.name.as_str()));
        }
    }

    /// Every pass: the stage's camera range and P1's height. Every layout tick: the camera's map.
    pub fn sense(&mut self, range: Option<[f32; 4]>, here: f32) {
        (self.range, self.here) = (range, here);
    }

    /// The camera has settled once the placing box its view gives stays the same, to half a unit, for `CALM` ticks running: a
    /// camera on its way to the fixed view gives a new box every tick (Eden rc14-int3-rr2-f1).
    pub fn see(&mut self, view: Option<View>) {
        self.view = view;
        let b = bounds(self.range, view, self.ground);
        let same = b.iter().flatten().zip(self.seen.iter().flatten()).all(|(a, z)| (a - z).abs() < 0.5);
        (self.seen, self.calm) = (b, if same { self.calm.saturating_add(1) } else { 0 });
    }

    fn start(&mut self) {
        self.ground = self.here;
        let [x, y] = bounds(self.range, self.view, self.ground);
        (self.phase, self.broken, self.bursts) = (Phase::Placing, 0, [None; 2]);
        self.cursor = [(x[0] + x[1]) / 2.0, (self.ground + 15.0).clamp(y[0], y[1])];
    }

    fn ready(&mut self) {
        if self.streak.enabled() {
            self.streak.restart(true);
        }
        (self.phase, self.broken, self.bursts) = (Phase::Ready, 0, [None; 2]);
        (self.frames, self.left, self.still) = (0, 0, false);
    }

    /// The stage in play (every pass): a new one keeps this stage's layout for the session and takes its own, empty if it has
    /// none, and opens Ready or Placing as the layout says. True when it changed: the stage's saved layouts are to be listed.
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
        (self.stage, self.at, self.n, self.five, self.pick) = (stage, at, n, [None; HOT], None);
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
        let (zl, held_b, pick, count) = (down(ZL), down(B), self.pick.unwrap_or(0), self.count());
        self.msg.1 = self.msg.1.saturating_sub(1);
        let (tilt, unit) = placing_parts(&p, was, &mut self.rep);
        // A camera change alone never moves the cursor: it is kept in the box when it moves, and once the camera has settled, so
        // it never stays off screen either (btt-5, 11-15; Eden rc14-int3-rr2-f1 dragged an untouched cursor 100 units up).
        let clamp = tilt.iter().chain(&unit).any(|v| *v != 0.0) || self.calm >= CALM;
        for (i, range) in b.into_iter().enumerate() {
            let at = self.cursor[i] + tilt[i] + unit[i];
            self.cursor[i] = if clamp { at.clamp(range[0], range[1]) } else { at };
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
        // The saved layouts: ZL + Y twice deletes the chosen one, ZL + Left / Right step through the hot five, and the Save and
        // Load chords save a new file or load the chosen one.
        let chosen = self.five[pick];
        match chosen {
            Some(h) if pressed(Y) && zl && self.say(true, format_args!("ZL + Y again to delete {}", h.name.as_str())) => self.erase(h),
            None if pressed(Y) && zl => self.none(),
            _ if pressed(Y) && !zl && self.say(true, format_args!("Y again to clear all")) => self.n = 0,
            _ => {}
        }
        for (k, d) in [(DPAD_RIGHT, 1), (DPAD_LEFT, count.saturating_sub(1))] {
            match count {
                _ if !zl || !pressed(k) => {}
                0 => self.none(),
                // The first step shows the newest; then one along, round the five.
                _ => (self.pick, self.msg.1) = (Some(self.pick.map_or(0, |p| (p + d) % count)), 0),
            }
        }
        if p.save {
            self.save();
        }
        if p.load {
            self.load(pick);
        }
    }

    fn none(&mut self) {
        self.say(false, format_args!("No saved layouts on this stage yet"));
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

    /// Save: the layout, to a tenth, as a new file (the writer's); its answer chooses it, first of the five (`adopt`).
    fn save(&mut self) {
        if self.n == 0 {
            self.say(false, format_args!("Place at least one target"));
        } else {
            let mut at = [[0.0; 2]; MAX];
            for (d, t) in at.iter_mut().zip(self.layout()) {
                *d = t.map(|v| (v * 10.0).round() / 10.0);
            }
            self.ask = Some(Ask::Save(self.stage, self.n as u8, at));
        }
    }

    /// Load: the chosen saved layout becomes the layout, and its file is used (it goes to the top of the five).
    fn load(&mut self, pick: usize) {
        match self.five[pick] {
            Some(h) => {
                self.apply(h);
                self.ask = Some(Ask::Use(h.id));
            }
            None => self.none(),
        }
    }

    /// A saved layout becomes the layout, still Placing, to edit or start (the chord, or Load on the Saved layouts screen).
    pub fn apply(&mut self, h: Hot) {
        (self.at, self.n) = (h.at, usize::from(h.n).min(MAX));
        self.say(false, format_args!("Loaded: {}", h.name.as_str()));
    }

    /// Delete: the file goes (the writer's), and the five close up at once so a second ZL + Y cannot ask for it again.
    fn erase(&mut self, h: Hot) {
        let mut rest = self.five.into_iter().flatten().filter(|o| o.id.as_str() != h.id.as_str());
        self.five = core::array::from_fn(|_| rest.next());
        let count = self.count();
        self.pick = self.pick.filter(|p| *p < count);
        self.ask = Some(Ask::Delete(h.id));
        self.say(false, format_args!("Deleted: {}", h.name.as_str()));
    }

    /// One pass of Player 1 (`tick`: the clock's frame): the clock runs while Running, and `hit(spot)` says whether a live hit of P1's
    /// breaks the unbroken target there (`eggs::breaks`; the caller also owns the hitlag and the break animation). The last target
    /// ends the run on that frame. The number of targets broken; the notice is the caller's to take (`notice`), after the pass.
    #[cfg(test)]
    pub fn pass(&mut self, tick: bool, mut hit: impl FnMut([f32; 2]) -> bool) -> u8 {
        self.pass_indexed(tick, |_, at| hit(at))
    }

    pub fn pass_indexed(&mut self, tick: bool, mut hit: impl FnMut(usize, [f32; 2]) -> bool) -> u8 {
        let was = self.broken;
        eggs::age(&mut self.bursts, tick);
        match self.phase {
            Phase::Running => {
                self.frames += u32::from(tick);
                for i in 0..self.n {
                    if self.broken >> i & 1 == 0 && hit(i, self.at[i]) {
                        self.broken |= 1 << i;
                        self.bursts = [Some((self.at[i], 0)), self.bursts[0]];
                    }
                }
                if self.broken.count_ones() as usize == self.n {
                    self.done();
                }
            }
            Phase::Done if result_tick(&mut self.left, tick) => self.ready(),
            _ => {}
        }
        (self.broken & !was).count_ones() as u8
    }

    /// The run is over: the clock stops, and the session's best for this layout.
    fn done(&mut self) {
        (self.phase, self.left) = (Phase::Done, END_FRAMES);
        let k = key(self.layout());
        self.top = record_time(&mut self.best, k, self.frames);
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
                write!(out, "{n}/{MAX}   ")?;
                match self.pick.and_then(|p| self.five[p].map(|h| (p + 1, h))) {
                    None => write!(out, "A place  B undo  X start"),
                    Some((i, h)) => write!(out, "{i}/{} {}: {}", self.count(), h.name.as_str(), h.n),
                }
            }
            Phase::Ready | Phase::Running => {
                write!(out, "{got}/{n} {m}:{s:02}.{c:02}")?;
                self.streak.write(out)
            }
            Phase::Done => {
                let (bm, bs, bc) = clock(self.top);
                write!(out, "Done! {n} in {m}:{s:02}.{c:02} ")?;
                write!(out, "(best {bm}:{bs:02}.{bc:02})")?;
                self.streak.write(out)
            }
        }
    }
}

/// Where the mode takes the game's camera (the fixed view) and reads the stage's camera range (the normal placing box): wherever
/// the practice is live, native Training or an active waiting-room practice (`scene::is_training`; rc.14, 11-15: the LDN pass
/// showed the camera local to the console). Elsewhere the game's own camera and the fallback box.
pub const fn own_camera(facts: super::scene::Facts) -> bool {
    super::scene::is_training(facts)
}

/// Placement alone owns the zoomed-out view; all exits restore the saved native mode.
pub const fn wants_camera(phase: Phase, covered: bool, facts: super::scene::Facts) -> bool {
    matches!(phase, Phase::Placing) && !covered && own_camera(facts)
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
/// Placing): the mode the game was in when this mode first took the camera (`saved`, while it holds it) and
/// the frames left before it may ask again.
/// Lock-free owner requests; the game thread alone steps the camera state.
pub struct CameraRequests(std::sync::atomic::AtomicU8);
impl CameraRequests {
    pub const fn idle() -> Self { Self(std::sync::atomic::AtomicU8::new(0)) }
    pub fn request(&self, owner: u8, want: bool) {
        use std::sync::atomic::Ordering;
        if want { self.0.fetch_or(owner, Ordering::AcqRel); }
        else { self.0.fetch_and(!owner, Ordering::AcqRel); }
    }
    pub fn wanted(&self) -> bool { self.0.load(std::sync::atomic::Ordering::Acquire) != 0 }
}

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
    /// at once, to the mode first seen, when it is not wanted. The first mode is retained even when it was already Fixed.
    pub fn step(&mut self, want: bool, mode: Option<i32>) -> Option<i32> {
        self.cool = self.cool.saturating_sub(1);
        let Some(now) = mode else {
            self.saved = None;
            return None;
        };
        if want { self.saved.get_or_insert(now); }
        match (want, self.saved) {
            (true, _) if now != FIXED && self.cool == 0 => {
                self.cool = RETRY;
                Some(FIXED)
            }
            (false, Some(back)) => {
                self.saved = None;
                (now != back).then_some(back)
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
