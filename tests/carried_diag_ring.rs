//! Host tests of the diagnostics channel core `src/model/diag_ring.rs` (WP-D), included by path:
//! wrap, drop counter, truncation, whole-line drain, and the shipped/diag line classification.
//! Carried over from the earlier fork's `training_mod_consts/tests/diag_ring.rs` (stage m5o,
//! ours-new); see CARRY-OVER.md.

// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model/diag_ring.rs"]
#[allow(dead_code)]
mod diag_ring;

use diag_ring::{shipped, tag_of, ByteRing, Push, StackLine, CUT_MARKER, SHIPPED_TAGS, STACK_LINE};

fn push<const N: usize>(ring: &mut ByteRing<N>, ts: u64, text: &str, max_line: usize) -> Push {
    let mut line = StackLine::new();
    let whole = line.format(format_args!("{text}"));
    ring.push(ts, whole.then_some(line.text()), format_args!("{text}"), max_line)
}

fn drain_all<const N: usize>(ring: &mut ByteRing<N>, chunk: usize) -> String {
    let mut out = Vec::new();
    let mut buf = vec![0u8; chunk];
    loop {
        let n = ring.drain_into(&mut buf);
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
    }
    String::from_utf8(out).unwrap()
}

fn lines_get_the_timestamp_and_sequence_prefix_and_drain_whole() {
    let mut ring = ByteRing::<1024>::new();
    assert_eq!(push(&mut ring, 1700000000123, "[GroupSwap] swap owner=0", 512), Push::Committed);
    assert_eq!(push(&mut ring, 1700000000124, "[BodyBank] build", 512), Push::Committed);
    assert_eq!(
        drain_all(&mut ring, 1024),
        "1700000000123 #1 [GroupSwap] swap owner=0\n1700000000124 #2 [BodyBank] build\n"
    );
    assert_eq!(ring.buffered(), 0);
    let stats = ring.stats();
    assert_eq!((stats.committed, stats.truncated, stats.dropped), (2, 0, 0));
}

fn the_ring_wraps_without_losing_or_reordering_lines() {
    let mut ring = ByteRing::<256>::new();
    let mut expected = String::new();
    let mut got = String::new();
    for i in 0..200u64 {
        let text = format!("[IcPair] line {i} {}", "x".repeat((i % 17) as usize));
        assert_eq!(push(&mut ring, 10 + i, &text, 200), Push::Committed, "line {}", i);
        expected.push_str(&format!("{} #{} {}\n", 10 + i, i + 1, text));
        // Drain in small whole-line chunks every few lines, so head and tail wrap many times.
        if i % 3 == 2 {
            got.push_str(&drain_all(&mut ring, 96));
        }
    }
    got.push_str(&drain_all(&mut ring, 96));
    assert_eq!(got, expected);
}

fn a_full_ring_drops_and_counts_and_never_blocks() {
    let mut ring = ByteRing::<256>::new();
    let mut dropped = 0;
    for i in 0..20 {
        if push(&mut ring, i, "[GroupSwap] a line that takes some room", 200) == Push::Dropped {
            dropped += 1;
        }
    }
    assert!(dropped > 0);
    assert_eq!(ring.stats().dropped, dropped);
    assert!(ring.buffered() <= 256);
    // Every committed line is whole and newline-terminated, and drains.
    let text = drain_all(&mut ring, 256);
    assert!(text.lines().all(|l| l.ends_with("some room")), "{}", text);
    ring.note_dropped(3);
    assert_eq!(ring.stats().dropped, dropped + 3);
}

fn long_lines_are_cut_at_the_per_line_limit_with_a_marker() {
    let mut ring = ByteRing::<4096>::new();
    let long = format!("[MiiHelper] {}", "y".repeat(3 * STACK_LINE));
    // Longer than the stack buffer: formatted straight into the ring, then cut at max_line.
    assert_eq!(push(&mut ring, 7, &long, 300), Push::Truncated);
    let text = drain_all(&mut ring, 4096);
    assert_eq!(text.len(), 300);
    assert!(text.ends_with(&format!("{}\n", std::str::from_utf8(CUT_MARKER).unwrap())), "{}", text);
    assert!(text.starts_with("7 #1 [MiiHelper] yyy"));
    // A long line under the limit is kept whole even though it overflowed the stack buffer.
    assert_eq!(push(&mut ring, 8, &long, 4096), Push::Committed);
    assert_eq!(drain_all(&mut ring, 4096), format!("8 #2 {long}\n"));
    assert_eq!(ring.stats().truncated, 1);
}

fn the_stack_line_stops_formatting_when_full() {
    let mut line = StackLine::new();
    assert!(line.format(format_args!("[TrainingModpack] result={}", "installed")));
    assert_eq!(line.text(), b"[TrainingModpack] result=installed");
    let big = "z".repeat(STACK_LINE + 10);
    assert!(!line.format(format_args!("[Tag] {big}")));
    assert_eq!(line.text().len(), STACK_LINE);
    assert!(line.text().starts_with(b"[Tag] zz"));
}

fn shipped_builds_keep_only_the_breadcrumb_tags() {
    let mut sorted = SHIPPED_TAGS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted, SHIPPED_TAGS, "sorted and unique");
    assert!(SHIPPED_TAGS.iter().all(|t| t.starts_with('[') && t.ends_with(']')));
    for kept in ["[TrainingModpack] result=installed", "[TrainingSeams] late group=mii_ordinary_gate sites=19", "[PracticeDiagnostics] rust_panic x", "[GroupSwap] teardown", "[LobbyPractice] scene", "[PreloadMetrics] v1 snapshot"] {
        assert!(shipped(kept.as_bytes()), "{}", kept);
    }
    // The refactor's new diagnostic lines, probes, stand-ins and metrics stay diag-only.
    for dropped in [
        "[Ledger] reconcile owner=0",
        "[Destroy] plan owner=0",
        "[Hud] readback ok=true",
        "[Scope] refused name=x",
        "[Heartbeat] n=1",
        "[PluginHeap] phase=boot",
        "[SourceOwnerProbe] x",
        "untagged line",
        "",
    ] {
        assert!(!shipped(dropped.as_bytes()), "{}", dropped);
    }
    assert_eq!(tag_of(b"[IcPair] x"), Some(&b"[IcPair]"[..]));
    assert_eq!(tag_of(b" [IcPair] x"), None);
    assert_eq!(tag_of(b"[unterminated"), None);
}

/// LOG-REFUSED (Surface R-GSP7: the release NRO printed no refusal): a memory refusal and the group fallback ship;
/// the per-load measurement lines stay in diag builds (`[CharacterSources]` is not shipped; the bank's
/// `[GroupSwap] memory` line is suppressed in its writer outside `diag`).
fn memory_refusals_ship_and_measurements_stay_diag() {
    for kept in [
        "[ProcessMemory] refused consumer=helper slot=2 kind=0x52 what=preload admission=Refuse; stays planned",
        "[GroupSwap] fallback owner=0 group=1 reason=process headroom Some(107237376) < 161505280 bytes",
    ] {
        assert!(shipped(kept.as_bytes()), "{}", kept);
    }
    assert!(!shipped(b"[CharacterSources] memory slot=2 kinds=[52] phase=helper delta=1"));
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("lines_get_the_timestamp_and_sequence_prefix_and_drain_whole", lines_get_the_timestamp_and_sequence_prefix_and_drain_whole as fn()),
        ("the_ring_wraps_without_losing_or_reordering_lines", the_ring_wraps_without_losing_or_reordering_lines as fn()),
        ("a_full_ring_drops_and_counts_and_never_blocks", a_full_ring_drops_and_counts_and_never_blocks as fn()),
        ("long_lines_are_cut_at_the_per_line_limit_with_a_marker", long_lines_are_cut_at_the_per_line_limit_with_a_marker as fn()),
        ("the_stack_line_stops_formatting_when_full", the_stack_line_stops_formatting_when_full as fn()),
        ("shipped_builds_keep_only_the_breadcrumb_tags", shipped_builds_keep_only_the_breadcrumb_tags as fn()),
        ("memory_refusals_ship_and_measurements_stay_diag", memory_refusals_ship_and_measurements_stay_diag as fn()),
    ]);
}
