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

#[test]
fn case_table() {
    crate::test_table::run(&[(
        "a_malformed_or_invalid_settings_request_is_refused",
        a_malformed_or_invalid_settings_request_is_refused as fn(),
    )]);
}
}
