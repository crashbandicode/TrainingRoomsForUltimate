//! A whole settings file sent by the test-only hardware bridge (state export `REQ_SET_APPLY`). It is accepted only when it
//! reads with no issue, so a malformed or invalid request changes nothing; boot keeps its own recovery (`parse_bytes`
//! repairs what it can and the caller keeps the bad file).

use super::settings::{self, Loaded};

/// The parsed file, or the number of issues that refuse it.
pub fn accept(bytes: &[u8]) -> Result<Loaded, usize> {
    let loaded = settings::parse_bytes(bytes);
    if loaded.corrupt() { Err(loaded.issues.len()) } else { Ok(loaded) }
}

#[cfg(test)]
mod tests {
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
}
