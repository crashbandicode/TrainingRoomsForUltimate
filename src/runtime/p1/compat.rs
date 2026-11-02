//! What `cpu_pilot.rs` still takes from areas that are carried but not declared yet (written for this repository, not carried).
//! Everything else it names is answered by the shared `runtime::compat` (worker B's providers over the merged packages' APIs).
//! Both modules disappear with the joint declaration commit: `training` becomes the lobby facade and `character_roster` the roster.

pub mod training {
    use crate::model::input_delay::Owners;
    use crate::runtime::compat::sync::Readable;
    use crate::runtime::features::recorder;
    use crate::runtime::published;

    pub fn recording_or_playback() -> bool {
        recorder::recording_or_playback()
    }

    pub mod ui {
        pub mod menu {
            use super::super::{published, Owners, Readable};

            /// The native (vanilla) pause menu is open (`Owners::VANILLA_PAUSE`).
            pub struct VanillaPause;

            impl Readable for VanillaPause {
                type Value = bool;
                fn read_value(&self) -> bool {
                    published::input_owners().contains(Owners::VANILLA_PAUSE)
                }
            }

            pub static VANILLA_MENU_ACTIVE: VanillaPause = VanillaPause;
        }
    }
}

pub mod character_roster {
    /// A hot swap is replacing a fighter (`published::HOT_SWAP_IN_PROGRESS`).
    pub fn busy() -> bool {
        crate::runtime::published::HOT_SWAP_IN_PROGRESS.load(std::sync::atomic::Ordering::Acquire)
    }
}
