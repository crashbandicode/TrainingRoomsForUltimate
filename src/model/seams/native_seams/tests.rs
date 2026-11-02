//! Host tests of the seam table and the 64-byte rule (WP-A core).

use super::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const HOOK: SeamKind = SeamKind::Hook { form: HookForm::Replace, placement: Placement::Entry };
const INLINE: SeamKind = SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid };

/// 16 distinct non-hook words (`add x<i>, x<i>, #<i>` style, never `b`/`ldr x17`/`br`/`nop`).
fn stock() -> [u8; GUARD_BYTES] {
    let mut bytes = [0u8; GUARD_BYTES];
    for i in 0..WORDS {
        let word = 0x9100_0000u32 | ((i as u32) << 10) | (i as u32);
        bytes[4 * i..4 * i + 4].copy_from_slice(&word.to_le_bytes());
    }
    bytes
}

fn with_words(mut bytes: [u8; GUARD_BYTES], at_word: usize, new: &[u32]) -> [u8; GUARD_BYTES] {
    for (k, word) in new.iter().enumerate() {
        let i = at_word + k;
        if i < WORDS {
            bytes[4 * i..4 * i + 4].copy_from_slice(&word.to_le_bytes());
        }
    }
    bytes
}

const B: u32 = 0x1400_1234;
const FAR: [u32; 4] = [LDR_X17_LITERAL_8, BR_X17, 0x8100_0000, 0x0000_0000];
const NOP_FAR: [u32; 5] = [NOP, LDR_X17_LITERAL_8, BR_X17, 0x8100_0000, 0x0000_0000];
/// The `.quad` both far jumps above carry (little endian words 0x8100_0000, 0).
const FAR_TARGET: u64 = 0x8100_0000;

#[test]
fn identical_bytes_are_stock_for_every_kind() {
    for kind in [HOOK, INLINE, SeamKind::Called, SeamKind::Relied] {
        assert_eq!(guard_outcome(&stock(), &stock(), kind), GuardOutcome::Stock);
    }
    assert!(GuardOutcome::Stock.usable());
    assert_eq!(GuardOutcome::Stock.code(), "stock");
}

#[test]
fn another_plugins_hook_in_the_words_we_replace_occupies_a_hook_site() {
    for (shape, len) in [(&[B][..], 1), (&FAR[..], 4), (&NOP_FAR[..], 5)] {
        let actual = with_words(stock(), 0, shape);
        let outcome = guard_outcome(&stock(), &actual, HOOK);
        assert_eq!(outcome, GuardOutcome::HookedByOther { at: 0, usable: false }, "shape len {}", len);
        assert_eq!(outcome.code(), "occupied");
        // The same change on a function we only call is fine.
        assert_eq!(guard_outcome(&stock(), &actual, SeamKind::Called), GuardOutcome::HookedByOther { at: 0, usable: true });
    }
    // A far jump that starts inside our span but runs past it still takes our words.
    let actual = with_words(stock(), 3, &FAR);
    assert_eq!(guard_outcome(&stock(), &actual, INLINE), GuardOutcome::HookedByOther { at: 12, usable: false });
}

#[test]
fn hooks_past_our_span_are_allowed_and_reported() {
    let actual = with_words(stock(), HOOK_SPAN_WORDS, &FAR);
    let outcome = guard_outcome(&stock(), &actual, HOOK);
    assert_eq!(outcome, GuardOutcome::HookedByOther { at: 4 * HOOK_SPAN_WORDS, usable: true });
    assert_eq!(outcome.code(), "other");
    // A far jump cut by the end of the window is still recognised.
    let actual = with_words(stock(), WORDS - 2, &FAR);
    assert_eq!(guard_outcome(&stock(), &actual, HOOK), GuardOutcome::HookedByOther { at: 4 * (WORDS - 2), usable: true });
    // Two hooks: the first decides `at`, any blocking one decides `usable`.
    let actual = with_words(with_words(stock(), 9, &[B]), 2, &[B]);
    assert_eq!(guard_outcome(&stock(), &actual, HOOK), GuardOutcome::HookedByOther { at: 8, usable: false });
}

#[test]
fn a_relied_on_word_must_stay_but_hooks_after_it_are_fine() {
    let at0 = with_words(stock(), 0, &[B]);
    assert_eq!(guard_outcome(&stock(), &at0, SeamKind::Relied), GuardOutcome::HookedByOther { at: 0, usable: false });
    let at1 = with_words(stock(), 1, &FAR);
    assert_eq!(guard_outcome(&stock(), &at1, SeamKind::Relied), GuardOutcome::HookedByOther { at: 4, usable: true });
}

#[test]
fn anything_that_is_not_a_hook_shape_is_foreign() {
    for kind in [HOOK, INLINE, SeamKind::Called, SeamKind::Relied] {
        let actual = with_words(stock(), 7, &[0xd280_0000]);
        assert_eq!(guard_outcome(&stock(), &actual, kind), GuardOutcome::Foreign { first_diff: 28 });
        // `ldr x17` without its `br x17` is not a far jump.
        let actual = with_words(stock(), 2, &[LDR_X17_LITERAL_8, 0xd503_201f]);
        assert_eq!(guard_outcome(&stock(), &actual, kind), GuardOutcome::Foreign { first_diff: 8 });
        // A change after a recognised hook is still foreign.
        let actual = with_words(with_words(stock(), 0, &[B]), 12, &[0]);
        assert_eq!(guard_outcome(&stock(), &actual, kind), GuardOutcome::Foreign { first_diff: 48 });
    }
    assert!(!GuardOutcome::Foreign { first_diff: 0 }.usable());
    assert_eq!(GuardOutcome::Foreign { first_diff: 0 }.code(), "foreign");
}

#[test]
fn scopes_are_a_bit_set() {
    let lobby = Scopes::TRAINING.with(Scopes::LOBBY_WAITING);
    assert!(lobby.contains(Scopes::TRAINING) && lobby.contains(Scopes::LOBBY_WAITING));
    assert!(!lobby.contains(Scopes::MATCH));
    assert_eq!(Scopes::TRAINING.with(Scopes::LOBBY_WAITING).with(Scopes::MATCH).with(Scopes::ONLINE), Scopes::ALL);
    assert!(Scopes::ALL.contains(Scopes::NONE));
}

#[test]
fn the_table_rows_are_well_formed() {
    let mut labels = BTreeSet::new();
    for seam in all() {
        assert!(labels.insert(seam.label), "duplicate label {}", seam.label);
        assert!(seam.label.contains('.'), "{}: label is <module>.<site>", seam.label);
        assert!(!seam.group.is_empty(), "{}: group", seam.label);
        if seam.compared() {
            assert!(seam.guard.iter().any(|b| *b != 0), "{}: guard is all zero", seam.label);
            assert_eq!(seam.offset % 4, 0, "{}: code offset is word aligned", seam.label);
        }
        if let SeamKind::Hook { placement: Placement::Entry, .. } = seam.kind {
            assert!(!seam.note.is_empty(), "{}: an entry hook needs its justification in `note`", seam.label);
        }
    }
    // Two hooks at one site would overwrite each other unless their features exclude each other.
    let hooks: Vec<_> = all().filter(|s| matches!(s.kind, SeamKind::Hook { .. })).collect();
    for (i, a) in hooks.iter().enumerate() {
        for b in &hooks[i + 1..] {
            assert!(
                a.offset != b.offset || a.feature != b.feature,
                "{} and {} hook {:#x} in the same build",
                a.label,
                b.label,
                a.offset
            );
        }
    }
    assert_eq!(count(), all().count());
    for (i, seam) in all().enumerate() {
        assert_eq!(find(seam.label).map(|(at, _)| at), Some(i));
    }
    assert!(find("no.such_label").is_none());
}

#[test]
fn the_card_scan_export_lists_each_offset_once() {
    let offsets: Vec<usize> = scan_offsets().collect();
    let unique: BTreeSet<usize> = offsets.iter().copied().collect();
    assert_eq!(offsets.len(), unique.len());
    assert_eq!(unique, all().map(|s| s.offset).collect::<BTreeSet<_>>());
    // `SEAM_OFFSETS_OUT=<file> cargo test … card_scan` writes them for `scan-offsets.py`.
    if let Ok(path) = std::env::var("SEAM_OFFSETS_OUT") {
        let text: String = offsets.iter().map(|o| format!("{o:#x}\n")).collect();
        std::fs::write(path, text).unwrap();
    }
}

/// One `#[skyline::hook(...)]` attribute in the root crate's `src/`.
#[derive(Debug)]
struct HookAttr {
    key: String,
    rule: Rule,
}

#[derive(Debug)]
enum Rule {
    Offset(usize),
    Needle(String),
    /// The row's label ends with `.<name>`; `*` matches any text (macro-generated names).
    Name(String),
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn hook_attributes() -> Vec<HookAttr> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    let mut files = Vec::new();
    // Only the carried runtime areas: the hooks of packages 1-5 are rows of src/model/sites.rs, not of this table.
    for area in ["swap", "mii", "preload", "kirby", "items", "stages", "lobby", "seams", "p1", "diag"] {
        let dir = root.join("src/runtime").join(area);
        if dir.is_dir() {
            rust_files(&dir, &mut files);
        }
    }
    let mut out = Vec::new();
    const OPEN: &str = "#[skyline::hook(";
    for file in files {
        let rel = file.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(&file).unwrap();
        let mut from = 0;
        while let Some(found) = text[from..].find(OPEN) {
            let args_start = from + found + OPEN.len();
            let mut depth = 1;
            let mut end = args_start;
            for (i, c) in text[args_start..].char_indices() {
                depth += (c == '(') as i32 - (c == ')') as i32;
                if depth == 0 {
                    end = args_start + i;
                    break;
                }
            }
            let args = &text[args_start..end];
            let after_fn = &text[end + text[end..].find("fn ").unwrap() + 3..];
            let name = if after_fn.starts_with("[<") {
                after_fn[..after_fn.find(">]").unwrap() + 2].to_string()
            } else {
                after_fn.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect()
            };
            let offset = args
                .split(',')
                .find_map(|part| part.trim().strip_prefix("offset"))
                .and_then(|rest| rest.trim_start().strip_prefix('='))
                .map(str::trim);
            let rule = match offset {
                Some(v) if v.starts_with("0x") => Rule::Offset(usize::from_str_radix(&v[2..].replace('_', ""), 16).unwrap()),
                Some(v) if v.starts_with("SITE_") => Rule::Needle(format!("needle.{}", v[5..].to_lowercase())),
                _ => Rule::Name(
                    name.trim_start_matches("[<")
                        .trim_end_matches(">]")
                        .split_whitespace()
                        .map(|t| if t.starts_with('$') { "*" } else { t })
                        .collect(),
                ),
            };
            out.push(HookAttr { key: format!("{rel}:{name}"), rule });
            from = end;
        }
    }
    out
}

/// `pattern` with `*` wildcards matches all of `text`.
fn glob(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !text.starts_with(first) || !text.ends_with(last) || text.len() < first.len() + last.len() {
        return false;
    }
    let mut rest = &text[first.len()..text.len() - last.len()];
    for part in &parts[1..parts.len() - 1] {
        match rest.find(part) {
            Some(i) => rest = &rest[i + part.len()..],
            None => return false,
        }
    }
    true
}

fn has_row(rule: &Rule) -> bool {
    let by_name = |label: &str, pattern: &str| label.rsplit_once('.').is_some_and(|(_, site)| glob(pattern, site));
    all().any(|seam| match rule {
        Rule::Offset(offset) => seam.offset == *offset,
        Rule::Needle(label) => seam.label == label,
        Rule::Name(pattern) => by_name(seam.label, pattern),
    })
    // A hook outside `main` (rows 27/36) has an image row, found by name like a symbol hook.
    || matches!(rule, Rule::Name(pattern) if images::IMAGE_ROWS.iter().any(|seam| by_name(seam.label, pattern)))
}

#[test]
fn glob_matches_macro_generated_names() {
    assert!(glob("handle_*daikon_*_prob", "handle_daisydaikon_1_prob"));
    assert!(!glob("handle_*daikon_*_prob", "handle_daisydaikon_1_probe"));
    assert!(glob("prepare_models", "prepare_models"));
    assert!(!glob("prepare_models", "prepare_model"));
}

#[test]
fn every_skyline_hook_attribute_has_a_row_or_is_listed_unconverted() {
    let attrs = hook_attributes();
    // The fork counted 150+ hooks over its whole src/. Here only the carried runtime areas are scanned and they
    // land in steps C7-C10 (docs/PACKAGE-6-PLAN.md); the ratchet is active as soon as one carried hook exists.
    if attrs.is_empty() {
        return;
    }
    let listed: BTreeSet<&str> = UNCONVERTED.iter().flat_map(|l| l.iter().copied()).collect();
    let keys: BTreeSet<&str> = attrs.iter().map(|a| a.key.as_str()).collect();
    let mut problems = Vec::new();
    for attr in &attrs {
        match (has_row(&attr.rule), listed.contains(attr.key.as_str())) {
            (true, true) => problems.push(format!("{} has a row now: delete it from UNCONVERTED", attr.key)),
            (false, false) => problems.push(format!("{} has no row ({:?}) and is not in UNCONVERTED", attr.key, attr.rule)),
            _ => {}
        }
    }
    for entry in &listed {
        if !keys.contains(entry) {
            problems.push(format!("UNCONVERTED lists {entry}, which is no hook attribute"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// CPU as P1 (cpu-p1): ssbu-cpu-pilot-core 0162eed leaves the version and 64-byte guards to its host
/// (`core/src/native.rs`), so `cpu_pilot` guards its one hook and the three game functions the library's
/// `native` calls (`offsets::AI_THINK`, `AI_SET_RANK`, `AI_RESET`). The legacy body checks in
/// `cpu_pilot::init` (still deciding while the group is shadow) are sub-windows of these guards.
#[test]
fn the_cpu_pilot_rows_guard_its_hook_and_every_function_the_library_calls() {
    let row = |label: &str| find(label).map(|(_, s)| s).unwrap_or_else(|| panic!("no row {}", label));
    let rows: Vec<_> = all().filter(|s| s.group == "cpu_pilot").collect();
    assert_eq!(rows.len(), 4, "cpu_pilot rows: {:?}", rows.iter().map(|s| s.label).collect::<Vec<_>>());
    for s in &rows {
        assert_eq!(s.feature, "cpu-pilot-probe", "{}", s.label);
        let sharing = if s.label == "cpu_pilot.ai_think" {
            Sharing::OwnHook { hook: "needle.set_cpu_controls" }
        } else {
            Sharing::None
        };
        assert_eq!(s.sharing, sharing, "{}: guarded, not external", s.label);
        assert!(s.compared(), "{}", s.label);
    }
    let hook = row("cpu_pilot.ai_process_begin");
    assert_eq!((hook.offset, hook.kind), (0x30c710, HOOK));
    for (label, offset) in [("cpu_pilot.ai_think", 0x2da180), ("cpu_pilot.ai_set_rank", 0x6b8400), ("cpu_pilot.ai_reset", 0x2d8d40)] {
        let called = row(label);
        assert_eq!((called.offset, called.kind), (offset, SeamKind::Called), "{label}");
    }
    // The legacy checks: 8 bytes at set_rank+0x20 and think+0x20, 16 bytes at process_begin+0x30.
    assert_eq!(row("cpu_pilot.ai_set_rank").guard[0x20..0x28], [0xa8, 0x03, 0x00, 0xb4, 0x95, 0x2e, 0x40, 0xf9]);
    assert_eq!(row("cpu_pilot.ai_think").guard[0x20..0x28], [0xf6, 0x57, 0x15, 0xa9, 0xf4, 0x4f, 0x16, 0xa9]);
    assert_eq!(
        hook.guard[0x30..0x40],
        [0x08, 0x24, 0x40, 0x39, 0x68, 0x94, 0x02, 0x34, 0x08, 0x00, 0x40, 0xf9, 0x09, 0xfd, 0x41, 0xb9]
    );
    // AI think is also the Training Modpack's own set_cpu_controls hook site: one hook row, one called row.
    assert_eq!(row("needle.set_cpu_controls").guard, row("cpu_pilot.ai_think").guard);
    // The modpack's own replace hook on think is a far jump at the entry. With the replacement address
    // the installer registered for needle.set_cpu_controls it is our own write: stock (M4d, R-CP0).
    let think = find_site("cpu_pilot.ai_think").unwrap().1;
    let ours = with_words(think.guard, 0, &FAR);
    assert_eq!(think.outcome_with(&ours, Some(FAR_TARGET)), GuardOutcome::Stock);
    // Unregistered (hook not installed), or a far jump to any other address: today's `other`, still usable.
    assert_eq!(think.outcome_with(&ours, None), GuardOutcome::HookedByOther { at: 0, usable: true });
    assert_eq!(think.outcome_with(&ours, Some(FAR_TARGET + 0x40)), GuardOutcome::HookedByOther { at: 0, usable: true });
    assert_eq!(row("cpu_pilot.ai_think").outcome(&ours), GuardOutcome::HookedByOther { at: 0, usable: true });
    // No row is external any more.
    assert!(all().all(|s| !matches!(s.sharing, Sharing::External { .. })));
}

/// `Sharing::OwnHook` (M4d): only a far jump at the entry whose `.quad` is the registered replacement,
/// with the other 64 bytes stock, reads `Stock`; every other shape keeps `outcome_for`'s answer.
#[test]
fn an_own_hook_row_reads_stock_only_for_our_registered_far_jump() {
    let own = Sharing::OwnHook { hook: "needle.x" };
    let rule = |actual: &[u8; GUARD_BYTES], target: Option<u64>| outcome_for_own(&stock(), actual, SeamKind::Called, own, target);
    for far in [&FAR[..], &NOP_FAR[..]] {
        let ours = with_words(stock(), 0, far);
        assert_eq!(entry_far_jump_target(&ours), Some(FAR_TARGET));
        assert_eq!(rule(&ours, Some(FAR_TARGET)), GuardOutcome::Stock);
        // Another plugin's far jump (another target), or no registration: `other` as before.
        assert_eq!(rule(&ours, Some(FAR_TARGET ^ 0x10)), GuardOutcome::HookedByOther { at: 0, usable: true });
        assert_eq!(rule(&ours, None), GuardOutcome::HookedByOther { at: 0, usable: true });
        // Our far jump plus another hook further in: there is another plugin there, so not stock.
        let crowded = with_words(ours, 8, &[B]);
        assert_eq!(rule(&crowded, Some(FAR_TARGET)), outcome_for(&stock(), &crowded, SeamKind::Called, own));
        assert_ne!(rule(&crowded, Some(FAR_TARGET)), GuardOutcome::Stock);
        // Our far jump plus a non-hook change stays foreign.
        let changed = with_words(ours, 9, &[0xd280_0000]);
        assert_eq!(rule(&changed, Some(FAR_TARGET)), GuardOutcome::Foreign { first_diff: 36 });
    }
    // A one-word branch at the entry has no target to compare.
    let branch = with_words(stock(), 0, &[B]);
    assert_eq!(entry_far_jump_target(&branch), None);
    assert_eq!(rule(&branch, Some(FAR_TARGET)), outcome_for(&stock(), &branch, SeamKind::Called, own));
    assert_eq!(rule(&stock(), Some(FAR_TARGET)), GuardOutcome::Stock);
    // Other sharing rules ignore the target.
    let far = with_words(stock(), 0, &FAR);
    assert_eq!(
        outcome_for_own(&stock(), &far, SeamKind::Called, Sharing::None, Some(FAR_TARGET)),
        GuardOutcome::HookedByOther { at: 0, usable: true }
    );
}

/// Every `Sharing::OwnHook` row names a replace/entry hook row of ours at the same address with the
/// same 64 bytes, and none is an image row (the installer registers main replace hooks only).
#[test]
fn own_hook_rows_name_our_replace_hook_at_the_same_address() {
    let own: Vec<_> = all().filter(|s| matches!(s.sharing, Sharing::OwnHook { .. })).collect();
    assert_eq!(own.len(), OWN_HOOK_ROWS);
    assert_eq!(own.iter().map(|s| s.label).collect::<Vec<_>>(), ["cpu_pilot.ai_think"]);
    assert_eq!(OWN_HOOK_PAIRS, [("cpu_pilot.ai_think", "needle.set_cpu_controls")]);
    // The run-time view adds no per-row label: the flag sits in `Site`'s padding (resident size, U17).
    assert_eq!(std::mem::size_of::<Site>(), 112);
    for s in own {
        let Sharing::OwnHook { hook } = s.sharing else { unreachable!() };
        let (_, h) = find(hook).unwrap_or_else(|| panic!("{}: no hook row {}", s.label, hook));
        assert_eq!(h.kind, HOOK, "{}: {} is a replace hook at the entry", s.label, hook);
        assert_eq!((h.offset, h.guard), (s.offset, s.guard), "{}: same address and bytes as {}", s.label, hook);
    }
    assert!(images::IMAGE_ROWS.iter().all(|s| !matches!(s.sharing, Sharing::OwnHook { .. })));
}

#[test]
fn the_outcome_tables_are_sized_from_the_row_files() {
    assert_eq!(COUNT, count(), "COUNT must count every row (the installer sizes its table with it)");
    assert_eq!(COUNT, all().count());
    assert_eq!(IMAGE_COUNT, images::IMAGE_ROWS.len());
}

#[test]
fn a_chain_row_accepts_a_far_jump_at_its_entry_and_nothing_else() {
    let chain = |actual: &[u8; GUARD_BYTES]| outcome_for(&stock(), actual, HOOK, Sharing::Chain);
    for far in [&FAR[..], &NOP_FAR[..]] {
        let actual = with_words(stock(), 0, far);
        assert_eq!(chain(&actual), GuardOutcome::HookedByOther { at: 0, usable: true });
        assert!(far_jump_at(&actual, 0));
        // Without the chain rule the same bytes occupy the entry.
        assert_eq!(outcome_for(&stock(), &actual, HOOK, Sharing::None), GuardOutcome::HookedByOther { at: 0, usable: false });
    }
    // A one-word branch at the entry is not chainable.
    assert_eq!(chain(&with_words(stock(), 0, &[B])), GuardOutcome::HookedByOther { at: 0, usable: false });
    // A far jump plus another hook inside our span after it is not chainable either.
    let crowded = with_words(with_words(stock(), 0, &FAR), 4, &[B]);
    assert!(!chain(&crowded).usable());
    // A non-hook change stays foreign.
    let foreign = with_words(with_words(stock(), 0, &FAR), 9, &[0xd280_0000]);
    assert_eq!(chain(&foreign), GuardOutcome::Foreign { first_diff: 36 });
    assert_eq!(chain(&stock()), GuardOutcome::Stock);
}

/// `Sharing::EntryOrBody` (SEAMS-ALL): the entry is ours when stock; another plugin's far jump over the
/// entry that ends before the body, with every other word stock, is `Shared` (we take the body); a
/// one-word branch, a far jump reaching the body, or any further change stays unusable.
#[test]
fn an_entry_or_body_row_is_stock_or_shared_and_nothing_else() {
    let rule = |actual: &[u8; GUARD_BYTES]| entry_or_body_outcome(&stock(), actual, HOOK, 4);
    assert_eq!(rule(&stock()), GuardOutcome::Stock);
    let far = with_words(stock(), 0, &FAR);
    assert_eq!(rule(&far), GuardOutcome::Shared);
    assert!(GuardOutcome::Shared.usable());
    assert_eq!(GuardOutcome::Shared.code(), "shared");
    // The same far jump with its `nop` covers the body's first word: not shared.
    assert_eq!(rule(&with_words(stock(), 0, &NOP_FAR)), GuardOutcome::HookedByOther { at: 0, usable: false });
    // A body further on leaves room for the `nop` form.
    assert_eq!(entry_or_body_outcome(&stock(), &with_words(stock(), 0, &NOP_FAR), HOOK, 5), GuardOutcome::Shared);
    // A one-word branch at the entry.
    assert_eq!(rule(&with_words(stock(), 0, &[B])), GuardOutcome::HookedByOther { at: 0, usable: false });
    // The far jump plus a hook in the body, or plus a non-hook change.
    assert!(!rule(&with_words(far, 4, &[B])).usable());
    assert!(!rule(&with_words(far, 12, &[B])).usable());
    assert_eq!(rule(&with_words(far, 9, &[0xd280_0000])), GuardOutcome::Foreign { first_diff: 36 });
    // Without the rule the far jump occupies the entry hook (the M5h projection, `Sharing::None`).
    assert_eq!(outcome_for(&stock(), &far, HOOK, Sharing::None), GuardOutcome::HookedByOther { at: 0, usable: false });
}

/// Every `Sharing::EntryOrBody` row's body is a row of ours inside the entry's 64 bytes, word-aligned
/// and past a 4-word far jump, and the run-time view applies the rule to the real row.
#[test]
fn entry_or_body_rows_name_a_body_row_inside_their_window() {
    assert_eq!(ENTRY_OR_BODY_COUNT, 1);
    assert_eq!(ENTRY_OR_BODY_ROWS, [("replay.write_record", "smash_replay_to_mp4", 0x143e2b0)]);
    for (label, _, body) in ENTRY_OR_BODY_ROWS {
        let (_, entry) = find_site(label).unwrap();
        assert!(body > entry.offset && body - entry.offset < GUARD_BYTES && (body - entry.offset).is_multiple_of(4), "{}", label);
        assert!((body - entry.offset) / 4 >= 4, "{}: a 4-word far jump ends before the body", label);
        assert!(all().any(|s| s.offset == body), "{}: a row guards the body at {:#x}", label, body);
        let mut far = entry.guard;
        far[..16].copy_from_slice(&with_words(stock(), 0, &FAR)[..16]);
        assert_eq!(entry.outcome(&far), GuardOutcome::Shared, "{}", label);
        assert_eq!(entry.outcome(&entry.guard), GuardOutcome::Stock, "{}", label);
    }
}

/// The diagnostic hook-sharing drill (`src/swap_native/seam_drill.rs`, `foreign=` in `diag.txt`): each
/// stand-in form on the one site per sharing rule it hooks reads as the drill expects. `inline` is a
/// one-word branch, `far` a 4-word far jump to the stand-in handler.
#[test]
fn the_drill_sites_classify_each_standin_form() {
    let at = |label: &str, form: &[u32]| {
        let (_, site) = find_site(label).unwrap_or_else(|| panic!("no row {}", label));
        let mut actual = site.guard;
        let words: Vec<u8> = form.iter().flat_map(|w| w.to_le_bytes()).collect();
        actual[..words.len()].copy_from_slice(&words);
        (site, actual)
    };
    let occupied = GuardOutcome::HookedByOther { at: 0, usable: false };
    let other = GuardOutcome::HookedByOther { at: 0, usable: true };
    // label, inline (one-word branch), far (far jump)
    let expected: [(&str, GuardOutcome, GuardOutcome); 5] = [
        // Sharing::Chain replace row: a far jump is chained, a branch skips the site.
        ("training.handle_effect", occupied, other),
        // A function we only call accepts either.
        ("game_mode.popup_idle", other, other),
        // Sharing::EntryOrBody: a branch leaves the adapter off, a far jump moves it to the body.
        ("replay.write_record", occupied, GuardOutcome::Shared),
        // OwnHook pair: our replace hook's row (no sharing rule) skips under either stand-in …
        ("needle.set_cpu_controls", occupied, occupied),
        // … and the Called row on the same address tolerates either.
        ("cpu_pilot.ai_think", other, other),
    ];
    for (label, inline, far) in expected {
        let (site, branch) = at(label, &[B]);
        assert_eq!(site.outcome(&branch), inline, "{} inline", label);
        assert_eq!(entry_form(&site.guard, &branch), EntryForm::Branch, "{}", label);
        let (site, jump) = at(label, &FAR);
        assert_eq!(site.outcome(&jump), far, "{} far", label);
        assert_eq!(entry_form(&site.guard, &jump), EntryForm::FarJump, "{}", label);
        assert_eq!(entry_form(&site.guard, &site.guard), EntryForm::Stock, "{}", label);
    }
    // The body row of the pair keeps its own 64 bytes under an entry stand-in of either form.
    let (body, _) = at("replay.write_record_shared", &[]);
    assert_eq!(body.outcome(&body.guard), GuardOutcome::Stock);
    // OwnHook: our own far jump (the registered target) on the pair's Called row reads stock; the
    // stand-in's far jump (another target) reads `other`.
    let (ai_think, ours) = at("cpu_pilot.ai_think", &FAR);
    assert_eq!(ai_think.outcome_with(&ours, Some(FAR_TARGET)), GuardOutcome::Stock);
    assert_eq!(ai_think.outcome_with(&ours, Some(FAR_TARGET + 0x40)), other);
    // A change that is not a hook at all.
    let (popup, mut changed) = at("game_mode.popup_idle", &[]);
    changed[0] ^= 1;
    assert_eq!(entry_form(&popup.guard, &changed), EntryForm::Changed);
    assert_eq!(EntryForm::Branch.name(), "branch");
    assert_eq!(EntryForm::FarJump.name(), "far");
}

#[test]
fn the_image_rows_are_well_formed() {
    let mut labels: BTreeSet<&str> = all().map(|s| s.label).collect();
    for seam in images::IMAGE_ROWS {
        assert!(labels.insert(seam.label), "duplicate label {}", seam.label);
        assert!(seam.image != Image::Main, "{}: main rows belong in rows_aN.rs", seam.label);
        match seam.locator {
            Locator::Offset(offset) => assert_eq!(offset % 4, 0, "{}: aligned", seam.label),
            Locator::Symbol(name) => assert!(!name.is_empty() && name.len() < 256, "{}: symbol", seam.label),
        }
        if let Some(guard) = seam.guard {
            assert!(guard.iter().any(|b| *b != 0), "{}: guard is all zero", seam.label);
        }
        if let SeamKind::Hook { placement: Placement::Entry, .. } = seam.kind {
            assert!(!seam.note.is_empty(), "{}: an entry hook needs its justification", seam.label);
        }
    }
}

#[test]
fn every_compared_row_lies_in_one_read_only_region_and_rodata_rows_are_relied() {
    let mut problems = Vec::new();
    for seam in all().filter(|s| s.compared()) {
        match seam.region() {
            None => problems.push(format!("{} {:#x}: its 64 bytes are not inside main .text or .rodata", seam.label, seam.offset)),
            Some(MainRegion::Rodata) if seam.kind != SeamKind::Relied => {
                problems.push(format!("{} {:#x}: a .rodata row can only be Relied", seam.label, seam.offset))
            }
            _ => {}
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn rodata_rows_compare_exactly_and_code_rows_by_their_kind() {
    assert_eq!(region_of(0), Some(MainRegion::Text));
    assert_eq!(region_of(MAIN_TEXT_END - GUARD_BYTES), Some(MainRegion::Text));
    assert_eq!(region_of(MAIN_TEXT_END - GUARD_BYTES + 4), None);
    assert_eq!(region_of(MAIN_RODATA_START), Some(MainRegion::Rodata));
    assert_eq!(region_of(MAIN_RODATA_END - GUARD_BYTES), Some(MainRegion::Rodata));
    assert_eq!(region_of(MAIN_RODATA_END - GUARD_BYTES + 4), None);
    assert_eq!(region_of(usize::MAX - 8), None);
    let table = Seam {
        label: "test.relied_table",
        group: "test",
        offset: 0x44b_e0d4,
        kind: SeamKind::Relied,
        guard: stock(),
        feature: "",
        scopes: Scopes::ALL,
        read_only: true,
        per_frame: false,
        sharing: Sharing::None,
        note: "",
    };
    assert_eq!(table.outcome(&stock()), GuardOutcome::Stock);
    // A hook shape after word 0 passes the Relied code rule, but read-only data is never hooked.
    let changed = with_words(stock(), 3, &[B]);
    assert_eq!(table.outcome(&changed), GuardOutcome::Foreign { first_diff: 12 });
    let code = Seam { offset: 0x100, ..table };
    assert_eq!(code.outcome(&changed), GuardOutcome::HookedByOther { at: 12, usable: true });
    assert_eq!(exact_outcome(&stock(), &stock()), GuardOutcome::Stock);
}

/// The owned 13.0.5 ELF, linked next to every refactor stage (`<stage>/../research`). A missing ELF
/// fails the caller unless `SKIP_OWNED_ELF_FACT=1`, which prints a visible SKIPPED line (row 32).
fn owned_elf(test: &str) -> Option<Vec<u8>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../research/game-13.0.5/main-13.0.5.elf");
    match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(_) => {
            assert!(
                std::env::var_os("SKIP_OWNED_ELF_FACT").is_some(),
                "owned 13.0.5 ELF missing at {}: link <stage>/../research, or set SKIP_OWNED_ELF_FACT=1 to skip",
                path
            );
            println!("SKIPPED {}: {} absent", test, path);
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Section {
    addr: usize,
    offset: usize,
    size: usize,
}

/// ELF64 little-endian section headers by name.
fn sections(elf: &[u8]) -> Vec<(String, Section)> {
    let u16_at = |o: usize| u16::from_le_bytes([elf[o], elf[o + 1]]) as usize;
    let u32_at = |o: usize| u32::from_le_bytes([elf[o], elf[o + 1], elf[o + 2], elf[o + 3]]) as usize;
    let u64_at = |o: usize| {
        let mut word = [0u8; 8];
        word.copy_from_slice(&elf[o..o + 8]);
        u64::from_le_bytes(word) as usize
    };
    assert_eq!(&elf[..4], b"\x7fELF");
    let (shoff, shentsize, shnum, shstrndx) = (u64_at(0x28), u16_at(0x3a), u16_at(0x3c), u16_at(0x3e));
    let header = |i: usize| shoff + i * shentsize;
    let strtab = u64_at(header(shstrndx) + 0x18);
    (0..shnum)
        .map(|i| {
            let h = header(i);
            let name_at = strtab + u32_at(h);
            let len = elf[name_at..].iter().position(|b| *b == 0).unwrap();
            let name = String::from_utf8_lossy(&elf[name_at..name_at + len]).into_owned();
            (name, Section { addr: u64_at(h + 0x10), offset: u64_at(h + 0x18), size: u64_at(h + 0x20) })
        })
        .collect()
}

#[test]
fn compared_rows_match_the_owned_elf() {
    let Some(elf) = owned_elf("compared_rows_match_the_owned_elf") else {
        return;
    };
    let sections = sections(&elf);
    let find = |name: &str| {
        sections.iter().find(|(n, _)| n == name).map(|(_, s)| *s).unwrap_or_else(|| panic!("no {} section", name))
    };
    let (text, rodata) = (find(".text"), find(".rodata"));
    assert_eq!((text.addr, text.addr + text.size), (0, MAIN_TEXT_END), ".text bounds");
    assert_eq!((rodata.addr, rodata.addr + rodata.size), (MAIN_RODATA_START, MAIN_RODATA_END), ".rodata bounds");
    let mut problems = Vec::new();
    for seam in all().filter(|s| s.compared()) {
        let section = match seam.region() {
            Some(MainRegion::Text) => text,
            Some(MainRegion::Rodata) => rodata,
            None => {
                problems.push(format!("{} {:#x}: outside .text and .rodata", seam.label, seam.offset));
                continue;
            }
        };
        let at = section.offset + (seam.offset - section.addr);
        if elf[at..at + GUARD_BYTES] != seam.guard[..] {
            problems.push(format!("{} {:#x}: guard differs from the owned ELF", seam.label, seam.offset));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_runtime_sites_are_the_authored_rows_without_their_text() {
    // U17: the NRO reads only `SITES` / `IMAGE_SITES`, the rows' guard data and labels in table order;
    // the review text (`note`, `feature`, scopes, flags, sharing reasons) stays in the authored rows.
    assert_eq!(SITES.len(), count());
    assert_eq!(COUNT, SITES.len());
    for (i, (site, seam)) in SITES.iter().zip(all()).enumerate() {
        assert_eq!((site.label, site.group, site.offset), (seam.label, seam.group, seam.offset), "row {}", i);
        assert_eq!(site.guard, seam.guard, "{}: guard", seam.label);
        assert_eq!(site.kind, seam.kind, "{}: kind", seam.label);
        assert_eq!(site.chain, seam.sharing == Sharing::Chain, "{}: sharing", seam.label);
        let own = match seam.sharing {
            Sharing::OwnHook { hook } => Some(hook),
            _ => None,
        };
        assert_eq!(site.own_hook(), own, "{}: own hook", seam.label);
        assert_eq!(site.own_hook_row, own.is_some(), "{}: own hook flag", seam.label);
        let entry_or_body = match seam.sharing {
            Sharing::EntryOrBody { owner, body } => Some((owner, body)),
            _ => None,
        };
        assert_eq!(site.entry_or_body(), entry_or_body, "{}: entry or body", seam.label);
        assert_eq!(site.entry_or_body_row, entry_or_body.is_some(), "{}: entry or body flag", seam.label);
        if matches!(seam.sharing, Sharing::None | Sharing::Chain | Sharing::OwnHook { .. } | Sharing::EntryOrBody { .. }) {
            assert_eq!(site.sharing(), seam.sharing, "{}: the rule the installer applies", seam.label);
        }
        assert_eq!(site.compared(), seam.compared(), "{}: compared", seam.label);
        assert_eq!(site.region(), seam.region(), "{}: region", seam.label);
        assert_eq!(find_site(seam.label).map(|(at, _)| at), find(seam.label).map(|(at, _)| at), "{}: index", seam.label);
    }
    assert_eq!(IMAGE_SITES.len(), images::IMAGE_ROWS.len());
    for (index, (site, seam)) in IMAGE_SITES.iter().zip(images::IMAGE_ROWS).enumerate() {
        assert_eq!((site.label, site.group, site.image, site.locator), (seam.label, seam.group, seam.image, seam.locator));
        assert_eq!(site.guard, seam.guard, "{}: guard", seam.label);
        assert_eq!(site.kind, seam.kind, "{}: kind", seam.label);
        assert_eq!(site.chain, seam.sharing == Sharing::Chain, "{}: sharing", seam.label);
        assert_eq!(find_image(seam.label).map(|(at, _)| at), Some(index), "{}: index", seam.label);
    }
}
