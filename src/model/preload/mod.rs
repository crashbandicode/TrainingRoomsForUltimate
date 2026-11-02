//! Package 6 cycle preload, pure half: admission grace and the Ready notice queue.
//!
//! Carried from the fork (stage m5o) by tools/carry_pkg6.py: the files below are generated from the manifest
//! docs/provenance/pkg6-carry-manifest.json and are never edited by hand (CARRY-OVER.md, pkg6 section).

pub mod boot_cache;
pub mod cycle_admission_grace;
pub mod notice_queue;
pub mod cycle_preload_capacity;
pub mod cycle_preload_config;
pub mod cycle_preload_policy;
// Clean-room tests of the carried INI writer (not carried): the reboot sequences of docs/PACKAGE-6-PLAN.md section 7.
#[cfg(test)]
mod ini_transactions_tests;
