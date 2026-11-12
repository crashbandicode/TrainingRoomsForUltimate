//! Pure contracts of the C116.1 shared-core refactor (`D175/refactor/WORKPLAN.md` §2.2): the
//! capability table per fighter form, the transition [`Target`] and the predicates derived from
//! the table. No pointers, no native calls, no locks; host-tested. The native adapter
//! (`src/native_body_bank.rs`) and the Mii adapters keep calling the `body_bank` names, which
//! re-export these functions with their exact signatures.
//!
//! Later work packages add their own submodules here (`ledger`, `native_model`, `lifecycle`,
//! `occupancy`, `presentation`, `scope`); each documents the FACT every rule encodes, with its
//! citation (RCA section, listing, Eden row), so the module reads on its own.

pub mod capabilities;
// U18: the Kirby copy registry model and the writer-side protocol for Phase B groups.
pub mod copy_registry;
// U19: Phase B's lease on `R+0x4c` bit 0 and the G68 copy keys the load completion takes.
pub mod request_bit;
// K15 root fix: the Kirby copy packages the bank's registrations need, held and released exactly.
pub mod copy_packages;
// U20: what a swap fallback does to the requested group (transient keeps, permanent returns its keys).
pub mod fallback;
// PT-TRAINER-PARK: a PT holder's article pool (the trainer) lives only while its window is attached.
pub mod special_articles;
pub mod target;
#[cfg(test)]
mod tests;
// WP2 (worker B): the per-R ledger as data and the independent native simulator.
pub mod ledger;
#[cfg(any(test, feature = "simulator"))]
pub mod native_model;
// IC-PAIR-STALL: Aegis → PT → IC pair on one R, ledger and native agreeing (tests only).
#[cfg(test)]
mod pair_after_objects;
// LEDGER-SCOPE: the R-ICP3/R-ICP4 reconcile observations replayed with the logged map counts (tests only).
#[cfg(test)]
mod order_replay;
// WP3 (worker C): the teardown protocol (one destroy plan for every destroy path).
pub mod lifecycle;
// WP4: live-consumer occupancy and the ledger-driven policy.
pub mod occupancy;
// WP-E: scope and read-only contract per callback.
pub mod scope;
// WP-P (worker E): presentation intent from the transition parts and the HUD model (H1-H6).
pub mod hud_model;
pub mod presentation;

pub use capabilities::*;
pub use target::*;
