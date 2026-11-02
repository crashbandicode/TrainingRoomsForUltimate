//! Save states (package 3, SPEC-state-01 to 05): the pure half. The records and their JSON, the load
//! machine, the placement rules, the settings the per-frame code reads, the portable setup library.
//! Nothing here touches the game; `runtime::features::save_states` applies the machine's answers.

#[path = "buff.rs"]
pub mod buff;
#[path = "charge.rs"]
pub mod charge;
#[path = "convert.rs"]
pub mod convert;
#[path = "file.rs"]
pub mod file;
#[path = "held_item.rs"]
pub mod held_item;
#[path = "hooks.rs"]
pub mod hooks;
#[path = "ids.rs"]
pub mod ids;
#[path = "items.rs"]
pub mod items;
#[path = "json.rs"]
pub mod json;
#[path = "machine.rs"]
pub mod machine;
#[path = "persist.rs"]
pub mod persist;
#[path = "pokemon.rs"]
pub mod pokemon;
#[path = "place.rs"]
pub mod place;
#[path = "record.rs"]
pub mod record;
pub mod quick_reset;
pub mod preview;
#[path = "settings.rs"]
pub mod settings;
#[path = "setup.rs"]
pub mod setup;
#[path = "steve.rs"]
pub mod steve;

/// The ItemManager singleton's pointer slot (`_ZN3lib9SingletonIN3app11ItemManagerEE9instance_E`, in
/// `.bss`), checked in `tests/state_sites.rs` against the guarded `adrp` / `ldr` pair of the
/// `state_item_manager_ref` row.
pub const ITEM_MANAGER_SLOT_OFFSET: usize = 0x52c_4070;
