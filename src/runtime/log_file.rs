//! Two-file rotation shared by diagnostics and sparse install/failure events.
use crate::model::storage;
use std::fmt::{self, Write as _};
use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::sync::Mutex;

// No path back to diagnostics: callers ignore an events error rather than reporting it recursively.
static EVENTS: Mutex<()> = Mutex::new(());

/// The existing DIAG rotation: replace the previous generation, then create an empty current file.
/// A legacy oversized current file is retained whole as previous, not silently truncated.
pub(crate) fn rotate_log(current: &str, previous: &str) -> io::Result<()> {
    if fs::metadata(current).is_ok() {
        match fs::remove_file(previous) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        fs::rename(current, previous)?;
    }
    OpenOptions::new().create(true).append(true).open(current)?;
    Ok(())
}

struct EventBytes(usize);
impl fmt::Write for EventBytes {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0 = self.0.saturating_add(text.len());
        Ok(())
    }
}

/// Serialize both event writers across metadata, rotation and append. Formatting includes one newline.
/// Only whole lines fitting the existing DIAG cap are admitted; failures do not recurse into logging.
pub(crate) fn append_event_at(
    current: &str,
    args: fmt::Arguments<'_>,
    sync: bool,
) -> io::Result<()> {
    let mut size = EventBytes(0);
    writeln!(size, "{args}").map_err(|_| io::Error::other("event format failed"))?;
    if size.0 as u64 > storage::DIAGNOSTIC_LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "event line exceeds file cap",
        ));
    }
    let _guard = EVENTS.lock().unwrap_or_else(|poison| poison.into_inner());
    let bytes = match fs::metadata(current) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
        Err(error) => return Err(error),
    };
    if storage::should_rotate(bytes, size.0) {
        let mut previous = storage::FixedPath::<256>::new();
        write!(
            previous,
            "{}.previous.log",
            current.strip_suffix(".log").unwrap_or(current)
        )
        .map_err(|_| io::Error::other("event previous path overflow"))?;
        rotate_log(current, previous.as_str())?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(current)?;
    writeln!(file, "{args}")?;
    if sync {
        file.sync_data()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn paths(name: &str) -> (std::path::PathBuf, String, String) {
        let root = std::env::temp_dir().join(format!("tr-events-{}-{name}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let current = root.join("events.log").to_str().unwrap().to_owned();
        let previous = root
            .join("events.previous.log")
            .to_str()
            .unwrap()
            .to_owned();
        (root, current, previous)
    }

    fn events_rotate_after_exact_cap_and_keep_newest_lines() {
        let (root, current, previous) = paths("boundary");
        let cap = storage::DIAGNOSTIC_LIMIT as usize;
        let first = "a".repeat(cap - 2);
        append_event_at(&current, format_args!("{first}"), true).unwrap();
        append_event_at(&current, format_args!(""), false).unwrap();
        assert_eq!(
            fs::metadata(&current).unwrap().len(),
            storage::DIAGNOSTIC_LIMIT
        );
        assert!(
            !std::path::Path::new(&previous).exists(),
            "exact cap does not rotate"
        );
        append_event_at(&current, format_args!("newest install"), true).unwrap();
        assert_eq!(
            fs::read_to_string(&previous).unwrap(),
            format!("{first}\n\n")
        );
        assert_eq!(fs::read_to_string(&current).unwrap(), "newest install\n");
        let second = "b".repeat(cap - 1);
        append_event_at(&current, format_args!("{second}"), false).unwrap();
        assert_eq!(fs::read_to_string(&previous).unwrap(), "newest install\n");
        append_event_at(&current, format_args!("42 diag_log write: full"), false).unwrap();
        assert_eq!(
            fs::read_to_string(&previous).unwrap(),
            format!("{second}\n")
        );
        assert_eq!(
            fs::read_to_string(&current).unwrap(),
            "42 diag_log write: full\n"
        );
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
        fs::remove_dir_all(root).unwrap();
    }

    fn event_writers_share_bounded_append() {
        let install = include_str!("install.rs");
        let diag = include_str!("diag_log.rs");
        assert!(install.contains(
            "log_file::append_event_at(storage::EVENTS_LOG_PATH, format_args!(\"{text}\"), true)"
        ));
        assert!(diag.contains("log_file::append_event_at(failures, format_args!(\"{} diag_log {what}: {error}\", clock::uptime_ms()), false)"));
        assert!(diag.contains("log_file::rotate_log(&log.current, &log.previous)?;"));
        assert!(
            crate::install_capacity::INSTALL_LINE_CAPACITY + 1
                <= storage::DIAGNOSTIC_LIMIT as usize
        );
        let (root, current, _) = paths("format");
        append_event_at(
            &current,
            format_args!("123 plugin=training_rooms/rc15 result=installed"),
            true,
        )
        .unwrap();
        let error = io::Error::other("échec");
        append_event_at(
            &current,
            format_args!("{} diag_log {}: {}", 124, "write", error),
            false,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&current).unwrap(),
            "123 plugin=training_rooms/rc15 result=installed\n124 diag_log write: échec\n"
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn concurrent_events_remain_whole_and_bounded() {
        let (root, current, previous) = paths("parallel");
        std::thread::scope(|scope| {
            for writer in 0..4 {
                let path = &current;
                scope.spawn(move || {
                    for line in 0..4000 {
                        append_event_at(
                            path,
                            format_args!("writer={writer} line={line:04}"),
                            false,
                        )
                        .unwrap();
                    }
                });
            }
        });
        for path in [&current, &previous] {
            assert!(fs::metadata(path).unwrap().len() <= storage::DIAGNOSTIC_LIMIT);
            let text = fs::read_to_string(path).unwrap();
            assert!(text.ends_with('\n'));
            for line in text.lines() {
                assert_eq!(line.len(), 18);
                assert!(line.starts_with("writer="));
                assert!(line.contains(" line="));
            }
        }
        // Ordering between writers is intentionally unconstrained; only the joined final append is newest.
        append_event_at(&current, format_args!("joined newest"), false).unwrap();
        assert!(fs::read_to_string(&current)
            .unwrap()
            .ends_with("joined newest\n"));
        fs::remove_dir_all(root).unwrap();
    }

    fn rotation_failure_does_not_append_or_recurse() {
        let (root, current, previous) = paths("failure");
        let full = vec![b'x'; storage::DIAGNOSTIC_LIMIT as usize];
        fs::write(&current, &full).unwrap();
        fs::create_dir(&previous).unwrap();
        assert!(append_event_at(&current, format_args!("new"), false).is_err());
        assert_eq!(fs::read(&current).unwrap(), full);
        fs::remove_dir(&previous).unwrap();
        let oversized = "z".repeat(storage::DIAGNOSTIC_LIMIT as usize);
        assert_eq!(
            append_event_at(&current, format_args!("{oversized}"), false)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(fs::read(&current).unwrap(), full);
        assert!(!std::path::Path::new(&previous).exists());
        fs::remove_dir_all(root).unwrap();
    }

    fn legacy_oversized_generation_is_not_truncated() {
        let (root, current, previous) = paths("legacy");
        let legacy = vec![b'l'; storage::DIAGNOSTIC_LIMIT as usize + 17];
        fs::write(&current, &legacy).unwrap();
        append_event_at(&current, format_args!("new boot"), true).unwrap();
        assert_eq!(fs::read(&previous).unwrap(), legacy);
        assert_eq!(fs::read_to_string(&current).unwrap(), "new boot\n");
        let full = "n".repeat(storage::DIAGNOSTIC_LIMIT as usize - 1);
        append_event_at(&current, format_args!("{full}"), false).unwrap();
        assert_eq!(fs::read_to_string(&previous).unwrap(), "new boot\n");
        assert!(fs::metadata(&current).unwrap().len() <= storage::DIAGNOSTIC_LIMIT);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn case_table() {
        crate::test_table::run(&[
            (
                "events_rotate_after_exact_cap_and_keep_newest_lines",
                events_rotate_after_exact_cap_and_keep_newest_lines as fn(),
            ),
            (
                "event_writers_share_bounded_append",
                event_writers_share_bounded_append as fn(),
            ),
            (
                "concurrent_events_remain_whole_and_bounded",
                concurrent_events_remain_whole_and_bounded as fn(),
            ),
            (
                "rotation_failure_does_not_append_or_recurse",
                rotation_failure_does_not_append_or_recurse as fn(),
            ),
            (
                "legacy_oversized_generation_is_not_truncated",
                legacy_oversized_generation_is_not_truncated as fn(),
            ),
        ]);
    }
}
