//! Run the production mailbox's ordering/concurrency/scratch regressions in the
//! root host gate, not only when the dependency crate is tested separately.
#[path = "../crates/hos-lock/src/mailbox.rs"]
mod mailbox;
