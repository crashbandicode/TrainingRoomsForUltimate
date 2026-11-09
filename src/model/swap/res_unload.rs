//! Operator option C (inbox 2026-11-09 17:57Z; primary 19:47Z): when a freed group's memory is really back. Pure: the
//! layout and the rules; `runtime::roster::res_unload` reads the game and makes the one call. The facts are
//! LANE-OPUS-RC6-PRELOAD-PRIORITY-REPORT.md F6 (owned 13.0.5 `main`, base 0; rows `native_seams::rows_res_unload`;
//! Eden `pp-re-trace-1` and `pp-re-trace-2`):
//! - A free only queues. The bodies' destruction (or a prefetched group's key release) drops each directory's and
//!   file's refs (`0x3543330`, `0x3540b10`); at <= 0 it is queued for the service's unload pass.
//! - `*(main+0x5332f20)` is the service's table `fs`: directories at `fs+0x40` (0x48 bytes: refs i32 @4, flags u8 @8,
//!   state u8 @9; count `fs+0x48`), file data at `fs+0x10` (0x18 bytes: refs i32 @8, used u8 @0xc, state u8 @0xd;
//!   count `fs+0x1c`). Both tables live for the process.
//! - The unload pass `0x353f760` runs on the service's update thread every 16 ms while nothing inflates. It releases a
//!   queued directory's files, then resets it (flags cleared, then `stlrb state = 0` last, `0x353faa8`); it frees each
//!   queued file, then resets it (`stlrb state = 0` after the free, `0x35402c0`). A free's unload ends within 0.5 s.
//! - The freed memory reaches the kernel's charge only when jemalloc (`malloc_conf "percpu_arena:percpu,
//!   dirty_decay_ms:1"`) purges it: when the decay ticker of the arena that owns it fires, about every 1,000 allocator
//!   operations in that per-CPU arena (Eden: 1.1 s and 2.9 s after `[Samus, Yoshi]`'s free). Nothing marks it.
//! - `mallctl("arena.4096.purge", 0, 0, 0, 0)` (`main+0x392ec40`; handler `0x394c640` -> `arena_i_decay(all)`
//!   `0x394d940`, 4096 = every arena) returns every purgeable page at once.
//!
//! So the release is finished when what the free queued is unloaded and a purge has run after that: then the
//! committed charge is the room. A free's capture is what is queued just after it: directories in use and unreferenced,
//! files used and unreferenced. Files the game releases later (Eden: 94 files 0.49 s after the free) are a later
//! release; the purge returns whatever was unloaded by then.

/// `main+0x5332f20`: the pointer to the service's table.
pub const FS_GLOBAL: usize = 0x5332f20;
pub const FS_DATA: usize = 0x10;
pub const FS_DATA_COUNT: usize = 0x1c;
pub const FS_DIRECTORIES: usize = 0x40;
pub const FS_DIRECTORY_COUNT: usize = 0x48;
pub const DATA_SIZE: usize = 0x18;
pub const DIRECTORY_SIZE: usize = 0x48;
pub const DIRECTORY_REFS: usize = 4;
pub const DIRECTORY_FLAGS: usize = 8;
pub const DIRECTORY_STATE: usize = 9;
pub const DATA_REFS: usize = 8;
pub const DATA_USED: usize = 0xc;
pub const DATA_STATE: usize = 0xd;
/// `mallctl` (`main+0x392ec40`) and the name that purges every arena (`MALLCTL_ARENAS_ALL` = 4096).
pub const MALLCTL: usize = 0x392ec40;
pub const PURGE_ALL: &[u8] = b"arena.4096.purge\0";
/// Table lengths beyond these are not the service's (a torn or foreign read): nothing is scanned.
pub const MAX_TABLE: u32 = 1 << 21;
/// Directories and files a capture keeps. A free queues at most a few hundred at once (Eden: 12 directories and 389
/// files), and the update thread drains the queue while the free runs.
pub const MAX_DIRECTORIES: usize = 256;
pub const MAX_FILES: usize = 4096;

/// One directory as read: `state` (acquire, first), flags and refs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Directory {
    pub state: u8,
    pub flags: u8,
    pub refs: i32,
}

impl Directory {
    /// Still to be unloaded: queued (in use, unreferenced), or its reset has cleared the flags but not yet `state`.
    /// In use and referenced is held by someone else (it stays); not in use with `state` 0 is unloaded.
    pub const fn waiting(self) -> bool {
        let in_use = self.flags & 1 != 0;
        (in_use && self.refs < 1) || (!in_use && self.state != 0)
    }
}

/// One file's data entry as read: `state` (acquire, first), used and refs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct File {
    pub state: u8,
    pub used: u8,
    pub refs: i32,
}

impl File {
    /// Still to be unloaded: queued (used, unreferenced, loaded), or its reset has cleared `used` but not yet `state`
    /// (the free comes before that store). A slot a new load reuses is used and referenced: not this release's.
    pub const fn waiting(self) -> bool {
        (self.used != 0 && self.refs < 1 && self.state != 0) || (self.used == 0 && self.state != 0)
    }
}

/// What a planner pass found of the captured directories and files.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Unloading {
    pub directories: u16,
    pub files: u16,
}

impl Unloading {
    pub const fn waiting(self) -> bool {
        self.directories != 0 || self.files != 0
    }
}

/// What the frees since the watch last settled left queued: directory and data indices. `complete` is false when a
/// bound stopped a scan. Kept in one static, never copied.
pub struct Capture {
    pub directories: [u32; MAX_DIRECTORIES],
    pub directory_count: usize,
    pub files: [u32; MAX_FILES],
    pub file_count: usize,
    pub complete: bool,
}

impl Capture {
    pub const fn new() -> Self {
        Capture { directories: [0; MAX_DIRECTORIES], directory_count: 0, files: [0; MAX_FILES], file_count: 0, complete: true }
    }
    pub fn clear(&mut self) {
        self.directory_count = 0;
        self.file_count = 0;
        self.complete = true;
    }
    pub fn directories(&self) -> &[u32] {
        &self.directories[..self.directory_count]
    }
    pub fn files(&self) -> &[u32] {
        &self.files[..self.file_count]
    }
    pub fn add_directory(&mut self, index: u32) {
        if self.directories().contains(&index) { return; }
        if self.directory_count == MAX_DIRECTORIES { self.complete = false; return; }
        self.directories[self.directory_count] = index;
        self.directory_count += 1;
    }
    /// A data index once per capture is not checked (the scan visits each index once).
    pub fn add_file(&mut self, index: u32) {
        if self.file_count == MAX_FILES { self.complete = false; return; }
        self.files[self.file_count] = index;
        self.file_count += 1;
    }
}

impl Default for Capture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// F6: what the unload pass writes, in its order. A queued directory: in use (flags bit 0), refs 0, state 3. Its
    /// reset clears the flags first and `state` last (`0x353fa90`, `0x353faa8`), so a read between them still waits.
    #[test]
    fn a_directory_waits_from_its_release_until_its_reset_reaches_state() {
        let loaded = Directory { state: 3, flags: 0x1, refs: 1 };
        assert!(!loaded.waiting(), "referenced: stays");
        let queued = Directory { refs: 0, ..loaded };
        assert!(queued.waiting(), "released: queued for the pass");
        let flags_cleared = Directory { flags: 0, ..queued };
        assert!(flags_cleared.waiting(), "the reset is under way: state not yet 0");
        assert!(!Directory { state: 0, ..flags_cleared }.waiting(), "unloaded");
        assert!(!Directory { refs: 1, ..queued }.waiting(), "another holder took it again: it stays");
        assert!(!Directory { state: 0, flags: 0, refs: 0 }.waiting(), "a free slot");
        assert!(Directory { state: 0, flags: 1, refs: 0 }.waiting(), "queued with nothing loaded still waits for the pass");
    }

    /// F6: a released file is queued when used and unreferenced; the pass frees it, then `used = 0`, then `state = 0`
    /// (`0x3540290`, `0x35402bc`, `0x35402c0`). A file another owner references stays; a reused slot is not ours.
    #[test]
    fn a_file_waits_from_its_release_until_its_reset_reaches_state() {
        let held = File { state: 3, used: 1, refs: 1 };
        assert!(!held.waiting(), "referenced: stays");
        let queued = File { refs: 0, ..held };
        assert!(queued.waiting());
        let used_cleared = File { used: 0, ..queued };
        assert!(used_cleared.waiting(), "freed, state not yet 0");
        assert!(!File { state: 0, ..used_cleared }.waiting(), "unloaded");
        assert!(!File { state: 1, used: 1, refs: 1 }.waiting(), "its slot reused by a new load");
        assert!(Unloading { directories: 0, files: 1 }.waiting() && !Unloading::default().waiting());
        assert_eq!(PURGE_ALL.last(), Some(&0), "a C string");
    }

    #[test]
    fn a_capture_keeps_each_directory_once_and_reports_its_bounds() {
        let mut capture = Capture::new();
        capture.add_directory(7);
        capture.add_directory(7);
        assert_eq!(capture.directories(), &[7]);
        for i in 0..MAX_DIRECTORIES as u32 { capture.add_directory(100 + i); }
        assert_eq!(capture.directories().len(), MAX_DIRECTORIES);
        assert!(!capture.complete, "a full capture is incomplete");
        capture.clear();
        assert!(capture.complete && capture.directories().is_empty() && capture.files().is_empty());
    }
}
