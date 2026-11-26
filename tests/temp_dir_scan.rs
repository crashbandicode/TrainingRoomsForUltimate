use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            rust_files(&path, files);
        } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

fn calls(source: &str) -> Vec<usize> {
    let code = super::game_thread_handoffs::code_only(source);
    let mut lines = Vec::new();
    for (at, _) in code.match_indices("temp_dir") {
        if at > 0 && (code.as_bytes()[at - 1].is_ascii_alphanumeric() || code.as_bytes()[at - 1] == b'_') {
            continue;
        }
        let start = at + "temp_dir".len();
        let rest = code[start..].trim_start();
        let Some(rest) = rest.strip_prefix('(') else { continue };
        let args = source.len() - rest.len();
        if empty_arguments(&source[args..]) {
            lines.push(code[..at].bytes().filter(|&byte| byte == b'\n').count() + 1);
        }
    }
    lines
}

fn empty_arguments(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at].is_ascii_whitespace() {
            at += 1;
        } else if bytes[at..].starts_with(b"//") {
            at += 2;
            while at < bytes.len() && bytes[at] != b'\n' { at += 1; }
        } else if bytes[at..].starts_with(b"/*") {
            at += 2;
            let mut depth = 1;
            while at < bytes.len() && depth > 0 {
                if bytes[at..].starts_with(b"/*") {
                    depth += 1;
                    at += 2;
                } else if bytes[at..].starts_with(b"*/") {
                    depth -= 1;
                    at += 2;
                } else {
                    at += 1;
                }
            }
        } else {
            return bytes[at] == b')';
        }
    }
    false
}

#[test]
fn no_unmanaged_temp_dirs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    rust_files(&root.join("tests"), &mut files);
    files.sort();
    let mut failures = Vec::new();
    for file in files {
        let relative = file.strip_prefix(root).unwrap();
        if relative == Path::new("tests/support/scratch.rs") {
            continue;
        }
        for line in calls(&fs::read_to_string(&file).unwrap()) {
            failures.push(format!("{}:{line}", relative.display()));
        }
    }
    assert!(failures.is_empty(), "unmanaged temporary directories:\n{}", failures.join("\n"));
}

#[test]
fn scan_distinguishes_calls_from_comments_strings_and_names() {
    let source = r#"
// temp_dir()
/* temp_dir() */
let text = "temp_dir()";
let x = std::env::temp_dir (
);
let y = not_temp_dir();
let z = temp_dir("fixture");
let w = temp_dir(/* comment with "quotes" and /* nesting */ */);
let a = temp_dir( // comment
);
"#;
    assert_eq!(calls(source), [5, 9, 10]);
}
