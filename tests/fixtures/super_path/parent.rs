// rc.8 §6 fixture: what `super` means. `native_service.rs` is a `#[path]` child of `character_roster.rs`, and its
// `super::service` is the parent's.
#[path = "renamed_child.rs"]
mod child;
mod plain;

fn service() {}

mod inline {
    fn caller() {
        super::service();
    }
}
