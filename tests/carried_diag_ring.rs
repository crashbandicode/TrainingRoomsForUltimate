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

use diag_ring::{ByteRing, Push, StackLine, CUT_MARKER, STACK_LINE};

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

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_full_ring_drops_and_counts_and_never_blocks", a_full_ring_drops_and_counts_and_never_blocks as fn()),
        ("long_lines_are_cut_at_the_per_line_limit_with_a_marker", long_lines_are_cut_at_the_per_line_limit_with_a_marker as fn()),
    ]);
}
