//! The guarded native-seam table (WP-A, WORKPLAN §2.7; playbook "Guard, then hook" and
//! HOOK-SHARING). One row per main-image site this plugin hooks, patches, calls or relies on, with
//! the site's 64 bytes from the owned 13.0.5 `main`, so the Switch installer
//! (`src/swap_native/seam_install.rs`) can compare every site before it is used, leave a site that
//! does not match alone, and log the outcome per site in one install line.
//!
//! Pure data and pure rules: nothing here reads the game. The rows live in one file per WP-A
//! module group (`rows_a1.rs` … `rows_a5.rs`, owned by WP-A1 … WP-A5), so the five conversions
//! never edit the same file. FACTs the rules encode:
//! - Skyline writes a hook as a one-word `b`, or as a far jump `ldr x17, #8; br x17; .quad target`
//!   (16 bytes), preceded by a `nop` (20 bytes) when the site is 4 past an 8-byte boundary
//!   (HOOK-SHARING "Skyline's two forms"; seen in Eden 2026-10-30).
//! - Another plugin's hook is allowed past the words our own write replaces, anywhere in a function
//!   we only call, and after the one word we rely on; anything else that differs is foreign
//!   (HOOK-SHARING "Guards").
//! - The table was read from the 13.0.5 main whose build ID is `TABLE_BUILD_ID`; the version guard
//!   in the root crate's `main` refuses any other game before a row is consulted (WP-B).

pub mod rows_a1;
pub mod rows_a2;
pub mod rows_a3;
pub mod rows_a4;
pub mod rows_a5;
/// WP-R: the per-body identity view's hook sites (`src/swap_native/identity_view.rs`).
pub mod rows_identity;
pub mod rows_q15;
/// Phase 2 N11: Lobby Items' spawner switch hook and the instructions it relies on (`src/runtime/lobby/lobby_items.rs`).
pub mod rows_lobby_items;
/// rc.6: the waiting room's practice panel, saved Training partners and behavior (`src/runtime/lobby/lobby_persist.rs`).
pub mod rows_lobby_persist;
/// rc.6: the mod's CPU level, the rank setter it calls and the native rank writers it follows
/// (`src/runtime/features/cpu_level.rs`).
pub mod rows_cpu_level;
/// rc.6, operator option C: the release watch's read of the game's resource unload (`src/runtime/roster/res_unload.rs`).
pub mod rows_res_unload;
/// rc.8: the game speed bind, the Training menu's own store-and-text path and the slow table's steps
/// (`src/runtime/features/game_speed.rs`).
pub mod rows_game_speed;
/// The installer's outcome-slot bits and its late report (`[TrainingSeams] late …`, R-A0b).
pub mod report;
/// The hook-sharing drill's setting and sites (the diagnostic build's stand-ins).
pub mod drill;

#[cfg(test)]
mod tests;

/// Game version the rows were read from.
pub const TABLE_VERSION: &str = "13.0.5";
/// NSO build ID of the `main` the rows were read from.
pub const TABLE_BUILD_ID: &str = "21450c647b8c5940ed5b68218ea128fd3c5d597d";
/// The guard rules and types are `model::guard`'s (one rule set for every site table).
pub use crate::model::guard::{
    entry_far_jump_target, entry_form, entry_or_body_outcome, exact_outcome, far_jump_at, guard_outcome, outcome_for,
    outcome_for_own, region_of, EntryForm, GuardOutcome, HookForm, MainRegion, Placement, SeamKind, Sharing, BR_X17,
    GUARD_BYTES, HOOK_SPAN_WORDS, LDR_X17_LITERAL_8, MAIN_RODATA_END, MAIN_RODATA_START, MAIN_TEXT_END, NOP, WORDS,
};

/// Where a callback may run (WP-E reviews and fills these; WP-E's `Scope` maps onto the bits).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scopes(pub u8);

impl Scopes {
    pub const NONE: Scopes = Scopes(0);
    pub const TRAINING: Scopes = Scopes(1);
    pub const LOBBY_WAITING: Scopes = Scopes(2);
    pub const MATCH: Scopes = Scopes(4);
    pub const ONLINE: Scopes = Scopes(8);
    pub const ALL: Scopes = Scopes(15);

    pub const fn with(self, other: Scopes) -> Scopes {
        Scopes(self.0 | other.0)
    }

    pub const fn contains(self, other: Scopes) -> bool {
        self.0 & other.0 == other.0
    }
}

/// One guarded site.
#[derive(Clone, Copy, Debug)]
pub struct Seam {
    /// Unique `<module>.<site>`; logged in the install line. Needle sites are `needle.<name>`.
    pub label: &'static str,
    /// The shadow/authoritative flip unit: the Rust module that installs the site.
    pub group: &'static str,
    /// Offset from the start of main `.text` (13.0.5 vaddr; `.text` maps at 0). A `.rodata` row uses
    /// the same base; `Seam::region` follows from it.
    pub offset: usize,
    pub kind: SeamKind,
    /// The owned 13.0.5 bytes at `offset`.
    pub guard: [u8; GUARD_BYTES],
    /// Cargo feature that compiles the site; "" = every build.
    pub feature: &'static str,
    pub scopes: Scopes,
    /// The callback writes no simulated state.
    pub read_only: bool,
    pub per_frame: bool,
    pub sharing: Sharing,
    /// Placement justification (required for `Placement::Entry` hooks) and the FACT it rests on.
    pub note: &'static str,
}

impl Seam {
    /// The run-time view of this row (`SITES` holds one per row, in table order).
    pub const fn site(&self) -> Site {
        Site {
            label: self.label,
            group: self.group,
            offset: self.offset,
            kind: self.kind,
            guard: self.guard,
            chain: matches!(self.sharing, Sharing::Chain),
            own_hook_row: matches!(self.sharing, Sharing::OwnHook { .. }),
            entry_or_body_row: matches!(self.sharing, Sharing::EntryOrBody { .. }),
        }
    }

    /// Rows the installer compares byte for byte at runtime (code, not data slots).
    pub const fn compared(&self) -> bool {
        self.site().compared()
    }
}

/// What the Switch installer reads of one `main` row: the guard data and the install line's label.
/// The authored rows (`Seam`, in the row files) also carry review text and WP-E's review fields
/// (`note`, `feature`, `scopes`, `read_only`, `per_frame`, the `Sharing` reasons); only host tests read
/// those, so the run-time table is this projection, built at compile time from the same rows. The
/// text then never reaches the NRO, whose resident size the game's heap pays on every boot (U17).
#[derive(Clone, Copy, Debug)]
pub struct Site {
    /// The row's unique label, logged in the install line.
    pub label: &'static str,
    /// The shadow/authoritative flip unit.
    pub group: &'static str,
    /// Offset from the start of main `.text`; `region()` follows from it.
    pub offset: usize,
    pub kind: SeamKind,
    /// The owned 13.0.5 bytes at `offset`.
    pub guard: [u8; GUARD_BYTES],
    /// `Sharing::Chain` (`outcome_for`).
    pub chain: bool,
    /// `Sharing::OwnHook` (`outcome_for_own`); the hook it names is in `OWN_HOOK_PAIRS`. A flag in the
    /// struct's padding rather than a label per row, so the run-time table does not grow.
    pub own_hook_row: bool,
    /// `Sharing::EntryOrBody` (`entry_or_body_outcome`); its owner and body offset are in
    /// `ENTRY_OR_BODY_ROWS`. A flag in the padding, like `own_hook_row`.
    pub entry_or_body_row: bool,
}

impl Site {
    const EMPTY: Site = Site {
        label: "",
        group: "",
        offset: 0,
        kind: SeamKind::Called,
        guard: [0; GUARD_BYTES],
        chain: false,
        own_hook_row: false,
        entry_or_body_row: false,
    };

    /// The label of our own hook row at this address, for a `Sharing::OwnHook` row.
    pub fn own_hook(&self) -> Option<&'static str> {
        if !self.own_hook_row {
            return None;
        }
        OWN_HOOK_PAIRS.iter().find(|(row, _)| *row == self.label).map(|(_, hook)| *hook)
    }

    /// (owner, body offset) of a `Sharing::EntryOrBody` row.
    pub fn entry_or_body(&self) -> Option<(&'static str, usize)> {
        if !self.entry_or_body_row {
            return None;
        }
        ENTRY_OR_BODY_ROWS.iter().find(|(row, _, _)| *row == self.label).map(|(_, owner, body)| (*owner, *body))
    }

    /// Rows the installer compares byte for byte at runtime (code, not data slots).
    pub const fn compared(&self) -> bool {
        !matches!(
            self.kind,
            SeamKind::Hook { form: HookForm::VtableSlot, .. } | SeamKind::Hook { placement: Placement::Data, .. }
        )
    }

    /// The sharing rule as `outcome_for` / `outcome_for_own` / `entry_or_body_outcome` read it.
    pub fn sharing(&self) -> Sharing {
        if let Some((owner, body)) = self.entry_or_body() {
            return Sharing::EntryOrBody { owner, body };
        }
        match (self.own_hook(), self.chain) {
            (Some(hook), _) => Sharing::OwnHook { hook },
            (None, true) => Sharing::Chain,
            (None, false) => Sharing::None,
        }
    }
}

/// Every row file, in table order: the one list a new row file joins (`COUNT`, `SITES`, and the host
/// tests' `all()` read it).
const TABLES: [&[Seam]; 12] = [
    rows_a1::ROWS, rows_a2::ROWS, rows_a3::ROWS, rows_a4::ROWS, rows_a5::ROWS, rows_identity::ROWS, rows_q15::ROWS,
    rows_lobby_items::ROWS, rows_lobby_persist::ROWS, rows_cpu_level::ROWS, rows_res_unload::ROWS,
    rows_game_speed::ROWS,
];

const fn total(tables: &[&[Seam]]) -> usize {
    let (mut n, mut t) = (0, 0);
    while t < tables.len() {
        n += tables[t].len();
        t += 1;
    }
    n
}

const fn project(tables: &[&[Seam]]) -> [Site; COUNT] {
    let mut out = [Site::EMPTY; COUNT];
    let (mut t, mut k) = (0, 0);
    while t < tables.len() {
        let rows = tables[t];
        let mut i = 0;
        while i < rows.len() {
            out[k] = rows[i].site();
            i += 1;
            k += 1;
        }
        t += 1;
    }
    out
}

/// The run-time table: one `Site` per row, in table order (the index `find_site` returns and the
/// installer's outcome slots use).
pub static SITES: [Site; COUNT] = project(&TABLES);

/// A row's run-time view by label, with its global index (stable for one build).
pub fn find_site(label: &str) -> Option<(usize, &'static Site)> {
    SITES.iter().enumerate().find(|(_, site)| site.label == label)
}

/// The authored table (host tests): one slice per row file.
#[cfg(test)]
pub static SEAMS: &[&[Seam]] = &TABLES;

/// The ratchet of `#[skyline::hook]` attributes without a row yet (empty at M5).
pub static UNCONVERTED: &[&[&str]] = &[
    rows_a1::UNCONVERTED,
    rows_a2::UNCONVERTED,
    rows_a3::UNCONVERTED,
    rows_a4::UNCONVERTED,
    rows_a5::UNCONVERTED,
    rows_identity::UNCONVERTED,
];

/// Number of `main` rows, a compile-time constant: the installer sizes its outcome table with it, so
/// no row can fall outside the table (WORKAROUND-DECISIONS: the former fixed 512 < 608 rows).
pub const COUNT: usize = total(&TABLES);

const fn own_hook_rows(tables: &[&[Seam]]) -> usize {
    let (mut n, mut t) = (0, 0);
    while t < tables.len() {
        let rows = tables[t];
        let mut i = 0;
        while i < rows.len() {
            if matches!(rows[i].sharing, Sharing::OwnHook { .. }) {
                n += 1;
            }
            i += 1;
        }
        t += 1;
    }
    n
}

/// Rows with `Sharing::OwnHook`. The installer sizes its table of registered replacement addresses
/// with it (one slot per such row is enough: each names one hook).
pub const OWN_HOOK_ROWS: usize = own_hook_rows(&TABLES);

const fn own_hook_pairs(tables: &[&[Seam]]) -> [(&'static str, &'static str); OWN_HOOK_ROWS] {
    let mut out = [("", ""); OWN_HOOK_ROWS];
    let (mut n, mut t) = (0, 0);
    while t < tables.len() {
        let rows = tables[t];
        let mut i = 0;
        while i < rows.len() {
            if let Sharing::OwnHook { hook } = rows[i].sharing {
                out[n] = (rows[i].label, hook);
                n += 1;
            }
            i += 1;
        }
        t += 1;
    }
    out
}

/// (row label, own hook label) of every `Sharing::OwnHook` row, in table order: the run-time view of
/// that rule (`Site::own_hook`, and the installer's registration of the hook's replacement address).
pub static OWN_HOOK_PAIRS: [(&str, &str); OWN_HOOK_ROWS] = own_hook_pairs(&TABLES);

/// A compared hook row: a site in code this plugin writes (`model::own_patches`).
const fn writes_code(seam: &Seam) -> bool {
    matches!(seam.kind, SeamKind::Hook { .. }) && seam.compared() && matches!(region_of(seam.offset), Some(MainRegion::Text))
}

const fn patch_rows(tables: &[&[Seam]]) -> usize {
    let (mut n, mut t) = (0, 0);
    while t < tables.len() {
        let rows = tables[t];
        let mut i = 0;
        while i < rows.len() {
            if writes_code(&rows[i]) {
                n += 1;
            }
            i += 1;
        }
        t += 1;
    }
    n
}

/// Compared hook rows in code: the installer records the bytes at each (`seam_install::prime` / `seal`).
pub const PATCH_ROWS: usize = patch_rows(&TABLES);

const fn patch_sites(tables: &[&[Seam]]) -> [usize; PATCH_ROWS] {
    let mut out = [0; PATCH_ROWS];
    let (mut n, mut k, mut t) = (0, 0, 0);
    while t < tables.len() {
        let rows = tables[t];
        let mut i = 0;
        while i < rows.len() {
            if writes_code(&rows[i]) {
                out[n] = k;
                n += 1;
            }
            i += 1;
            k += 1;
        }
        t += 1;
    }
    out
}

/// The global index (`SITES`) of every compared hook row in code, in table order.
pub static PATCH_SITES: [usize; PATCH_ROWS] = patch_sites(&TABLES);

const fn entry_or_body_count(tables: &[&[Seam]]) -> usize {
    let (mut n, mut t) = (0, 0);
    while t < tables.len() {
        let rows = tables[t];
        let mut i = 0;
        while i < rows.len() {
            if matches!(rows[i].sharing, Sharing::EntryOrBody { .. }) {
                n += 1;
            }
            i += 1;
        }
        t += 1;
    }
    n
}

/// Rows with `Sharing::EntryOrBody`.
pub const ENTRY_OR_BODY_COUNT: usize = entry_or_body_count(&TABLES);

const fn entry_or_body_rows(tables: &[&[Seam]]) -> [(&'static str, &'static str, usize); ENTRY_OR_BODY_COUNT] {
    let mut out = [("", "", 0); ENTRY_OR_BODY_COUNT];
    let (mut n, mut t) = (0, 0);
    while t < tables.len() {
        let rows = tables[t];
        let mut i = 0;
        while i < rows.len() {
            if let Sharing::EntryOrBody { owner, body } = rows[i].sharing {
                out[n] = (rows[i].label, owner, body);
                n += 1;
            }
            i += 1;
        }
        t += 1;
    }
    out
}

/// (row label, owner, body offset) of every `Sharing::EntryOrBody` row, in table order: the run-time
/// view of that rule (`Site::entry_or_body`).
pub static ENTRY_OR_BODY_ROWS: [(&str, &str, usize); ENTRY_OR_BODY_COUNT] = entry_or_body_rows(&TABLES);

/// Every authored row, in table order (the global index of `find`). Host tests only: the NRO reads `SITES`.
#[cfg(test)]
pub fn all() -> impl Iterator<Item = &'static Seam> {
    SEAMS.iter().flat_map(|rows| rows.iter())
}

#[cfg(test)]
pub fn count() -> usize {
    SEAMS.iter().map(|rows| rows.len()).sum()
}

/// The loaded image a site lives in (WORKAROUND-DECISIONS rows 27/36: sites outside `main` belong in the
/// table too). `Seam` rows are always `Image::Main`; `ImageSeam` rows name their image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Image {
    Main,
    /// `lua2cpp_common.nro`, loaded per scene (the three `L2CFighter*` replace hooks).
    CommonNro,
    /// The game's `sdk` module (nn functions).
    Sdk,
    /// Another NRO, by module name (item and fighter modules).
    Nro(&'static str),
}

impl Image {
    /// The image's name in the install line (`images=`) and the late report (`late image=`).
    pub const fn name(self) -> &'static str {
        match self {
            Image::Main => "main",
            Image::CommonNro => "common",
            Image::Sdk => "sdk",
            Image::Nro(name) => name,
        }
    }
}

/// Where in its image a non-`main` site is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locator {
    /// Offset from the image's `.text` start.
    Offset(usize),
    /// A symbol resolved at run time (`nn::ro::LookupSymbol`): images we hold no owned dump of.
    Symbol(&'static str),
}

/// A guarded site outside `main`. `guard` is `None` until its 64 bytes are captured: the installer
/// then logs `[TrainingModpack] seam=<label> image=<image> capture=<128 hex digits>` once, and the
/// owning A-WP copies the bytes into the row. An uncaptured row is reported `uncaptured`, never
/// `stock`, and is never usable by an authoritative group.
#[derive(Clone, Copy, Debug)]
pub struct ImageSeam {
    pub label: &'static str,
    pub group: &'static str,
    pub image: Image,
    pub locator: Locator,
    pub kind: SeamKind,
    pub guard: Option<[u8; GUARD_BYTES]>,
    pub feature: &'static str,
    pub scopes: Scopes,
    pub read_only: bool,
    pub per_frame: bool,
    pub sharing: Sharing,
    pub note: &'static str,
}

pub mod images;

/// What the Switch installer reads of one non-`main` row (the `Site` projection of an `ImageSeam`).
#[derive(Clone, Copy, Debug)]
pub struct ImageSite {
    pub label: &'static str,
    pub group: &'static str,
    pub image: Image,
    pub locator: Locator,
    pub kind: SeamKind,
    pub guard: Option<[u8; GUARD_BYTES]>,
    /// `Sharing::Chain`: the only sharing rule the guard applies (`outcome_for`).
    pub chain: bool,
}

impl ImageSite {
    const EMPTY: ImageSite = ImageSite {
        label: "",
        group: "",
        image: Image::Main,
        locator: Locator::Offset(0),
        kind: SeamKind::Called,
        guard: None,
        chain: false,
    };

    /// The sharing rule as `outcome_for` reads it.
    pub const fn sharing(&self) -> Sharing {
        if self.chain { Sharing::Chain } else { Sharing::None }
    }
}

impl ImageSeam {
    /// The run-time view of this row (`IMAGE_SITES` holds one per row, in `IMAGE_ROWS` order).
    pub const fn site(&self) -> ImageSite {
        ImageSite {
            label: self.label,
            group: self.group,
            image: self.image,
            locator: self.locator,
            kind: self.kind,
            guard: self.guard,
            chain: matches!(self.sharing, Sharing::Chain),
        }
    }
}

/// Number of non-`main` rows (the installer's second outcome table).
pub const IMAGE_COUNT: usize = images::IMAGE_ROWS.len();

const fn project_images(rows: &[ImageSeam]) -> [ImageSite; IMAGE_COUNT] {
    let mut out = [ImageSite::EMPTY; IMAGE_COUNT];
    let mut i = 0;
    while i < rows.len() {
        out[i] = rows[i].site();
        i += 1;
    }
    out
}

/// The run-time table of non-`main` rows, in `images::IMAGE_ROWS` order.
pub static IMAGE_SITES: [ImageSite; IMAGE_COUNT] = project_images(images::IMAGE_ROWS);

/// A non-`main` row's run-time view by label, with its index in `IMAGE_SITES`.
pub fn find_image(label: &str) -> Option<(usize, &'static ImageSite)> {
    IMAGE_SITES.iter().enumerate().find(|(_, site)| site.label == label)
}

impl Site {
    /// The read-only region of `main` this row's 64 bytes lie in (from its offset).
    pub const fn region(&self) -> Option<MainRegion> {
        region_of(self.offset)
    }

    /// This row's outcome for the bytes now at its site: kind rule + sharing rule for code, an exact
    /// comparison for `.rodata` (no plugin hooks read-only data, so any difference is foreign).
    pub fn outcome(&self, actual: &[u8; GUARD_BYTES]) -> GuardOutcome {
        self.outcome_with(actual, None)
    }

    /// `outcome` with the registered replacement address of this row's own hook (`Sharing::OwnHook`).
    pub fn outcome_with(&self, actual: &[u8; GUARD_BYTES], own_target: Option<u64>) -> GuardOutcome {
        match (self.region(), self.entry_or_body()) {
            (Some(MainRegion::Rodata), _) => exact_outcome(&self.guard, actual),
            (_, Some((_, body))) if body > self.offset && (body - self.offset).is_multiple_of(4) => {
                entry_or_body_outcome(&self.guard, actual, self.kind, (body - self.offset) / 4)
            }
            _ => outcome_for_own(&self.guard, actual, self.kind, self.sharing(), own_target),
        }
    }
}

impl Seam {
    /// The read-only region of `main` this row's 64 bytes lie in (from its offset).
    pub const fn region(&self) -> Option<MainRegion> {
        region_of(self.offset)
    }

    /// This row's outcome for the bytes now at its site (the run-time view's rule, `Site::outcome`).
    pub fn outcome(&self, actual: &[u8; GUARD_BYTES]) -> GuardOutcome {
        self.site().outcome(actual)
    }
}

/// An authored row by label, with its global index (stable for one build). Host tests only.
#[cfg(test)]
pub fn find(label: &str) -> Option<(usize, &'static Seam)> {
    all().enumerate().find(|(_, seam)| seam.label == label)
}

/// Offsets for `tools/build/scan-offsets.py` (the playbook card scan), in table order, deduplicated.
#[cfg(test)]
pub fn scan_offsets() -> impl Iterator<Item = usize> {
    all().enumerate().filter_map(|(i, seam)| {
        let first = all().position(|other| other.offset == seam.offset) == Some(i);
        first.then_some(seam.offset)
    })
}
