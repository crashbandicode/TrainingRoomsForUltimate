use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

pub const SITE: usize = 0x37a2dd4;
pub const CAP: usize = 8;
pub const MIN_BYTES: usize = 0x20;
pub const LINE: usize = 512;
pub const FILE_CAP: u64 = 64 * 1024;
pub const FILE_FULL: u32 = 0xffff_fffe;
pub const PATH: &[u8] = b"sd:/ultimate/TrainingRooms/cut-files.log\0";
pub const API_SYMBOL: &[u8] = b"arcrop_api_version\0";
pub const GUARANTEE_SYMBOL: &[u8] = b"arcrop_refuses_cut_files\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event { CutFile, BrokenEnd }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Finding {
    pub event: Event,
    pub copied: u64,
    pub declared: u32,
    pub table: u32,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arcropolis {
    NotLoaded,
    UpstreamOrOlderFork { api: Option<(u32, u32)> },
    Fork { api: Option<(u32, u32)>, guarantee: u32 },
}

pub fn classify(api: Option<(u32, u32)>, guarantee: Option<u32>) -> Arcropolis {
    match (api, guarantee.filter(|&value| value != 0)) {
        (api, Some(guarantee)) => Arcropolis::Fork { api, guarantee },
        (Some(api), None) => Arcropolis::UpstreamOrOlderFork { api: Some(api) },
        (None, None) => Arcropolis::NotLoaded,
    }
}

impl core::fmt::Display for Arcropolis {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotLoaded => f.write_str("ARCropolis not loaded"),
            Self::UpstreamOrOlderFork { api } => {
                f.write_str("ARCropolis")?;
                if let Some((major, minor)) = api { write!(f, " API {major}.{minor}")?; }
                f.write_str(" (upstream or older fork)")
            }
            Self::Fork { api, guarantee } => {
                f.write_str("ARCropolis")?;
                if let Some((major, minor)) = api { write!(f, " API {major}.{minor}")?; }
                write!(f, " (fork, cut-file guarantee v{guarantee})")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub finding: Finding,
    pub index: u32,
    pub copy: u64,
    pub at_ms: u64,
    pub guarantee: Option<u32>,
    pub persisted: u32,
}

pub trait Sink {
    fn create(&mut self);
    fn open(&mut self) -> Result<(), u32>;
    fn size(&mut self) -> Result<u64, u32>;
    fn grow(&mut self, size: u64) -> Result<(), u32>;
    fn write_flush(&mut self, offset: u64, bytes: &[u8]) -> Result<(), u32>;
    fn close(&mut self);
}

pub fn persist<S: Sink>(sink: &mut S, line: &[u8; LINE]) -> u32 {
    S::create(sink);
    if let Err(error) = S::open(sink) { return error; }
    let result = (|| {
        let offset = S::size(sink)?;
        let next = offset.checked_add(LINE as u64).filter(|&size| size <= FILE_CAP).ok_or(FILE_FULL)?;
        S::grow(sink, next)?;
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

pub fn line(entry: &Entry, tr_version: &str) -> [u8; LINE] {
    let mut output = LineWriter { bytes: [b' '; LINE], len: 0 };
    let _ = write_text(&mut output, entry, tr_version);
    output.bytes[LINE - 1] = b'\n';
    output.bytes
}

pub fn write_text(out: &mut impl core::fmt::Write, entry: &Entry, tr_version: &str) -> core::fmt::Result {
    let Finding { event, copied, declared, table } = entry.finding;
    let event_name = match event { Event::CutFile => "cut_file", Event::BrokenEnd => "broken_end" };
    write!(out, "{} {event_name} declares={declared} copied={copied} table={table:#x} index={:#x} copy={:#x} arcropolis=",
        entry.at_ms, entry.index, entry.copy)?;
    match entry.guarantee.filter(|&value| value != 0) {
        Some(value) => write!(out, "fork-guarantee-v{value}")?,
        None => out.write_str("upstream-or-older-fork")?,
    }
    write!(out, " tr={tr_version} | ")?;
    match (event, entry.guarantee.filter(|&value| value != 0)) {
        (Event::CutFile, None) => write!(out,
            "A mod file reached the game cut short (declares {declared} bytes, {copied} copied): an ARCropolis defect that can crash the game; the crashbandicode ARCropolis fork (cut-file guarantee v1) refuses cut files."),
        (Event::CutFile, Some(_)) => write!(out,
            "Unexpected cut file with the fork (declares {declared} bytes, {copied} copied); please report."),
        (Event::BrokenEnd, _) => write!(out,
            "A picture file reached the game with a broken end ({declared} bytes, relocation table not found): not the ARCropolis cut; please report."),
    }
}

struct Slot {
    ready: AtomicBool,
    event: AtomicU32,
    copied: AtomicU64,
    declared: AtomicU32,
    table: AtomicU32,
    index: AtomicU32,
    copy: AtomicU64,
    at_ms: AtomicU64,
    has_guarantee: AtomicBool,
    guarantee: AtomicU32,
    persisted: AtomicU32,
}

impl Slot {
    const fn new() -> Self {
        Self {
            ready: AtomicBool::new(false), event: AtomicU32::new(0), copied: AtomicU64::new(0),
            declared: AtomicU32::new(0), table: AtomicU32::new(0), index: AtomicU32::new(0),
            copy: AtomicU64::new(0), at_ms: AtomicU64::new(0), has_guarantee: AtomicBool::new(false),
            guarantee: AtomicU32::new(0), persisted: AtomicU32::new(0),
        }
    }
}

pub struct Records {
    seen: AtomicU32,
    reported: AtomicU32,
    slots: [Slot; CAP],
}

impl Default for Records {
    fn default() -> Self { Self::new() }
}

impl Records {
    pub const fn new() -> Self {
        Self { seen: AtomicU32::new(0), reported: AtomicU32::new(0), slots: [const { Slot::new() }; CAP] }
    }

    pub fn detected<S: Sink>(
        &self, sink: &mut S, finding: Finding, index: u32, copy: u64, at_ms: u64,
        guarantee: Option<u32>, tr_version: &str,
    ) -> Option<usize> {
        let number = self.seen.fetch_add(1, Ordering::Relaxed) as usize;
        let slot = self.slots.get(number)?;
        let entry = Entry { finding, index, copy, at_ms, guarantee, persisted: 0 };
        let persisted = persist(sink, &line(&entry, tr_version));
        slot.event.store(u32::from(finding.event == Event::BrokenEnd), Ordering::Relaxed);
        slot.copied.store(finding.copied, Ordering::Relaxed);
        slot.declared.store(finding.declared, Ordering::Relaxed);
        slot.table.store(finding.table, Ordering::Relaxed);
        slot.index.store(index, Ordering::Relaxed);
        slot.copy.store(copy, Ordering::Relaxed);
        slot.at_ms.store(at_ms, Ordering::Relaxed);
        slot.has_guarantee.store(guarantee.is_some(), Ordering::Relaxed);
        slot.guarantee.store(guarantee.unwrap_or(0), Ordering::Relaxed);
        slot.persisted.store(persisted, Ordering::Relaxed);
        slot.ready.store(true, Ordering::Release);
        Some(number)
    }

    pub fn take(&self) -> Option<Entry> {
        let number = self.reported.load(Ordering::Relaxed);
        let slot = self.slots.get(number as usize)?;
        if !slot.ready.load(Ordering::Acquire) { return None; }
        let entry = Entry {
            finding: Finding {
                event: if slot.event.load(Ordering::Relaxed) == 0 { Event::CutFile } else { Event::BrokenEnd },
                copied: slot.copied.load(Ordering::Relaxed),
                declared: slot.declared.load(Ordering::Relaxed),
                table: slot.table.load(Ordering::Relaxed),
            },
            index: slot.index.load(Ordering::Relaxed),
            copy: slot.copy.load(Ordering::Relaxed),
            at_ms: slot.at_ms.load(Ordering::Relaxed),
            guarantee: slot.has_guarantee.load(Ordering::Relaxed).then(|| slot.guarantee.load(Ordering::Relaxed)),
            persisted: slot.persisted.load(Ordering::Relaxed),
        };
        self.reported.store(number + 1, Ordering::Relaxed);
        Some(entry)
    }

    pub fn overflow(&self) -> u32 { self.seen.load(Ordering::Relaxed).saturating_sub(CAP as u32) }
}

pub fn on_copy<S: Sink>(
    records: &Records, sink: &mut S, copy: &[u8], index: u32, address: u64,
    at_ms: u64, guarantee: impl FnOnce() -> Option<u32>, tr_version: &str,
) -> Option<usize> {
    let finding = inspect(copy)?;
    records.detected(sink, finding, index, address, at_ms, guarantee(), tr_version)
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
        fn grow(&mut self, size: u64) -> Result<(), u32> {
            self.call("grow")?;
            self.bytes.resize(size as usize, 0);
            Ok(())
        }
        fn write_flush(&mut self, offset: u64, bytes: &[u8]) -> Result<(), u32> {
            self.call("write")?;
            self.bytes[offset as usize..offset as usize + bytes.len()].copy_from_slice(bytes);
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
        assert_eq!(records.detected(&mut sink, cut(), 0x27, 0xabc, 1234, None, "1.2.3"), Some(0));
        assert_eq!(sink.calls, ["create", "open", "size", "grow", "write", "close"]);
        assert!(!sink.open);
        assert_eq!(sink.bytes.len(), LINE);
        assert_eq!(sink.bytes[LINE - 1], b'\n');
        assert!(sink.bytes.starts_with(b"1234 cut_file declares=8296 copied=4096"));
        assert_eq!(records.take(), Some(Entry {
            finding: cut(), index: 0x27, copy: 0xabc, at_ms: 1234, guarantee: None, persisted: 0,
        }));
        assert_eq!(records.take(), None);
    }

    #[test]
    fn sink_failures_close_if_open_succeeded() {
        let line = [b' '; LINE];
        for failure in ["size", "grow", "write"] {
            let mut sink = FakeSink { fail: Some(failure), ..FakeSink::default() };
            assert_eq!(persist(&mut sink, &line), 42);
            assert!(!sink.open);
            assert_eq!(sink.calls.last(), Some(&"close"));
        }
        let mut sink = FakeSink { fail: Some("open"), ..FakeSink::default() };
        assert_eq!(persist(&mut sink, &line), 42);
        assert_eq!(sink.calls, ["create", "open"]);
        assert!(sink.bytes.is_empty());
        assert!(!sink.open);
        let records = Records::new();
        let mut sink = FakeSink { fail: Some("write"), ..FakeSink::default() };
        assert_eq!(records.detected(&mut sink, cut(), 0, 1, 2, None, "1"), Some(0));
        assert_eq!(records.take().map(|entry| entry.persisted), Some(42));
        assert!(!sink.open);
    }

    #[test]
    fn ninth_event_only_increments_overflow() {
        let records = Records::new();
        let mut sink = FakeSink::default();
        for i in 0..CAP {
            assert_eq!(records.detected(&mut sink, cut(), i as u32, 4, 9, None, "1"), Some(i));
        }
        let calls = sink.calls.len();
        let length = sink.bytes.len();
        assert_eq!(records.detected(&mut sink, cut(), 99, 4, 9, None, "1"), None);
        assert_eq!(sink.calls.len(), calls);
        assert_eq!(sink.bytes.len(), length);
        assert_eq!(records.overflow(), 1);
        for i in 0..CAP {
            assert_eq!(records.take().map(|entry| entry.index), Some(i as u32));
        }
        assert_eq!(records.take(), None);
    }

    #[test]
    fn whole_file_never_touches_sink_or_asks_for_guarantee() {
        let records = Records::new();
        let mut sink = FakeSink::default();
        let asked = Cell::new(false);
        let result = on_copy(&records, &mut sink, &bntx(), 0, 0, 0, || {
            asked.set(true);
            Some(1)
        }, "1");
        assert_eq!(result, None);
        assert!(!asked.get());
        assert!(sink.calls.is_empty());
        assert_eq!(records.take(), None);
        let bytes = bntx();
        let result = on_copy(&records, &mut sink, &bytes[..4096], 7, 0x1234, 55, || {
            asked.set(true);
            Some(1)
        }, "1");
        assert_eq!(result, Some(0));
        assert!(asked.get());
        assert_eq!(records.take(), Some(Entry {
            finding: cut(), index: 7, copy: 0x1234, at_ms: 55, guarantee: Some(1), persisted: 0,
        }));
    }

    #[test]
    fn sentences_are_event_specific_and_the_longest_line_fits() {
        let entry = Entry {
            finding: Finding { event: Event::CutFile, copied: u64::MAX, declared: u32::MAX, table: u32::MAX },
            index: u32::MAX, copy: u64::MAX, at_ms: u64::MAX, guarantee: None, persisted: 0,
        };
        let mut text = String::new();
        write_text(&mut text, &entry, "255.255.255").unwrap();
        assert!(text.contains("an ARCropolis defect that can crash the game; the crashbandicode ARCropolis fork (cut-file guarantee v1) refuses cut files."));
        assert!(text.len() < LINE);
        let fixed = line(&entry, "255.255.255");
        assert_eq!(&fixed[..text.len()], text.as_bytes());
        assert!(fixed[text.len()..LINE - 1].iter().all(|&byte| byte == b' '));
        assert_eq!(fixed[LINE - 1], b'\n');
        let mut fork = String::new();
        write_text(&mut fork, &Entry { guarantee: Some(1), ..entry }, "1").unwrap();
        assert!(fork.contains("Unexpected cut file with the fork"));
        assert!(!fork.contains("ARCropolis defect"));
        let mut zero = String::new();
        write_text(&mut zero, &Entry { guarantee: Some(0), ..entry }, "1").unwrap();
        assert!(zero.contains("arcropolis=upstream-or-older-fork"));
        assert!(zero.contains("an ARCropolis defect"));
        let mut broken = String::new();
        write_text(&mut broken, &Entry { finding: Finding { event: Event::BrokenEnd, ..entry.finding }, ..entry }, "1").unwrap();
        assert!(broken.contains("not the ARCropolis cut; please report."));
        assert!(!broken.contains("ARCropolis defect"));
        assert!(!broken.contains("refuses cut files"));
    }

    #[test]
    fn boot_line_classifies_api_and_real_guarantee() {
        assert_eq!(classify(None, None).to_string(), "ARCropolis not loaded");
        assert_eq!(classify(None, Some(0)).to_string(), "ARCropolis not loaded");
        assert_eq!(classify(Some((1, 9)), None).to_string(), "ARCropolis API 1.9 (upstream or older fork)");
        assert_eq!(classify(Some((1, 9)), Some(0)).to_string(), "ARCropolis API 1.9 (upstream or older fork)");
        assert_eq!(classify(Some((1, 9)), Some(1)).to_string(), "ARCropolis API 1.9 (fork, cut-file guarantee v1)");
        assert_eq!(classify(None, Some(2)).to_string(), "ARCropolis (fork, cut-file guarantee v2)");
    }

    #[test]
    fn file_stops_at_the_fixed_cap() {
        let mut sink = FakeSink { bytes: vec![0; FILE_CAP as usize - LINE], ..FakeSink::default() };
        let line = [b' '; LINE];
        assert_eq!(persist(&mut sink, &line), 0);
        assert_eq!(sink.bytes.len(), FILE_CAP as usize);
        sink.calls.clear();
        assert_eq!(persist(&mut sink, &line), FILE_FULL);
        assert_eq!(sink.calls, ["create", "open", "size", "close"]);
        assert!(!sink.open);
        assert_eq!(sink.bytes.len(), FILE_CAP as usize);
    }
    }
}
