use std::fs;
use std::io::ErrorKind;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).expect("system clock before UNIX epoch").as_nanos();
        let parent = std::env::temp_dir();
        Self::create_with(|counter| parent.join(format!("{prefix}-{}-{nanos}-{counter}", std::process::id())))
    }

    fn create_with(mut candidate: impl FnMut(u64) -> PathBuf) -> Self {
        let mut last = PathBuf::new();
        for _ in 0..64 {
            let path = candidate(NEXT.fetch_add(1, Ordering::Relaxed));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => last = path,
                Err(error) => panic!("cannot create {}: {error}", path.display()),
            }
        }
        panic!("scratch directory collision limit at {}", last.display());
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Deref for Scratch {
    type Target = Path;

    fn deref(&self) -> &Path {
        self.path()
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

impl From<&Scratch> for PathBuf {
    fn from(scratch: &Scratch) -> Self {
        scratch.path().to_path_buf()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_directories() {
        let first = Scratch::new("tr-scratch-distinct");
        let second = Scratch::new("tr-scratch-distinct");
        assert_ne!(first.path(), second.path());
        assert!(first.is_dir() && AsRef::<Path>::as_ref(&second).is_dir());
    }

    #[test]
    fn removed_on_drop() {
        let dir = Scratch::new("tr-scratch-drop");
        let path = dir.path().to_path_buf();
        assert!(path.is_dir());
        drop(dir);
        assert!(!path.exists());
    }

    #[test]
    fn removed_on_panic() {
        let parent = Scratch::new("tr-scratch-panic-parent");
        let path = parent.join("child");
        let target = path.clone();
        let result = std::panic::catch_unwind(|| {
            let _dir = Scratch::create_with(|_| target.clone());
            assert!(target.is_dir());
            panic!("test unwind");
        });
        assert!(result.is_err());
        assert!(!path.exists());
    }

    #[test]
    fn skips_existing_candidate() {
        let parent = Scratch::new("tr-scratch-collision-parent");
        let existing = parent.join("existing");
        let available = parent.join("available");
        fs::create_dir(&existing).unwrap();
        let mut attempts = 0;
        let dir = Scratch::create_with(|_| {
            attempts += 1;
            if attempts == 1 { existing.clone() } else { available.clone() }
        });
        assert_eq!(attempts, 2);
        assert_eq!(dir.path(), available);
        assert!(existing.is_dir() && dir.is_dir());
    }
}
