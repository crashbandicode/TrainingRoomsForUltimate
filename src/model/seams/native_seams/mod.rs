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
/// Bytes compared per site.
pub const GUARD_BYTES: usize = 64;
/// Words a Skyline hook may overwrite at a site: `nop` + far jump (5 words, 20 bytes).
pub const HOOK_SPAN_WORDS: usize = 5;

/// Layout of the owned 13.0.5 `main`, from its section table (offsets from `.text`, which maps at 0):
/// `.text` is `[0, MAIN_TEXT_END)`, `.rodata` is `[MAIN_RODATA_START, MAIN_RODATA_END)`. A row's
/// region follows from its offset; host test `compared_rows_match_the_owned_elf` checks these values.
pub const MAIN_TEXT_END: usize = 0x39c_8440;
pub const MAIN_RODATA_START: usize = 0x39c_9000;
pub const MAIN_RODATA_END: usize = 0x4f1_6f60;

/// The read-only part of `main` a row's 64 bytes lie in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainRegion {
    /// Code: compared with the kind rules (another plugin's hook may be allowed there).
    Text,
    /// Read-only data the plugin relies on (switch jump tables, e.g. the Mii ordinary gate's): never
    /// hooked, so it is compared exactly.
    Rodata,
}

/// The region holding all 64 bytes at `offset`, or `None` when they are not inside one read-only section.
pub const fn region_of(offset: usize) -> Option<MainRegion> {
    match offset.checked_add(GUARD_BYTES) {
        Some(end) if end <= MAIN_TEXT_END => Some(MainRegion::Text),
        Some(end) if offset >= MAIN_RODATA_START && end <= MAIN_RODATA_END => Some(MainRegion::Rodata),
        _ => None,
    }
}

/// How this plugin writes a site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookForm {
    /// `#[skyline::hook(offset|replace = …)]` on a function: Skyline far-jumps the entry.
    Replace,
    /// `#[skyline::hook(…, inline)]`: a one-word branch (or far jump) at the site.
    Inline,
    /// A hand-made `A64HookFunction` / `A64InlineHook`.
    Raw,
    /// A text patch (`nop`, data word).
    Patch,
    /// A data pointer slot (vtable); not code, not compared byte for byte at runtime.
    VtableSlot,
}

/// Where in its function the site sits (the playbook's "hook after the prologue" audit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// The first instruction of a function. Allowed only with a justification in `Seam::note`.
    Entry,
    /// Right after the prologue.
    AfterPrologue,
    /// Inside the body.
    Mid,
    /// Not code (`.data`/`.rodata`).
    Data,
}

/// What this plugin does with the site, which decides what another plugin may have changed there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeamKind {
    /// We write here: the words our write replaces (`HOOK_SPAN_WORDS`) must be stock.
    Hook { form: HookForm, placement: Placement },
    /// We call this function: other plugins' hooks anywhere in its 64 bytes are fine.
    Called,
    /// We rely on this instruction: its own word must stay; hooks after it are fine.
    Relied,
}

/// How the site is shared with other plugins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sharing {
    None,
    /// Another plugin may own the entry; then we use the first body word that still runs when it
    /// calls the original (`live_replay` 0x143e2a0 / 0x143e2b0 with smash_replay_to_mp4). `body` is the
    /// body site's offset (a row of its own). The entry row is usable `Stock` (we take the entry) or
    /// `Shared`: a far jump at the entry (any plugin's, `owner` in particular) that ends before `body`,
    /// with every other word of the 64 bytes stock (we take the body). A one-word branch, a far jump
    /// reaching `body`, or any other change stays unusable (`entry_or_body_outcome`).
    EntryOrBody { owner: &'static str, body: usize },
    /// We chain onto another plugin's far jump at the entry (`effect_hook_chain`).
    Chain,
    /// Listed but not converted, with the reason. No row uses it since the CPU-as-P1 merge guarded
    /// `cpu_pilot` (WORKPLAN §2.9's boundary, WORKAROUND-DECISIONS row 40).
    External { reason: &'static str },
    /// The site is also the entry of this plugin's own replace hook `hook` (a
    /// `Hook { form: Replace, placement: Entry }` row at the same offset), installed before this row is
    /// checked. Skyline's far jump there whose `.quad` target is the replacement address the installer
    /// registered for `hook` (`outcome_for_own`), with the rest of the 64 bytes stock, is our own write:
    /// `Stock` for this row. Any other far jump or branch keeps the kind rule's outcome.
    OwnHook { hook: &'static str },
}

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
const TABLES: [&[Seam]; 6] =
    [rows_a1::ROWS, rows_a2::ROWS, rows_a3::ROWS, rows_a4::ROWS, rows_a5::ROWS, rows_identity::ROWS];

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

/// Whether a far jump (`ldr x17, #8; br x17`, optionally after a `nop`) starts at word `word`.
pub fn far_jump_at(actual: &[u8; GUARD_BYTES], word: usize) -> bool {
    let words = words(actual);
    let at = |k: usize| words.get(word + k).copied();
    (at(0) == Some(LDR_X17_LITERAL_8) && at(1) == Some(BR_X17))
        || (at(0) == Some(NOP) && at(1) == Some(LDR_X17_LITERAL_8) && at(2) == Some(BR_X17))
}

/// `guard_outcome` plus the row's sharing rule (WORKAROUND-DECISIONS row 28): a `Sharing::Chain`
/// hook chains onto another plugin's far jump at its entry (`effect_hook_chain`), so that case is
/// usable; a one-word branch or anything else at the entry is not.
pub fn outcome_for(
    expected: &[u8; GUARD_BYTES],
    actual: &[u8; GUARD_BYTES],
    kind: SeamKind,
    sharing: Sharing,
) -> GuardOutcome {
    let outcome = guard_outcome(expected, actual, kind);
    match (sharing, outcome) {
        (Sharing::Chain, GuardOutcome::HookedByOther { at: 0, usable: false })
            if far_jump_at(actual, 0) && guard_outcome(expected, &rest_from(actual, expected), kind).usable() =>
        {
            GuardOutcome::HookedByOther { at: 0, usable: true }
        }
        _ => outcome,
    }
}

/// The `Sharing::EntryOrBody` rule for an entry row whose body site is `body_word` words after it:
/// `Stock` (we take the entry), `Shared` when a far jump at the entry (any plugin's) ends at or before
/// the body word and every other word of the 64 bytes is stock (we take the body, which still runs when
/// that plugin calls the original), otherwise the kind rule's outcome (a one-word branch at the entry,
/// a far jump reaching the body, or any other change stays unusable for an entry hook).
pub fn entry_or_body_outcome(
    expected: &[u8; GUARD_BYTES],
    actual: &[u8; GUARD_BYTES],
    kind: SeamKind,
    body_word: usize,
) -> GuardOutcome {
    let outcome = guard_outcome(expected, actual, kind);
    let jump_words = if words(actual)[0] == NOP { 5 } else { 4 };
    match outcome {
        GuardOutcome::HookedByOther { at: 0, .. }
            if far_jump_at(actual, 0)
                && (jump_words..WORDS).contains(&body_word)
                && rest_from(actual, expected) == *expected =>
        {
            GuardOutcome::Shared
        }
        _ => outcome,
    }
}

/// What a site's first words hold, for the diagnostic stand-in drill's `<site>=<entry>/<outcome>`:
/// the 13.0.5 words, a one-word branch, a far jump, or something else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryForm {
    Stock,
    Branch,
    FarJump,
    Changed,
}

impl EntryForm {
    pub const fn name(self) -> &'static str {
        match self {
            EntryForm::Stock => "stock",
            EntryForm::Branch => "branch",
            EntryForm::FarJump => "far",
            EntryForm::Changed => "changed",
        }
    }
}

/// The form at word 0 of `actual` against the 13.0.5 bytes `expected`.
pub fn entry_form(expected: &[u8; GUARD_BYTES], actual: &[u8; GUARD_BYTES]) -> EntryForm {
    let first = words(actual)[0];
    if far_jump_at(actual, 0) {
        EntryForm::FarJump
    } else if first != words(expected)[0] && first & B_MASK == B_OPCODE {
        EntryForm::Branch
    } else if actual == expected {
        EntryForm::Stock
    } else {
        EntryForm::Changed
    }
}

/// `actual` with the entry far jump (and its `nop`) replaced by the expected words, to check the rest.
fn rest_from(actual: &[u8; GUARD_BYTES], expected: &[u8; GUARD_BYTES]) -> [u8; GUARD_BYTES] {
    let len = (if words(actual)[0] == NOP { 5 } else { 4 }) * 4;
    let mut out = *actual;
    out[..len].copy_from_slice(&expected[..len]);
    out
}

/// The `.quad` target of a far jump at the entry: bytes 8..16 after `ldr x17, #8; br x17`, or 12..20
/// when a `nop` aligns it (Skyline / And64InlineHook writes the replacement address there).
pub fn entry_far_jump_target(actual: &[u8; GUARD_BYTES]) -> Option<u64> {
    if !far_jump_at(actual, 0) {
        return None;
    }
    let at = if words(actual)[0] == NOP { 12 } else { 8 };
    let mut quad = [0u8; 8];
    quad.copy_from_slice(&actual[at..at + 8]);
    Some(u64::from_le_bytes(quad))
}

/// `outcome_for` plus the `Sharing::OwnHook` rule. `own_target` is the replacement address the
/// installer registered when it installed the named hook (`None`: not installed). A far jump at the
/// entry to exactly that address, with the rest of the 64 bytes stock, is our own write: `Stock`.
/// Anything else (another target, a one-word branch, a further hook or change in the rest, or no
/// registration) keeps `outcome_for`'s answer.
pub fn outcome_for_own(
    expected: &[u8; GUARD_BYTES],
    actual: &[u8; GUARD_BYTES],
    kind: SeamKind,
    sharing: Sharing,
    own_target: Option<u64>,
) -> GuardOutcome {
    match (sharing, own_target) {
        (Sharing::OwnHook { .. }, Some(target))
            if entry_far_jump_target(actual) == Some(target)
                && guard_outcome(expected, &rest_from(actual, expected), kind) == GuardOutcome::Stock =>
        {
            GuardOutcome::Stock
        }
        _ => outcome_for(expected, actual, kind, sharing),
    }
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

/// Byte-exact comparison: `Stock`, or `Foreign` at the first differing byte.
pub fn exact_outcome(expected: &[u8; GUARD_BYTES], actual: &[u8; GUARD_BYTES]) -> GuardOutcome {
    match expected.iter().zip(actual.iter()).position(|(e, a)| e != a) {
        None => GuardOutcome::Stock,
        Some(first_diff) => GuardOutcome::Foreign { first_diff },
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

/// The 64-byte comparison of one site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardOutcome {
    /// Identical to 13.0.5.
    Stock,
    /// Only Skyline hook shapes differ. `at` is the byte offset of the first; `usable` applies the
    /// kind's rule (false: another plugin owns the words we would replace or rely on).
    HookedByOther { at: usize, usable: bool },
    /// A `Sharing::EntryOrBody` entry row whose entry another plugin's far jump owns, with the body
    /// after it stock: usable, through the body site (`entry_or_body_outcome`).
    Shared,
    /// A change that is not a hook: the site is left alone.
    Foreign { first_diff: usize },
}

impl GuardOutcome {
    pub const fn usable(self) -> bool {
        match self {
            GuardOutcome::Stock | GuardOutcome::Shared => true,
            GuardOutcome::HookedByOther { usable, .. } => usable,
            GuardOutcome::Foreign { .. } => false,
        }
    }

    /// The install line's word for the outcome.
    pub const fn code(self) -> &'static str {
        match self {
            GuardOutcome::Stock => "stock",
            GuardOutcome::HookedByOther { usable: true, .. } => "other",
            GuardOutcome::HookedByOther { usable: false, .. } => "occupied",
            GuardOutcome::Shared => "shared",
            GuardOutcome::Foreign { .. } => "foreign",
        }
    }
}

const WORDS: usize = GUARD_BYTES / 4;
const NOP: u32 = 0xd503_201f;
const LDR_X17_LITERAL_8: u32 = 0x5800_0051;
const BR_X17: u32 = 0xd61f_0220;
const B_MASK: u32 = 0xfc00_0000;
const B_OPCODE: u32 = 0x1400_0000;

fn words(bytes: &[u8; GUARD_BYTES]) -> [u32; WORDS] {
    let mut out = [0u32; WORDS];
    for (i, word) in out.iter_mut().enumerate() {
        *word = u32::from_le_bytes([bytes[4 * i], bytes[4 * i + 1], bytes[4 * i + 2], bytes[4 * i + 3]]);
    }
    out
}

/// Length in words of a Skyline hook shape starting at word `i` (cut at the end of the window).
fn hook_shape(actual: &[u32; WORDS], i: usize) -> Option<usize> {
    let at = |k: usize| actual.get(i + k).copied();
    let len = if at(0) == Some(NOP) && at(1) == Some(LDR_X17_LITERAL_8) && at(2) == Some(BR_X17) {
        5
    } else if at(0) == Some(LDR_X17_LITERAL_8) && at(1) == Some(BR_X17) {
        4
    } else if actual[i] & B_MASK == B_OPCODE {
        1
    } else {
        return None;
    };
    Some(len.min(WORDS - i))
}

/// Whether a hook shape at word `word` takes words this kind needs as they are.
const fn blocks(kind: SeamKind, word: usize) -> bool {
    match kind {
        SeamKind::Hook { .. } => word < HOOK_SPAN_WORDS,
        SeamKind::Called => false,
        SeamKind::Relied => word == 0,
    }
}

/// Compare a site's 64 bytes now (`actual`) with its 13.0.5 bytes (`expected`).
pub fn guard_outcome(expected: &[u8; GUARD_BYTES], actual: &[u8; GUARD_BYTES], kind: SeamKind) -> GuardOutcome {
    let expected = words(expected);
    let actual = words(actual);
    let mut first_hook = None;
    let mut usable = true;
    let mut i = 0;
    while i < WORDS {
        if actual[i] == expected[i] {
            i += 1;
            continue;
        }
        let Some(len) = hook_shape(&actual, i) else {
            return GuardOutcome::Foreign { first_diff: 4 * i };
        };
        first_hook.get_or_insert(4 * i);
        // A shape that starts inside the span also covers every word it spans.
        if (i..i + len).any(|w| blocks(kind, w)) {
            usable = false;
        }
        i += len;
    }
    match first_hook {
        None => GuardOutcome::Stock,
        Some(at) => GuardOutcome::HookedByOther { at, usable },
    }
}
