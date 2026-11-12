//! JSON written straight into a formatter (written for this repository, not carried). The carried CPU-as-P1 fallback logged its
//! admission gates and its first pair by building a `serde_json::Value` inside the fighter-frame hook, a heap allocation per line
//! (Eden `rc6-int-smoke1`: `hook_stats allocs=cpu_frame:…`). These write the same fields through `serde_json::to_writer`, which
//! formats numbers on the stack and escapes strings in place, so a hook logs a typed value without the heap.

use std::fmt;
use std::io;

use serde::ser::{Serialize, SerializeMap, Serializer};

/// `value` as compact JSON inside a `format_args!` line. Nothing is allocated: the sink never fails, so serde_json builds no
/// error value; when the formatter refuses bytes (a full line) the rest is walked and dropped, and the error is the formatter's.
pub struct Json<T>(pub T);

impl<T: Serialize> fmt::Display for Json<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sink = Sink { f, refused: false };
        let _ = serde_json::to_writer(&mut sink, &self.0);
        if sink.refused {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

struct Sink<'a, 'b> {
    f: &'a mut fmt::Formatter<'b>,
    refused: bool,
}

impl io::Write for Sink<'_, '_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // serde_json writes whole UTF-8 runs: punctuation, digits, and string text cut only at the ASCII bytes it escapes.
        let text = std::str::from_utf8(bytes).unwrap_or("\u{fffd}");
        if !self.refused && self.f.write_str(text).is_err() {
            self.refused = true;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A JSON object of `(name, value)` pairs in the order given: `Obj((("binding", binding), ("fim_mask", mask)))`.
pub struct Obj<T>(pub T);

macro_rules! obj {
    ($n:literal; $($v:ident $k:ident),+) => {
        impl<$($v: Serialize),+> Serialize for Obj<($((&'static str, $v),)+)> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let ($($k,)+) = &self.0;
                let mut map = serializer.serialize_map(Some($n))?;
                $(map.serialize_entry($k.0, &$k.1)?;)+
                map.end()
            }
        }
    };
}
obj!(1; A a);
obj!(2; A a, B b);
obj!(3; A a, B b, C c);
obj!(4; A a, B b, C c, D d);
obj!(5; A a, B b, C c, D d, E e);
obj!(6; A a, B b, C c, D d, E e, F f);
obj!(7; A a, B b, C c, D d, E e, F f, G g);
obj!(8; A a, B b, C c, D d, E e, F f, G g, H h);
obj!(16; A a, B b, C c, D d, E e, F f, G g, H h, I i, J j, K k, L l, M m, N n, O o, P p);

/// A value's `Debug` text as a JSON string. serde_json's `collect_str` escapes it while it is formatted (no `String`).
pub struct DebugStr<T>(pub T);

impl<T: fmt::Debug> Serialize for DebugStr<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format_args!("{:?}", self.0))
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use serde_json::json;

    #[derive(serde::Serialize)]
    struct Pair {
        ai: usize,
        local_entry: i32,
        name: &'static str,
    }

    fn an_object_reads_as_the_json_macro_wrote_it() {
        let pair = Pair { ai: 77_040_471_552, local_entry: 0, name: "Mii \"Gunner\"\n" };
        let line = format!("{}", Json(Obj((("detail", Obj((("binding", &pair), ("fim_mask", 0u8)))), ("gate", "admission_ready")))));
        let old = json!({"gate": "admission_ready", "detail": {"binding": {"ai": 77_040_471_552usize, "local_entry": 0, "name": "Mii \"Gunner\"\n"}, "fim_mask": 0}});
        assert_eq!(line, old.to_string());
    }

    fn options_floats_and_debug_text_keep_the_values_format() {
        #[derive(Debug)]
        enum Status {
            Pending,
        }
        let line = format!("{}", Json(Obj((("a", None::<u8>), ("b", Some(1.5f32)), ("c", DebugStr(Status::Pending)), ("d", [1u64, 2])))));
        assert_eq!(line, json!({"a": null, "b": 1.5, "c": "Pending", "d": [1, 2]}).to_string());
        assert_eq!(format!("{}", Json(DebugStr(Some("x\u{e9}")))), json!(format!("{:?}", Some("x\u{e9}"))).to_string());
    }

    fn a_full_line_reports_the_formatter_error_and_stops_writing() {
        struct Short(String);
        impl fmt::Write for Short {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                if self.0.len() + s.len() > 8 {
                    return Err(fmt::Error);
                }
                self.0.push_str(s);
                Ok(())
            }
        }
        let mut short = Short(String::new());
        assert!(fmt::write(&mut short, format_args!("{}", Json(Obj((("gate", "admission_ready"),))))).is_err());
        assert_eq!(short.0, "{\"gate\":");
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("an_object_reads_as_the_json_macro_wrote_it", an_object_reads_as_the_json_macro_wrote_it as fn()),
            ("options_floats_and_debug_text_keep_the_values_format", options_floats_and_debug_text_keep_the_values_format as fn()),
            ("a_full_line_reports_the_formatter_error_and_stops_writing", a_full_line_reports_the_formatter_error_and_stops_writing as fn()),
        ]);
    }
    }
}
