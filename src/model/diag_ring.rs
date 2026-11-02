//! Diagnostics channel core (WP-D). Pure: no I/O, no allocation, no locking policy; the runtime's
//! `diag_log` wraps one `ByteRing` in a spin lock and a drain thread writes it to the diagnostics
//! file. Host-tested by `tests/carried_diag_ring.rs`.
//!
//! Carried over from the earlier fork's `src/diag_ring.rs` (stage m5o, ours-new); CARRY-OVER.md lists
//! the edits (this doc paragraph and the added `[TrainingRooms]` tag).
//!
//! Contract: a caller formats its line on its own stack frame (`StackLine`), so a line that is not
//! admitted (shipped builds keep only `SHIPPED_TAGS`) costs no lock and no ring space, and a short
//! line is copied into the ring in one short critical section. Only a line longer than the stack
//! buffer is formatted again directly into the ring under the lock. A full ring drops the line and
//! counts it; it never blocks and never grows. Every committed line is
//! `"<unix ms> #<seq> <text>\n"`, the format the Eden tools and `swap_trace.py` already parse.

use core::fmt::{self, Write};

/// Bytes of a line kept for the admission check and the short-line copy (on the caller's stack).
pub const STACK_LINE: usize = 512;
/// Appended to a line cut at the per-line limit or by a full ring.
pub const CUT_MARKER: &[u8] = b" ...[cut]";

/// Log tags the shipped (non-`diag`) build writes: the install line and the late seam report
/// (`[TrainingSeams] late …`, R-A0b), boot/panic breadcrumbs and
/// the sparse lifecycle/boundary events (scene, swap submit/ready/refused, teardown, guard
/// installs), plus the opt-in Preload Samples lines users compare on their Switch
/// (docs/cycle-preload.md). Everything else (probes, observers, traces, metrics, `[Ledger]`, `[Destroy]`,
/// `[Hud]`, `[Scope]`, stand-ins) is written only by `--features diag` builds. Sorted (host test).
pub const SHIPPED_TAGS: &[&str] = &[
    "[BodyBank]",
    "[CharacterRoster]",
    "[CpuPilot]",
    "[CyclePreload]",
    "[EffectHookChain]",
    "[GroupSwap]",
    "[IcPair]",
    "[KirbyHats]",
    "[KirbyPark]",
    "[LobbyPractice]",
    "[Migration]",
    "[MiiHelper]",
    "[MiiOrdinaryCpu]",
    "[MiiOrdinaryPlayer]",
    "[NativeGcGuard]",
    "[NativeInstall]",
    "[NativeUnlink]",
    "[PracticeDiagnostics]",
    "[PracticeSave]",
    "[PreloadMetrics]",
    "[ProcessMemory]",
    "[TrainingModpack]",
    "[TrainingRooms]",
    "[TrainingSeams]",
    "[TrioBoundary]",
    "[UiAnchor]",
];

/// The leading `[Tag]` of a line's text, if any.
pub fn tag_of(text: &[u8]) -> Option<&[u8]> {
    if text.first() != Some(&b'[') {
        return None;
    }
    let end = text.iter().take(48).position(|b| *b == b']')?;
    Some(&text[..=end])
}

/// Whether a shipped build writes this line.
pub fn shipped(text: &[u8]) -> bool {
    tag_of(text).is_some_and(|tag| SHIPPED_TAGS.iter().any(|t| t.as_bytes() == tag))
}

/// A line formatted on the caller's stack. Formatting stops (with `fmt::Error`) once the buffer is
/// full, so an over-long or filtered line costs at most `STACK_LINE` bytes of formatting.
pub struct StackLine {
    buf: [u8; STACK_LINE],
    len: usize,
    overflow: bool,
}

impl StackLine {
    pub const fn new() -> Self {
        Self { buf: [0; STACK_LINE], len: 0, overflow: false }
    }

    /// Format `args`; false when the line did not fit (`text()` then holds its first bytes).
    pub fn format(&mut self, args: fmt::Arguments<'_>) -> bool {
        self.len = 0;
        self.overflow = false;
        let ok = fmt::write(self, args).is_ok();
        ok && !self.overflow
    }

    pub fn text(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

impl Default for StackLine {
    fn default() -> Self {
        Self::new()
    }
}

impl Write for StackLine {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let room = STACK_LINE - self.len;
        let take = s.len().min(room);
        self.buf[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        if take < s.len() {
            self.overflow = true;
            return Err(fmt::Error);
        }
        Ok(())
    }
}

/// What happened to one line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Push {
    Committed,
    /// Committed, cut at `max_line` or by the ring's free space (ends with `CUT_MARKER`).
    Truncated,
    /// Not enough room for even a cut line: dropped and counted.
    Dropped,
}

/// Counters, monotonic for the session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub committed: u64,
    pub truncated: u64,
    pub dropped: u64,
    pub buffered: usize,
}

/// A fixed byte ring of newline-terminated lines.
pub struct ByteRing<const N: usize> {
    buf: [u8; N],
    head: u64,
    tail: u64,
    seq: u64,
    truncated: u64,
    dropped: u64,
}

/// Room kept for `CUT_MARKER` and the newline.
const RESERVE: usize = CUT_MARKER.len() + 1;
/// The smallest room a line is attempted in (prefix + a few text bytes + `RESERVE`).
const MIN_ROOM: usize = 48 + RESERVE;

impl<const N: usize> ByteRing<N> {
    pub const fn new() -> Self {
        Self { buf: [0; N], head: 0, tail: 0, seq: 0, truncated: 0, dropped: 0 }
    }

    pub fn buffered(&self) -> usize {
        (self.head - self.tail) as usize
    }

    pub fn stats(&self) -> Stats {
        Stats { committed: self.seq, truncated: self.truncated, dropped: self.dropped, buffered: self.buffered() }
    }

    /// Count a line the caller could not hand to the ring (e.g. the lock was busy).
    pub fn note_dropped(&mut self, lines: u64) {
        self.dropped += lines;
    }

    /// Commit one line: `"<ts> #<seq> "` then the line. `text` is the already formatted line when
    /// it fit the caller's stack buffer; otherwise `args` is formatted directly into the ring.
    /// The line is limited to `max_line` bytes (prefix included) and to the free space.
    pub fn push(&mut self, ts_ms: u64, text: Option<&[u8]>, args: fmt::Arguments<'_>, max_line: usize) -> Push {
        let free = N - self.buffered();
        let room = free.min(max_line.max(MIN_ROOM));
        if room < MIN_ROOM {
            self.dropped += 1;
            return Push::Dropped;
        }
        let seq = self.seq + 1;
        let mut sink = RingSink { ring: self, written: 0, limit: room - RESERVE, cut: false };
        let _ = write!(sink, "{} #{} ", ts_ms, seq);
        match text {
            Some(text) => sink.put(text),
            None => {
                let _ = fmt::write(&mut sink, args);
            }
        }
        let (mut written, cut) = (sink.written, sink.cut);
        if cut {
            self.put_at(written, CUT_MARKER);
            written += CUT_MARKER.len();
        }
        self.put_at(written, b"\n");
        written += 1;
        self.head += written as u64;
        self.seq += 1;
        if cut {
            self.truncated += 1;
            Push::Truncated
        } else {
            Push::Committed
        }
    }

    fn put_at(&mut self, offset: usize, bytes: &[u8]) {
        let start = (self.head as usize + offset) % N;
        let first = bytes.len().min(N - start);
        self.buf[start..start + first].copy_from_slice(&bytes[..first]);
        self.buf[..bytes.len() - first].copy_from_slice(&bytes[first..]);
    }

    /// Move whole lines (at most `out.len()` bytes) out of the ring; returns the byte count.
    /// A single line longer than `out` (impossible while `max_line <= out.len()`) is moved cut.
    pub fn drain_into(&mut self, out: &mut [u8]) -> usize {
        let n = self.buffered().min(out.len());
        if n == 0 {
            return 0;
        }
        let start = (self.tail as usize) % N;
        let first = n.min(N - start);
        out[..first].copy_from_slice(&self.buf[start..start + first]);
        out[first..n].copy_from_slice(&self.buf[..n - first]);
        let take = match out[..n].iter().rposition(|b| *b == b'\n') {
            Some(last) => last + 1,
            None => n,
        };
        self.tail += take as u64;
        take
    }
}

impl<const N: usize> Default for ByteRing<N> {
    fn default() -> Self {
        Self::new()
    }
}

struct RingSink<'a, const N: usize> {
    ring: &'a mut ByteRing<N>,
    written: usize,
    limit: usize,
    cut: bool,
}

impl<const N: usize> RingSink<'_, N> {
    fn put(&mut self, bytes: &[u8]) {
        let take = bytes.len().min(self.limit - self.written);
        self.ring.put_at(self.written, &bytes[..take]);
        self.written += take;
        if take < bytes.len() {
            self.cut = true;
        }
    }
}

impl<const N: usize> Write for RingSink<'_, N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.put(s.as_bytes());
        if self.cut {
            return Err(fmt::Error);
        }
        Ok(())
    }
}
