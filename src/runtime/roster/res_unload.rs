//! Operator option C (inbox 2026-11-09 17:57Z; primary 19:47Z): the release watch's reads of the game's resource unload
//! and its purge bookkeeping (`engine::swap::res_unload`, LANE-OPUS-RC6-PRELOAD-PRIORITY-REPORT.md F6). Nothing here
//! calls the game or takes a native lock; nothing is read unless every row of the `res_unload` group
//! (`native_seams::rows_res_unload`) passed.
//!
//! - [`capture`] runs on the game thread right after a free's release (`leave_group`). It scans the service's
//!   directory and data tables, which live for the process, for what is queued now: directories in use and
//!   unreferenced, files used and unreferenced. The update thread may be draining the queue meanwhile; what it has
//!   unloaded already is done.
//! - [`waiting`] runs in each planner pass: the captured directories first (`state` by an acquire load), then files.
//! - The purge: the planner requests one once the unload is done ([`request_purge`]); the bank's MainThread hook runs it
//!   (`native_body_bank::purge_on_main_thread`) and records it ([`purged`]); the next pass settles (`ReleaseWatch`).

use crate::engine::seams::native_fields as fields;
use crate::engine::swap::res_unload::{
    Capture, Directory, File, Unloading, DATA_REFS, DATA_SIZE, DATA_STATE, DATA_USED, DIRECTORY_FLAGS, DIRECTORY_REFS,
    DIRECTORY_SIZE, DIRECTORY_STATE, FS_DATA, FS_DATA_COUNT, FS_DIRECTORIES, FS_DIRECTORY_COUNT, FS_GLOBAL, MAX_TABLE,
};
use hos_lock::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering};

/// The rows the layout and the purge come from (`native_seams::rows_res_unload`); all or nothing.
pub(crate) const SEAMS: &[&str] = &["res_unload.fs_global", "res_unload.directory_release", "res_unload.file_release",
    "res_unload.pass_entry", "res_unload.pass_directory_reset", "res_unload.pass_file_reset", "res_unload.update_thread",
    "res_unload.inflate_thread", "res_unload.mallctl", "res_unload.mallctl_caller", "res_unload.purge_ctl",
    "res_unload.arena_decay"];
/// `main` once the rows passed; 0 reads nothing and purges nothing.
static BASE: AtomicUsize = AtomicUsize::new(0);
/// What the frees since the watch last settled left queued. Taken alone, never under another lock.
static CAPTURE: Mutex<Capture> = Mutex::new(Capture::new());
/// Bumped by every capture, so a settle forgets only the captures it read.
static GENERATION: AtomicU32 = AtomicU32::new(0);
/// The purge the planner last requested and the last one the MainThread hook completed (`ReleaseWatch::Purging`).
static REQUESTED: AtomicU32 = AtomicU32::new(0);
static COMPLETED: AtomicU32 = AtomicU32::new(0);

/// Once from the bank's install, with its gate of [`SEAMS`] (group `res_unload`): nothing is read unless all passed.
pub(crate) fn install(base: usize, rows: bool) {
    if rows { BASE.store(base, Ordering::Release); }
    crate::engine::compat::logging::diagnostic(format_args!("[GroupSwap] release watch reads the resource unload and purges={rows}"));
}

/// `main` for the purge, once the rows passed.
pub(crate) fn base() -> Option<usize> {
    Some(BASE.load(Ordering::Acquire)).filter(|base| *base != 0)
}

/// The service's directory and data tables.
#[derive(Clone, Copy)]
struct Table {
    data: usize,
    data_count: u32,
    directories: usize,
    directory_count: u32,
}

impl Table {
    /// # Safety
    /// `fs` is the service's table (`*(main+0x5332f20)`, guarded by the rows); it lives for the process.
    unsafe fn read(fs: usize) -> Option<Table> {
        let t = Table {
            data: fields::child(fs, FS_DATA)?,
            data_count: fields::read::<u32>(fs, FS_DATA_COUNT)?,
            directories: fields::child(fs, FS_DIRECTORIES)?,
            directory_count: fields::read::<u32>(fs, FS_DIRECTORY_COUNT)?,
        };
        (t.data_count <= MAX_TABLE && t.directory_count <= MAX_TABLE).then_some(t)
    }
    fn directory(self, index: u32) -> Option<usize> {
        (index < self.directory_count).then(|| self.directories + index as usize * DIRECTORY_SIZE)
    }
    fn data(self, index: u32) -> Option<usize> {
        (index < self.data_count).then(|| self.data + index as usize * DATA_SIZE)
    }
    /// # Safety
    /// `at` is an entry of this table.
    unsafe fn directory_at(at: usize) -> Directory {
        let state = acquire_u8(at + DIRECTORY_STATE);
        Directory { state, flags: fields::read_unchecked(at, DIRECTORY_FLAGS), refs: fields::read_unchecked(at, DIRECTORY_REFS) }
    }
    /// # Safety
    /// `at` is an entry of this table.
    unsafe fn file_at(at: usize) -> File {
        let state = acquire_u8(at + DATA_STATE);
        File { state, used: fields::read_unchecked(at, DATA_USED), refs: fields::read_unchecked(at, DATA_REFS) }
    }
}

/// # Safety
/// A live byte the game writes with `stlrb` (the entries' `state`).
unsafe fn acquire_u8(address: usize) -> u8 {
    (*(address as *const AtomicU8)).load(Ordering::Acquire)
}

/// The table, once the rows passed.
fn table() -> Option<Table> {
    let base = base()?;
    // SAFETY: `main+0x5332f20` is the service's table pointer (rows `res_unload.*`); the table lives for the process.
    unsafe { Table::read(fields::child(base, FS_GLOBAL)?) }
}

/// After a free's release, on the game thread: adds what the service has queued now to the watch. Returns the
/// directories and files added and whether the capture is complete, for the free's line; None before the rows passed.
pub(crate) fn capture() -> Option<(usize, usize, bool)> {
    let t = table()?;
    let mut capture = CAPTURE.lock();
    GENERATION.fetch_add(1, Ordering::AcqRel);
    let (directories, files) = (capture.directory_count, capture.file_count);
    // SAFETY: every index is below the table's own count; both tables live for the process.
    unsafe {
        for index in 0..t.directory_count {
            let at = t.directories + index as usize * DIRECTORY_SIZE;
            if fields::read_unchecked::<u8>(at, DIRECTORY_FLAGS) & 1 != 0 && Table::directory_at(at).waiting() {
                capture.add_directory(index);
            }
        }
        for index in 0..t.data_count {
            let at = t.data + index as usize * DATA_SIZE;
            if fields::read_unchecked::<u8>(at, DATA_USED) != 0 && Table::file_at(at).waiting() {
                capture.add_file(index);
            }
        }
    }
    Some((capture.directory_count - directories, capture.file_count - files, capture.complete))
}

/// The planner pass's read: how many captured directories and files the game has still to unload, and the capture
/// generation it read. None before the rows passed or when the table is unreadable.
pub(crate) fn waiting() -> Option<(Unloading, u32)> {
    let t = table()?;
    let capture = CAPTURE.lock();
    let mut out = Unloading::default();
    // SAFETY: entry addresses come from the table's bounds; `state` is read with acquire, directories before files.
    unsafe {
        for &index in capture.directories() {
            if t.directory(index).is_some_and(|at| Table::directory_at(at).waiting()) {
                out.directories = out.directories.saturating_add(1);
            }
        }
        for &index in capture.files() {
            if t.data(index).is_some_and(|at| Table::file_at(at).waiting()) {
                out.files = out.files.saturating_add(1);
            }
        }
    }
    Some((out, GENERATION.load(Ordering::Acquire)))
}

/// The watch settled on the read of `generation`: forget those captures, unless a free captured more since.
pub(crate) fn settled(generation: u32) {
    let mut capture = CAPTURE.lock();
    if GENERATION.load(Ordering::Acquire) == generation { capture.clear(); }
}

/// A scene change: nothing freed is watched any more, and no purge is owed.
pub(crate) fn clear() {
    CAPTURE.lock().clear();
    REQUESTED.store(COMPLETED.load(Ordering::Acquire), Ordering::Release);
}

/// The purges the MainThread hook has completed (`ReleaseWatch::pass`).
pub(crate) fn purges() -> u32 {
    COMPLETED.load(Ordering::Acquire)
}

/// The planner: the watch waits for purge `ticket` (`ReleaseWatch::requested`).
pub(crate) fn request_purge(ticket: u32) {
    REQUESTED.store(ticket, Ordering::Release);
}

/// The MainThread hook: the purge it owes, if any.
pub(crate) fn owed_purge() -> Option<u32> {
    let requested = REQUESTED.load(Ordering::Acquire);
    (requested != COMPLETED.load(Ordering::Acquire)).then_some(requested)
}

/// The MainThread hook ran purge `ticket` (after reading it from [`owed_purge`]).
pub(crate) fn purged(ticket: u32) {
    COMPLETED.store(ticket, Ordering::Release);
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::swap::body_bank::ReleaseWatch;

    /// A host copy of the service's tables, written in the unload pass's order (F6).
    struct Tables {
        fs: [u64; 16],
        data: Vec<[u8; DATA_SIZE]>,
        directories: Vec<[u8; DIRECTORY_SIZE]>,
    }

    impl Tables {
        fn new() -> Box<Tables> {
            let mut t = Box::new(Tables { fs: [0; 16], data: vec![[0; DATA_SIZE]; 8], directories: vec![[0; DIRECTORY_SIZE]; 6] });
            for d in 0..8 { t.set_data(d, 3, 1, 1); }
            for d in 0..6 { t.set_directory(d, 3, 1, 1); }
            t.fs[FS_DATA / 8] = t.data.as_ptr() as u64;
            t.fs[FS_DATA_COUNT / 8] = 8 << 32;
            t.fs[FS_DIRECTORIES / 8] = t.directories.as_ptr() as u64;
            t.fs[FS_DIRECTORY_COUNT / 8] = 6;
            t
        }
        fn set_data(&mut self, index: usize, state: u8, used: u8, refs: i32) {
            let d = &mut self.data[index];
            d[DATA_REFS..DATA_REFS + 4].copy_from_slice(&refs.to_le_bytes());
            d[DATA_USED] = used;
            d[DATA_STATE] = state;
        }
        fn set_directory(&mut self, index: usize, state: u8, flags: u8, refs: i32) {
            let d = &mut self.directories[index];
            d[DIRECTORY_REFS..DIRECTORY_REFS + 4].copy_from_slice(&refs.to_le_bytes());
            d[DIRECTORY_FLAGS] = flags;
            d[DIRECTORY_STATE] = state;
        }
        /// A `main` whose `FS_GLOBAL` slot is `slot`, holding the table pointer.
        fn main(&self, slot: &mut u64) -> usize {
            *slot = self.fs.as_ptr() as u64;
            (slot as *const u64 as usize).checked_sub(FS_GLOBAL).expect("a heap address above 0x5332f20")
        }
    }

    /// Operator option C with the primary's purge (19:47Z), the host test first. Eden `pp-re-trace-2` (`[Samus, Yoshi]`,
    /// `#840`): the free queued 12 directories and 389 files; the update thread unloaded them within 0.5 s, and jemalloc
    /// returned the pages 1.1 s and 2.9 s later. The watch captures what the free queued, holds however many passes
    /// until every captured directory and then every captured file is reset, requests one purge, holds until the
    /// MainThread hook has run it, and only then settles. A file another owner still references does not hold it.
    fn the_watch_settles_on_unloaded_then_purged_in_that_order() {
        let mut t = Tables::new();
        let mut slot = 0u64;
        BASE.store(t.main(&mut slot), Ordering::Release);
        clear();
        // The free: directories 1 and 4 and files 2, 3 and 6 drop to 0 (queued); file 5 keeps another holder.
        for d in [1, 4] { t.set_directory(d, 3, 1, 0); }
        for f in [2, 3, 6] { t.set_data(f, 3, 1, 0); }
        t.set_data(0, 0, 0, 0);
        assert_eq!(capture(), Some((2, 3, true)), "the queued directories and files; not the free slot, not the held ones");
        let read = || waiting().map(|(unloading, _)| unloading);
        let pass = |watch: ReleaseWatch| {
            let next = watch.pass(read().map(|u| u.waiting()), purges(), base().is_some());
            if let Some(ticket) = next.requested() { request_purge(ticket); }
            next
        };
        let mut watch = ReleaseWatch::Settled.freed();
        for _ in 0..120 {
            watch = pass(watch);
            assert_eq!(watch, ReleaseWatch::Unloading, "nothing inflated: the pass has not run");
            assert_eq!(owed_purge(), None, "no purge before the unload ends");
        }
        // The pass: directory 1's reset (flags first, then state), directory 4 still queued.
        t.set_directory(1, 3, 0, 0);
        assert_eq!(read(), Some(Unloading { directories: 2, files: 3 }));
        t.set_directory(1, 0, 0, 0);
        t.set_directory(4, 0, 0, 0);
        watch = pass(watch);
        assert_eq!(watch, ReleaseWatch::Unloading, "directories reset, their files not yet freed");
        t.set_data(2, 3, 0, 0);
        assert_eq!(read(), Some(Unloading { directories: 0, files: 3 }), "freed, state not yet 0");
        for f in [2, 3] { t.set_data(f, 0, 0, 0); }
        t.set_data(6, 1, 1, 1);
        assert_eq!(read(), Some(Unloading::default()), "file 6's slot reused by a new load: not this release's");
        watch = pass(watch);
        let ticket = watch.requested().expect("unload done: the purge is requested");
        assert_eq!(owed_purge(), Some(ticket));
        for _ in 0..120 {
            watch = pass(watch);
            assert!(!watch.settled(), "the MainThread hook has not purged yet");
        }
        // The hook (native_body_bank::purge_on_main_thread) runs it and records it; the next pass settles.
        purged(owed_purge().unwrap());
        assert_eq!(owed_purge(), None);
        watch = pass(watch);
        assert!(watch.settled());
        let (_, generation) = waiting().unwrap();
        settled(generation);
        assert_eq!(capture(), Some((0, 0, true)), "nothing queued any more");
        BASE.store(0, Ordering::Release);
        assert_eq!(waiting(), None, "rows not passed: nothing read");
        assert!(ReleaseWatch::Settled.freed().pass(None, purges(), base().is_some()).settled(), "nor any hold");
    }

    fn an_index_outside_the_table_is_not_read() {
        let t = Table { data: 0x1000, data_count: 4, directories: 0x2000, directory_count: 2 };
        assert_eq!(t.directory(1), Some(0x2000 + DIRECTORY_SIZE));
        assert_eq!(t.directory(2), None);
        assert_eq!(t.data(4), None);
        assert_eq!(t.data(u32::MAX), None);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_watch_settles_on_unloaded_then_purged_in_that_order", the_watch_settles_on_unloaded_then_purged_in_that_order as fn()),
            ("an_index_outside_the_table_is_not_read", an_index_outside_the_table_is_not_read as fn()),
        ]);
    }
    }
}
