//! A small pretty JSON writer with the layout `serde_json::to_string_pretty` gives a derived struct
//! (two-space indent, `"key": value`, one array element per line, `{}` and `[]` when empty), so the
//! files this plugin writes are byte-identical to the fork's for the same values (the carried tests
//! compare them). Fields come out in the order the caller writes them: serde_json's own map sorts keys
//! and the fork's structs write them in declaration order. Floats and strings are formatted by
//! serde_json itself.

use serde_json::Value;

#[derive(Default)]
pub struct Writer {
    out: String,
    /// One entry per open container: whether it has no element yet.
    stack: Vec<bool>,
}

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn finish(self) -> String {
        self.out
    }

    fn indent(&mut self) {
        for _ in 0..self.stack.len() {
            self.out.push_str("  ");
        }
    }

    /// Before an element of the open container: the separator and the indent.
    fn element(&mut self) {
        if let Some(first) = self.stack.last_mut() {
            if *first {
                *first = false;
                self.out.push('\n');
            } else {
                self.out.push_str(",\n");
            }
            self.indent();
        }
    }

    pub fn begin_object(&mut self) {
        self.out.push('{');
        self.stack.push(true);
    }

    pub fn end_object(&mut self) {
        self.close('}');
    }

    pub fn begin_array(&mut self) {
        self.out.push('[');
        self.stack.push(true);
    }

    pub fn end_array(&mut self) {
        self.close(']');
    }

    fn close(&mut self, bracket: char) {
        let empty = self.stack.pop().unwrap_or(true);
        if !empty {
            self.out.push('\n');
            self.indent();
        }
        self.out.push(bracket);
    }

    /// Start an array element that is itself written next (an object or an array).
    pub fn item(&mut self) {
        self.element();
    }

    /// `"key": ` and nothing after it; the caller writes the value (a container or `raw`).
    pub fn key(&mut self, key: &str) {
        self.element();
        self.out.push_str(&Value::String(key.to_owned()).to_string());
        self.out.push_str(": ");
    }

    /// A value written as it is (already valid JSON).
    pub fn raw(&mut self, text: &str) {
        self.out.push_str(text);
    }

    pub fn field_str(&mut self, key: &str, value: &str) {
        self.key(key);
        self.out.push_str(&Value::String(value.to_owned()).to_string());
    }

    pub fn field_int(&mut self, key: &str, value: i64) {
        self.key(key);
        self.out.push_str(&value.to_string());
    }

    pub fn field_bool(&mut self, key: &str, value: bool) {
        self.key(key);
        self.out.push_str(if value { "true" } else { "false" });
    }

    pub fn field_f32(&mut self, key: &str, value: f32) {
        self.key(key);
        self.f32_value(value);
    }

    pub fn field_null(&mut self, key: &str) {
        self.key(key);
        self.out.push_str("null");
    }

    pub fn f32_value(&mut self, value: f32) {
        // serde_json writes a non-finite float as `null`.
        self.out.push_str(&serde_json::to_string(&value).unwrap_or_else(|_| "null".to_owned()));
    }

    /// A generic value, pretty-printed at the current depth (keys in serde_json's map order).
    pub fn value(&mut self, value: &Value) {
        match value {
            Value::Object(map) => {
                self.begin_object();
                for (key, inner) in map {
                    self.key(key);
                    self.value(inner);
                }
                self.end_object();
            }
            Value::Array(items) => {
                self.begin_array();
                for inner in items {
                    self.item();
                    self.value(inner);
                }
                self.end_array();
            }
            scalar => self.out.push_str(&scalar.to_string()),
        }
    }
}
