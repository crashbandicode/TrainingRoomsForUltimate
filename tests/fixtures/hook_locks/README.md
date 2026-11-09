Fixture for `tests/hook_locks.rs`: a minimal runtime tree (never compiled) with one seeded violation per lock-site form.

`compat/menu.rs` and `ui/worker.rs::reads_menu`: a typed static (`read(&MENU)`) whose trait impl takes a hook's lock (`a_typed_static_read_reaches_its_impl`).
