//! Save states (package 3, SPEC-state-01 to 05): the pure half. The records and their JSON, the load
//! machine, the placement rules, the settings the per-frame code reads, the portable setup library.
//! Nothing here touches the game; `runtime::features::save_states` applies the machine's answers.

#[path = "file.rs"]
pub mod file;
#[path = "held_item.rs"]
pub mod held_item;
#[path = "hooks.rs"]
pub mod hooks;
#[path = "json.rs"]
pub mod json;
#[path = "machine.rs"]
pub mod machine;
#[path = "persist.rs"]
pub mod persist;
#[path = "place.rs"]
pub mod place;
#[path = "record.rs"]
pub mod record;
#[path = "settings.rs"]
pub mod settings;
