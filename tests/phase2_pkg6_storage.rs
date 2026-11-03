//! Original Q7 persisted-choice boundaries; no native capture acceptance claim.
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::capture_settings::CaptureSettings;

#[test]
fn q7_explicit_menu_choice_repairs_invalid_owned_keys_and_survives_reload() {
    for original in [
        "# retained\n[capture]\nenabled=off\nenabled=on\nkeep=7\n",
        "# retained\n[capture]\nenabled=maybe\nkeep=7\n",
        "# retained\n[capture]\nenabled\nkeep=7\n",
        "# retained\n[capture]\nenabled # note=retained\nkeep=7\n",
        "# retained\n[capture]\nenabled=maybe ; first\n[capture]\nenabled=on # duplicate\nkeep=7\n",
    ] {
        assert!(CaptureSettings::parse(original).is_err());
        for enabled in [true, false] {
            let persisted = CaptureSettings::to_ini_text(original, enabled);
            assert_eq!(CaptureSettings::parse(&persisted), Ok(CaptureSettings { enabled }),
                "an explicit accepted choice must not persist another invalid document");
            assert!(persisted.contains("# retained") && persisted.contains("keep=7"));
            assert_eq!(CaptureSettings::from_bytes(Some(persisted.as_bytes())).0.enabled, enabled);
            assert!(CaptureSettings::persistence_compatible(&persisted).is_ok());
        }
    }
}

#[test]
fn q7_repair_keeps_foreign_sections_and_every_owned_comment() {
    let original = "# retained\n[other]\nenabled=foreign\n[capture]\nenabled=bad ; first\nEnabled # second=note\nenabled=off # third\nkeep=7\n";
    for enabled in [true, false] {
        let text = CaptureSettings::to_ini_text(original, enabled);
        assert_eq!(CaptureSettings::parse(&text), Ok(CaptureSettings { enabled }));
        for line in ["# retained", "[other]\nenabled=foreign", "; first", "# second=note", "# third", "keep=7"] {
            assert!(text.contains(line), "lost {line:?} in {text}");
        }
        assert_eq!(CaptureSettings::to_ini_text(&text, enabled), text, "rewriting a repaired document is stable");
    }
}

#[test]
fn q7_persisted_explicit_choice_continues_in_a_fresh_process() {
    const KEY: &str = "TR_PKG6_Q7_CONTINUATION";
    const TEST: &str = "q7_persisted_explicit_choice_continues_in_a_fresh_process";
    let now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    if let Some(root) = std::env::var_os(KEY) {
        let root = std::path::PathBuf::from(root);
        let path = root.join("replay_capture.ini");
        let mode = std::env::var("TR_PKG6_Q7_MODE").unwrap();
        println!("Q7 child={mode} pid={} start={}", std::process::id(), now());
        let original = std::fs::read_to_string(&path).unwrap();
        if mode == "save" {
            assert!(CaptureSettings::parse(&original).is_err());
            let text = CaptureSettings::to_ini_text(&original, true);
            model::ini_files::write_atomic(&path, text.as_bytes()).unwrap();
            assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
            assert!(CaptureSettings::parse(&text).unwrap().enabled);
            let saved_hash = model::sha256::sha256_hex(text.as_bytes());
            std::fs::write(root.join("saved.sha256"), &saved_hash).unwrap();
            println!("Q7 save enabled=true bytes={} sha256={saved_hash}", text.len());
        } else {
            assert_eq!(mode, "restore");
            assert_eq!(model::sha256::sha256_hex(original.as_bytes()), std::fs::read_to_string(root.join("saved.sha256")).unwrap());
            assert!(CaptureSettings::from_bytes(Some(original.as_bytes())).0.enabled);
            println!("Q7 restore enabled=true bytes={} sha256={}", original.len(), model::sha256::sha256_hex(original.as_bytes()));
            let text = CaptureSettings::to_ini_text(&original, false);
            model::ini_files::write_atomic(&path, text.as_bytes()).unwrap();
            assert!(!CaptureSettings::parse(&std::fs::read_to_string(&path).unwrap()).unwrap().enabled);
            assert!(text.contains("# retained") && text.contains("keep=7"));
            println!("Q7 continued enabled=false sha256={}", model::sha256::sha256_hex(text.as_bytes()));
        }
        return;
    }
    let root = std::env::temp_dir().join(format!("tr-pkg6-q7-{}-{}", std::process::id(), now()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("replay_capture.ini"), b"# retained\n[capture]\nenabled=off\nenabled=on\nkeep=7\n").unwrap();
    let result = || {
        for mode in ["save", "restore"] {
            println!("Q7 launch={mode} at={}", now());
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", TEST, "--nocapture"])
                .env(KEY, &root).env("TR_PKG6_Q7_MODE", mode).output().unwrap();
            println!("{}", String::from_utf8_lossy(&output.stdout));
            println!("Q7 exited={mode} at={} success={}", now(), output.status.success());
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        }
    };
    let outcome = std::panic::catch_unwind(result);
    std::fs::remove_dir_all(root).unwrap();
    if let Err(error) = outcome { std::panic::resume_unwind(error); }
}
