//! CPU behaviour (package 2, SPEC-cpu-01 … 07): the pure rules of what the practice CPU does, with
//! the state each rule keeps per fighter body. Nothing here reads or writes the game; the runtime
//! gathers facts, calls `state::Cpu`, and applies the answer (see `state` for the contract).
//!
//! Shared by every CPU spec: `hash` and `ids` (the game's constant table by name), `rng` (the
//! plugin's own generator, never the game's), `settings` (the rows as one `Copy` value and the picks),
//! `role` (decision B4: the CPU role only; decision B6: one state per body), `counter` (the frame
//! counters), `playback` (decisions B5 / N4), `status` (the situation predicates), `mash` (the queue
//! and its buffer entry), `facts` (what the runtime reads and applies). The features follow, one
//! module per spec as it is built: `defence` (SPEC-cpu-02), `ledge` (SPEC-cpu-03), `tech` (SPEC-cpu-04), `shield` (SPEC-cpu-05).

pub mod advantage;
pub mod command;
pub mod counter;
pub mod defence;
pub mod facts;
pub mod hash;
pub mod ids;
pub mod ledge;
pub mod mash;
pub mod playback;
pub mod rng;
pub mod role;
pub mod settings;
pub mod shield;
pub mod state;
pub mod status;
pub mod tech;
pub mod throw;
