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

pub fn inspect(_copy: &[u8]) -> Option<Finding> { None }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arcropolis {
    NotLoaded,
    UpstreamOrOlderFork { api: Option<(u32, u32)> },
    Fork { api: Option<(u32, u32)>, guarantee: u32 },
}

pub fn classify(_api: Option<(u32, u32)>, _guarantee: Option<u32>) -> Arcropolis {
    Arcropolis::NotLoaded
}

impl core::fmt::Display for Arcropolis {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { Ok(()) }
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

pub fn persist<S: Sink>(_sink: &mut S, _line: &[u8; LINE]) -> u32 { 0 }

pub fn line(_entry: &Entry, _tr_version: &str) -> [u8; LINE] { [b' '; LINE] }

pub fn write_text(_out: &mut impl core::fmt::Write, _entry: &Entry, _tr_version: &str) -> core::fmt::Result {
    Ok(())
}

pub struct Records;

impl Records {
    pub const fn new() -> Self { Self }

    pub fn detected<S: Sink>(
        &self, _sink: &mut S, _finding: Finding, _index: u32, _copy: u64, _at_ms: u64,
        _guarantee: Option<u32>, _tr_version: &str,
    ) -> Option<usize> { None }

    pub fn take(&self) -> Option<Entry> { None }

    pub fn overflow(&self) -> u32 { 0 }
}

pub fn on_copy<S: Sink>(
    _records: &Records, _sink: &mut S, _copy: &[u8], _index: u32, _address: u64,
    _at_ms: u64, _guarantee: impl FnOnce() -> Option<u32>, _tr_version: &str,
) -> Option<usize> { None }

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
