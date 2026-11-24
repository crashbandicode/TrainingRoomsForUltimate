macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
#[path = "support/playback_engine.rs"]
#[allow(dead_code, unused_imports)]
pub mod engine;
#[path = "trim_playback.rs"]
mod trim_playback;
