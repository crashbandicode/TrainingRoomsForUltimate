lib_unit_tests! {
use super::accept;

/// Review finding 2: `set eA==` (the byte `x`) used to publish factory settings and answer ok.
fn a_malformed_or_invalid_settings_request_is_refused() {
    assert!(accept(b"x").is_err());
    assert!(accept(b"").is_err());
    assert!(accept(br#"{"menu": 5}"#).is_err());
    let factory = crate::model::settings::parse_bytes(b"x").settings;
    let mut text = factory.to_json();
    assert!(accept(text.as_bytes()).is_ok(), "a whole, valid settings file is accepted");
    text.truncate(text.len() / 2);
    assert!(accept(text.as_bytes()).is_err(), "a cut file is refused");
}

/// Astra 4ac6a3f 1: a restore is complete only when the committed settings read back as the backed-up file.
fn the_live_restore_reads_back_against_the_committed_menu() {
    use super::matches;
    let factory = crate::model::settings::parse_bytes(b"x").settings;
    let text = factory.to_json();
    assert!(matches(text.as_bytes(), Some(&factory.menu)));
    assert!(!matches(text.as_bytes(), None), "nothing committed yet is not a readback");
    assert!(!matches(b"x", Some(&factory.menu)), "a bad file never matches");
    let mut other = factory.clone();
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value["menu"] = serde_json::json!({});
    other.menu = crate::model::settings::parse_bytes(value.to_string().as_bytes()).settings.menu;
    if other.menu != factory.menu {
        assert!(!matches(text.as_bytes(), Some(&other.menu)));
    }
}

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_malformed_or_invalid_settings_request_is_refused", a_malformed_or_invalid_settings_request_is_refused as fn()),
        ("the_live_restore_reads_back_against_the_committed_menu", the_live_restore_reads_back_against_the_committed_menu as fn()),
    ]);
}
}
