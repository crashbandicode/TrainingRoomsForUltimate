//! Detects cut picture files before the game casts them and records a diagnostic without changing game state.
//! The guarded hook at `main+0x37a2dd4` uses a stack line and a fixed, lock-free record.
//! See `docs/DECISIONS.md#rc16-cut-file-diagnostic`.

use core::fmt;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

/// The picture-copy seam in 13.0.5.
pub const SITE: usize = 0x37a2dd4;
/// Maximum card lines per boot.
pub const CAP: usize = 8;
/// Minimum BNTX header length.
pub const MIN_BYTES: usize = 0x20;
/// Fixed card-line length.
pub const LINE: usize = 512;
/// Maximum card-file size.
pub const FILE_CAP: u64 = 64 * 1024;
/// Persistence result when another line would exceed the cap.
pub const FILE_FULL: u32 = 0xffff_fffe;
/// NUL-terminated card-file path.
pub const PATH: &[u8] = b"sd:/ultimate/TrainingRooms/cut-files.log\0";
/// NUL-terminated ARCropolis API version export.
pub const API_SYMBOL: &[u8] = b"arcrop_api_version\0";
/// NUL-terminated fork guarantee export.
pub const GUARANTEE_SYMBOL: &[u8] = b"arcrop_refuses_cut_files\0";
/// The crate's version, in every line.
pub const TR_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The row's label (`seam_install::gate` takes labels).
pub const ROW: &str = "cut_file.picture_copied";

/// The two distinct picture-file findings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// The declared data or relocation table extends beyond the copy.
    CutFile,
    /// The relocation table is missing or has no sections.
    BrokenEnd,
}

/// Header and copy sizes for a finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The kind of finding.
    pub event: Event,
    /// Bytes copied by the game.
    pub copied: u64,
    /// Size declared in the header.
    pub declared: u32,
    /// Relocation table offset.
    pub table: u32,
}

/// Inspect only the copied bytes, returning the first matching finding.
pub fn inspect(copy: &[u8]) -> Option<Finding> {
    if copy.len() < MIN_BYTES || copy.get(..4)? != b"BNTX" || copy.get(0xc..0xe)? != [0xff, 0xfe] {
        return None;
    }
    let table = u32::from_le_bytes(copy.get(0x18..0x1c)?.try_into().ok()?);
    let declared = u32::from_le_bytes(copy.get(0x1c..0x20)?.try_into().ok()?);
    let copied = copy.len() as u64;
    let event = if u64::from(declared) > copied || u64::from(table) + 0x10 > copied {
        Event::CutFile
    } else {
        let offset = table as usize;
        let magic = copy.get(offset..offset + 4)?;
        let sections = i32::from_le_bytes(copy.get(offset + 8..offset + 12)?.try_into().ok()?);
        if magic == b"_RLT" && sections >= 1 {
            return None;
        }
        Event::BrokenEnd
    };
    Some(Finding { event, copied, declared, table })
}

/// What the hook knows about one copy besides its bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyMeta {
    /// File-table index.
    pub index: u32,
    /// Address of the game's copy.
    pub address: u64,
    /// Console uptime in milliseconds.
    pub at_ms: u64,
}

/// The boot line's ARCropolis API and fork marker.
pub struct BootLine {
    /// API version if exported.
    pub api: Option<(u32, u32)>,
    /// Nonzero fork guarantee version, if exported.
    pub guarantee: Option<u32>,
}

impl core::fmt::Display for BootLine {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let guarantee = self.guarantee.filter(|&value| value != 0);
        if self.api.is_none() && guarantee.is_none() { return f.write_str("ARCropolis not loaded"); }
        f.write_str("ARCropolis")?;
        if let Some((major, minor)) = self.api { write!(f, " API {major}.{minor}")?; }
        match guarantee {
            Some(value) => write!(f, " (fork, cut-file guarantee v{value})"),
            None => f.write_str(" (upstream or older fork)"),
        }
    }
}

/// One detection and its persistence result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The detected header condition.
    pub finding: Finding,
    /// File index, copy address and uptime.
    pub meta: CopyMeta,
    /// Fork guarantee version, if present.
    pub guarantee: Option<u32>,
    /// Zero on success, otherwise a persistence error.
    pub persisted: u32,
}

/// The append operations used by the hook and the fake sink.
pub trait Sink {
    /// Create the file if absent.
    fn create(&mut self);
    /// Open for positioned writes that may extend the file.
    fn open(&mut self) -> Result<(), u32>;
    /// Read its current size.
    fn size(&mut self) -> Result<u64, u32>;
    /// Write and flush one line.
    fn write_flush(&mut self, offset: u64, bytes: &[u8]) -> Result<(), u32>;
    /// Close a successfully opened file.
    fn close(&mut self);
}

/// Append one line under the file cap and close after a successful open.
fn persist<S: Sink>(sink: &mut S, tail: &AtomicU32, line: &[u8; LINE]) -> u32 {
    S::create(sink);
    if let Err(error) = S::open(sink) { return error; }
    let result = (|| {
        // Honor both earlier boots' bytes and this boot's reservations, even if their writes haven't landed yet.
        tail.fetch_max(S::size(sink)?.min(FILE_CAP) as u32, Ordering::Relaxed);
        let offset = tail.fetch_add(LINE as u32, Ordering::Relaxed) as u64;
        if offset + LINE as u64 > FILE_CAP { return Err(FILE_FULL); }
        S::write_flush(sink, offset, line)
    })();
    S::close(sink);
    result.err().unwrap_or(0)
}

struct LineWriter {
    bytes: [u8; LINE],
    len: usize,
}

impl fmt::Write for LineWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let count = text.len().min(LINE - 1 - self.len);
        self.bytes[self.len..self.len + count].copy_from_slice(&text.as_bytes()[..count]);
        self.len += count;
        if count == text.len() { Ok(()) } else { Err(fmt::Error) }
    }
}

/// Format one space-padded, newline-terminated card line.
pub fn line(entry: &Entry) -> [u8; LINE] {
    let mut output = LineWriter { bytes: [b' '; LINE], len: 0 };
    let _ = write!(output, "{entry}");
    output.bytes[LINE - 1] = b'\n';
    output.bytes
}

impl fmt::Display for Entry {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Finding { event, copied, declared, table } = self.finding;
        let event_name = match event { Event::CutFile => "cut_file", Event::BrokenEnd => "broken_end" };
        write!(out, "{} {event_name} declares={declared} copied={copied} table={table:#x} index={:#x} copy={:#x} arcropolis=",
            self.meta.at_ms, self.meta.index, self.meta.address)?;
        match self.guarantee.filter(|&value| value != 0) {
            Some(value) => write!(out, "fork-guarantee-v{value}")?,
            None => out.write_str("upstream-or-older-fork")?,
        }
        write!(out, " tr={TR_VERSION} | ")?;
        match (event, self.guarantee.filter(|&value| value != 0)) {
            (Event::CutFile, None) => write!(out,
                "A mod file reached the game cut short (declares {declared} bytes, {copied} copied): an ARCropolis defect that can crash the game; the crashbandicode ARCropolis fork (cut-file guarantee v1) keeps the stock file instead of a cut one."),
            (Event::CutFile, Some(_)) => write!(out,
                "Unexpected cut file with the fork (declares {declared} bytes, {copied} copied); please report."),
            (Event::BrokenEnd, _) => write!(out,
                "A picture file reached the game with a broken end ({declared} bytes, relocation table not found): not the ARCropolis cut; please report."),
        }
    }
}

impl Entry {
    fn pack(&self) -> [u64; 6] {
        [
            self.finding.copied,
            u64::from(self.finding.declared) | (u64::from(self.finding.table) << 32),
            u64::from(self.meta.index) | (u64::from(self.persisted) << 32),
            self.meta.address,
            self.meta.at_ms,
            u64::from(self.finding.event == Event::BrokenEnd)
                | (u64::from(self.guarantee.is_some()) << 8)
                | (u64::from(self.guarantee.unwrap_or(0)) << 32),
        ]
    }

    fn unpack(words: [u64; 6]) -> Entry {
        Entry {
            finding: Finding {
                event: if words[5] & 1 == 0 { Event::CutFile } else { Event::BrokenEnd },
                copied: words[0],
                declared: words[1] as u32,
                table: (words[1] >> 32) as u32,
            },
            meta: CopyMeta { index: words[2] as u32, address: words[3], at_ms: words[4] },
            guarantee: (words[5] & (1 << 8) != 0).then_some((words[5] >> 32) as u32),
            persisted: (words[2] >> 32) as u32,
        }
    }
}

struct Slot {
    ready: AtomicBool,
    words: [AtomicU64; 6],
}

impl Slot {
    const fn new() -> Self {
        Self { ready: AtomicBool::new(false), words: [const { AtomicU64::new(0) }; 6] }
    }
}

/// Eight lock-free published slots and their overflow count.
pub struct Records {
    seen: AtomicU32,
    reported: AtomicU32,
    tail: AtomicU32,
    slots: [Slot; CAP],
}

impl Default for Records {
    fn default() -> Self { Self::new() }
}

impl Records {
    /// Start a fresh boot's records.
    pub const fn new() -> Self {
        Self { seen: AtomicU32::new(0), reported: AtomicU32::new(0), tail: AtomicU32::new(0), slots: [const { Slot::new() }; CAP] }
    }

    /// Persist a detection before publishing its claimed slot.
    pub fn detected<S: Sink>(&self, sink: &mut S, finding: Finding, meta: CopyMeta, guarantee: Option<u32>) -> Option<usize> {
        let number = self.seen.fetch_add(1, Ordering::Relaxed) as usize;
        let slot = self.slots.get(number)?;
        let mut entry = Entry { finding, meta, guarantee, persisted: 0 };
        entry.persisted = persist(sink, &self.tail, &line(&entry));
        for (word, value) in slot.words.iter().zip(entry.pack()) { word.store(value, Ordering::Relaxed); }
        slot.ready.store(true, Ordering::Release);
        Some(number)
    }

    /// Take the next published slot on the single drain thread.
    pub fn take(&self) -> Option<Entry> {
        let number = self.reported.load(Ordering::Relaxed);
        let slot = self.slots.get(number as usize)?;
        if !slot.ready.load(Ordering::Acquire) { return None; }
        let entry = Entry::unpack(slot.words.each_ref().map(|word| word.load(Ordering::Relaxed)));
        self.reported.store(number + 1, Ordering::Relaxed);
        Some(entry)
    }

    /// Number of detections after the eighth.
    pub fn overflow(&self) -> u32 { self.seen.load(Ordering::Relaxed).saturating_sub(CAP as u32) }
}

/// Inspect a copy, asking for the marker and touching the sink only on a finding.
pub fn on_copy<S: Sink>(records: &Records, sink: &mut S, copy: &[u8], meta: CopyMeta, guarantee: impl FnOnce() -> Option<u32>) -> Option<usize> {
    let finding = inspect(copy)?;
    records.detected(sink, finding, meta, guarantee())
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use std::cell::Cell;

    fn bntx() -> Vec<u8> {
        let mut bytes = vec![0; 0x2068];
        bytes[..4].copy_from_slice(b"BNTX");
        bytes[0xc..0xe].copy_from_slice(&[0xff, 0xfe]);
        bytes[0x18..0x1c].copy_from_slice(&0x2000u32.to_le_bytes());
        bytes[0x1c..0x20].copy_from_slice(&0x2068u32.to_le_bytes());
        bytes[0x2000..0x2004].copy_from_slice(b"_RLT");
        bytes[0x2004..0x2008].copy_from_slice(&0x2000u32.to_le_bytes());
        bytes[0x2008..0x200c].copy_from_slice(&1i32.to_le_bytes());
        bytes
    }

    fn cut() -> Finding {
        Finding { event: Event::CutFile, copied: 4096, declared: 8296, table: 0x2000 }
    }

    fn meta(index: u32, address: u64, at_ms: u64) -> CopyMeta {
        CopyMeta { index, address, at_ms }
    }

    #[derive(Default)]
    struct FakeSink {
        calls: Vec<&'static str>,
        open: bool,
        bytes: Vec<u8>,
        fail: Option<&'static str>,
    }

    impl FakeSink {
        fn call(&mut self, name: &'static str) -> Result<(), u32> {
            self.calls.push(name);
            if self.fail == Some(name) { Err(42) } else { Ok(()) }
        }
    }

    impl Sink for FakeSink {
        fn create(&mut self) { self.calls.push("create"); }
        fn open(&mut self) -> Result<(), u32> {
            self.call("open")?;
            self.open = true;
            Ok(())
        }
        fn size(&mut self) -> Result<u64, u32> {
            self.call("size")?;
            Ok(self.bytes.len() as u64)
        }
        fn write_flush(&mut self, offset: u64, bytes: &[u8]) -> Result<(), u32> {
            self.call("write")?;
            let end = offset as usize + bytes.len();
            self.bytes.resize(end.max(self.bytes.len()), 0);
            self.bytes[offset as usize..end].copy_from_slice(bytes);
            Ok(())
        }
        fn close(&mut self) { self.calls.push("close"); self.open = false; }
    }

    #[test]
    fn whole_and_smaller_declared_files_are_not_flagged() {
        let mut bytes = bntx();
        assert_eq!(inspect(&bytes), None);
        bytes[0x1c..0x20].copy_from_slice(&0x1000u32.to_le_bytes());
        assert_eq!(inspect(&bytes), None);
    }

    #[test]
    fn cut_file_reports_header_and_copy_numbers() {
        assert_eq!(inspect(&bntx()[..4096]), Some(cut()));
    }

    #[test]
    fn table_beyond_or_straddling_copy_is_not_read() {
        let mut bytes = bntx();
        bytes[0x1c..0x20].copy_from_slice(&4096u32.to_le_bytes());
        assert_eq!(inspect(&bytes[..4096]), Some(Finding { declared: 4096, ..cut() }));
        for table in [4088u32, u32::MAX] {
            bytes[0x18..0x1c].copy_from_slice(&table.to_le_bytes());
            assert_eq!(inspect(&bytes[..4096]), Some(Finding { declared: 4096, table, ..cut() }));
        }
    }

    #[test]
    fn missing_relocation_magic_or_count_is_broken_end() {
        let mut bytes = bntx();
        bytes[0x2000] = 0;
        assert_eq!(inspect(&bytes), Some(Finding { event: Event::BrokenEnd, copied: 8296, ..cut() }));
        bytes[0x2000] = b'_';
        bytes[0x2008..0x200c].copy_from_slice(&0i32.to_le_bytes());
        assert_eq!(inspect(&bytes), Some(Finding { event: Event::BrokenEnd, copied: 8296, ..cut() }));
        bytes[0x2008..0x200c].copy_from_slice(&(-1i32).to_le_bytes());
        assert_eq!(inspect(&bytes), Some(Finding { event: Event::BrokenEnd, copied: 8296, ..cut() }));
    }

    #[test]
    fn other_data_is_ignored() {
        let mut bytes = bntx();
        bytes[0] = b'X';
        assert_eq!(inspect(&bytes), None);
        bytes[0] = b'B';
        bytes[0xc] = 0;
        assert_eq!(inspect(&bytes), None);
        assert_eq!(inspect(&bntx()[..31]), None);
        assert_eq!(inspect(&[]), None);
    }

    #[test]
    fn detection_persists_a_fixed_line_before_publication() {
        let records = Records::new();
        let mut sink = FakeSink::default();
        assert_eq!(records.detected(&mut sink, cut(), meta(0x27, 0xabc, 1234), None), Some(0));
        assert_eq!(sink.calls, ["create", "open", "size", "write", "close"]);
        assert!(!sink.open);
        assert_eq!(sink.bytes.len(), LINE);
        assert_eq!(sink.bytes[LINE - 1], b'\n');
        assert!(sink.bytes.starts_with(b"1234 cut_file declares=8296 copied=4096"));
        assert_eq!(records.take(), Some(Entry {
            finding: cut(), meta: meta(0x27, 0xabc, 1234), guarantee: None, persisted: 0,
        }));
        assert_eq!(records.take(), None);
    }

    #[test]
    fn overlapping_successful_detections_keep_both_card_lines() {
        use std::cell::RefCell;
        struct Handle<'a> {
            file: &'a RefCell<Vec<u8>>,
            after_size: Option<&'a mut dyn FnMut()>,
        }
        impl Sink for Handle<'_> {
            fn create(&mut self) {}
            fn open(&mut self) -> Result<(), u32> { Ok(()) }
            fn size(&mut self) -> Result<u64, u32> {
                let observed = self.file.borrow().len();
                if let Some(other) = self.after_size.take() { other(); }
                Ok(observed as u64)
            }
            fn write_flush(&mut self, offset: u64, bytes: &[u8]) -> Result<(), u32> {
                let mut file = self.file.borrow_mut();
                let from = offset as usize;
                let end = from + bytes.len();
                if end > file.len() { file.resize(end, 0); }
                file[from..end].copy_from_slice(bytes);
                Ok(())
            }
            fn close(&mut self) {}
        }
        let records = Records::new();
        let file = RefCell::new(vec![b'P'; LINE]);
        let mut other = || {
            let mut handle = Handle { file: &file, after_size: None };
            assert_eq!(records.detected(&mut handle, cut(), meta(2, 0, 2), None), Some(1));
        };
        let mut first = Handle { file: &file, after_size: Some(&mut other) };
        assert_eq!(records.detected(&mut first, cut(), meta(1, 0, 1), None), Some(0));
        let entries = [records.take().unwrap(), records.take().unwrap()];
        assert!(entries.iter().all(|entry| entry.persisted == 0));
        let bytes = file.borrow();
        assert_eq!(bytes.len(), 3 * LINE, "two successful appends must leave two lines after the old contents");
        assert_eq!(&bytes[..LINE], &[b'P'; LINE]);
        for entry in &entries {
            assert!(bytes[LINE..].as_chunks::<LINE>().0.contains(&line(entry)));
        }
    }

    #[test]
    fn sink_failures_close_if_open_succeeded() {
        let line = [b' '; LINE];
        let tail = AtomicU32::new(0);
        for failure in ["size", "write"] {
            let mut sink = FakeSink { fail: Some(failure), ..FakeSink::default() };
            assert_eq!(persist(&mut sink, &tail, &line), 42);
            assert!(!sink.open);
            assert_eq!(sink.calls.last(), Some(&"close"));
        }
        let mut sink = FakeSink { fail: Some("open"), ..FakeSink::default() };
        assert_eq!(persist(&mut sink, &tail, &line), 42);
        assert_eq!(sink.calls, ["create", "open"]);
        assert!(sink.bytes.is_empty());
        assert!(!sink.open);
        let records = Records::new();
        let mut sink = FakeSink { fail: Some("write"), ..FakeSink::default() };
        assert_eq!(records.detected(&mut sink, cut(), meta(0, 1, 2), None), Some(0));
        assert_eq!(records.take().map(|entry| entry.persisted), Some(42));
        assert!(!sink.open);
    }

    #[test]
    fn ninth_event_only_increments_overflow() {
        let records = Records::new();
        let mut sink = FakeSink::default();
        for i in 0..CAP {
            assert_eq!(records.detected(&mut sink, cut(), meta(i as u32, 4, 9), None), Some(i));
        }
        let calls = sink.calls.len();
        let length = sink.bytes.len();
        assert_eq!(records.detected(&mut sink, cut(), meta(99, 4, 9), None), None);
        assert_eq!(sink.calls.len(), calls);
        assert_eq!(sink.bytes.len(), length);
        assert_eq!(records.overflow(), 1);
        for i in 0..CAP {
            assert_eq!(records.take().map(|entry| entry.meta.index), Some(i as u32));
        }
        assert_eq!(records.take(), None);
    }

    #[test]
    fn whole_file_never_touches_sink_or_asks_for_guarantee() {
        let records = Records::new();
        let mut sink = FakeSink::default();
        let asked = Cell::new(false);
        let result = on_copy(&records, &mut sink, &bntx(), meta(0, 0, 0), || {
            asked.set(true);
            Some(1)
        });
        assert_eq!(result, None);
        assert!(!asked.get());
        assert!(sink.calls.is_empty());
        assert_eq!(records.take(), None);
        let bytes = bntx();
        let result = on_copy(&records, &mut sink, &bytes[..4096], meta(7, 0x1234, 55), || {
            asked.set(true);
            Some(1)
        });
        assert_eq!(result, Some(0));
        assert!(asked.get());
        assert_eq!(records.take(), Some(Entry {
            finding: cut(), meta: meta(7, 0x1234, 55), guarantee: Some(1), persisted: 0,
        }));
    }

    #[test]
    fn sentences_are_event_specific_and_the_longest_line_fits() {
        let entry = Entry {
            finding: Finding { event: Event::CutFile, copied: u64::MAX, declared: u32::MAX, table: u32::MAX },
            meta: meta(u32::MAX, u64::MAX, u64::MAX), guarantee: None, persisted: u32::MAX,
        };
        let text = entry.to_string();
        assert_eq!(text, format!(
            "{} cut_file declares={} copied={} table={:#x} index={:#x} copy={:#x} arcropolis=upstream-or-older-fork tr={} | A mod file reached the game cut short (declares {} bytes, {} copied): an ARCropolis defect that can crash the game; the crashbandicode ARCropolis fork (cut-file guarantee v1) keeps the stock file instead of a cut one.",
            u64::MAX, u32::MAX, u64::MAX, u32::MAX, u32::MAX, u64::MAX, TR_VERSION, u32::MAX, u64::MAX
        ));
        assert!(text.len() < LINE);
        let fixed = line(&entry);
        assert_eq!(&fixed[..text.len()], text.as_bytes());
        assert!(fixed[text.len()..LINE - 1].iter().all(|&byte| byte == b' '));
        assert_eq!(fixed[LINE - 1], b'\n');
        let fork = Entry { guarantee: Some(u32::MAX), ..entry }.to_string();
        assert_eq!(fork, format!(
            "{} cut_file declares={} copied={} table={:#x} index={:#x} copy={:#x} arcropolis=fork-guarantee-v{} tr={} | Unexpected cut file with the fork (declares {} bytes, {} copied); please report.",
            u64::MAX, u32::MAX, u64::MAX, u32::MAX, u32::MAX, u64::MAX, u32::MAX, TR_VERSION, u32::MAX, u64::MAX
        ));
        assert_eq!(Entry { guarantee: Some(0), ..entry }.to_string(), text);
        let broken = Entry { finding: Finding { event: Event::BrokenEnd, ..entry.finding }, guarantee: Some(1), ..entry }.to_string();
        assert_eq!(broken, format!(
            "{} broken_end declares={} copied={} table={:#x} index={:#x} copy={:#x} arcropolis=fork-guarantee-v1 tr={} | A picture file reached the game with a broken end ({} bytes, relocation table not found): not the ARCropolis cut; please report.",
            u64::MAX, u32::MAX, u64::MAX, u32::MAX, u32::MAX, u64::MAX, TR_VERSION, u32::MAX
        ));
        for value in [Entry { guarantee: Some(u32::MAX), ..entry },
            Entry { finding: Finding { event: Event::BrokenEnd, ..entry.finding }, guarantee: Some(u32::MAX), ..entry }] {
            let text = value.to_string();
            let fixed = line(&value);
            assert!(text.len() < LINE);
            assert_eq!(&fixed[..text.len()], text.as_bytes());
            assert_eq!(fixed[text.len() - 1], b'.');
        }
    }

    #[test]
    fn boot_line_classifies_api_and_real_guarantee() {
        assert_eq!(BootLine { api: None, guarantee: None }.to_string(), "ARCropolis not loaded");
        assert_eq!(BootLine { api: None, guarantee: Some(0) }.to_string(), "ARCropolis not loaded");
        assert_eq!(BootLine { api: Some((1, 9)), guarantee: None }.to_string(), "ARCropolis API 1.9 (upstream or older fork)");
        assert_eq!(BootLine { api: Some((1, 9)), guarantee: Some(0) }.to_string(), "ARCropolis API 1.9 (upstream or older fork)");
        assert_eq!(BootLine { api: Some((1, 9)), guarantee: Some(1) }.to_string(), "ARCropolis API 1.9 (fork, cut-file guarantee v1)");
        assert_eq!(BootLine { api: None, guarantee: Some(1) }.to_string(), "ARCropolis (fork, cut-file guarantee v1)");
    }

    #[test]
    fn packed_entries_round_trip_every_field() {
        let entry = Entry {
            finding: Finding { event: Event::BrokenEnd, copied: u64::MAX, declared: u32::MAX, table: u32::MAX },
            meta: meta(u32::MAX, u64::MAX, u64::MAX),
            guarantee: Some(u32::MAX),
            persisted: u32::MAX,
        };
        for guarantee in [Some(u32::MAX), None, Some(1), Some(0)] {
            for event in [Event::CutFile, Event::BrokenEnd] {
                let value = Entry { finding: Finding { event, ..entry.finding }, guarantee, ..entry };
                assert_eq!(Entry::unpack(value.pack()), value);
            }
        }
    }

    #[test]
    fn file_stops_at_the_fixed_cap() {
        let mut sink = FakeSink { bytes: vec![0; FILE_CAP as usize - LINE], ..FakeSink::default() };
        let line = [b' '; LINE];
        let tail = AtomicU32::new(0);
        assert_eq!(persist(&mut sink, &tail, &line), 0);
        assert_eq!(sink.bytes.len(), FILE_CAP as usize);
        sink.calls.clear();
        assert_eq!(persist(&mut sink, &tail, &line), FILE_FULL);
        assert_eq!(sink.calls, ["create", "open", "size", "close"]);
        assert!(!sink.open);
        assert_eq!(sink.bytes.len(), FILE_CAP as usize);
    }
    }
}
