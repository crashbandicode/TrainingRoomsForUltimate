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

