# Guard tables

One file per table of guarded sites: every place in the 13.0.5 `main` the plugin hooks, patches, calls or relies on,
with the 64 bytes expected there. `build.rs` turns each file into the Rust array its source file includes
(`include!(concat!(env!("OUT_DIR"), "/tables/<name>.rs"))`); the name is `<source file stem>.<TABLE>`.

- Line 1: `#! <Struct>`, then the field names, tab-separated.
- A row: one line, one cell per field. A cell is the field's Rust value as written (an offset, an enum path, a quoted
  string), except `guard`: its 64 bytes as 128 hex digits.
- A line starting with `# ` is a comment for the row below it.

A malformed file fails the build. `src/model/table_digest_tests.rs` pins every row's content and order; a deliberate
change to a table updates the digest there.

To regenerate a table from the owned ELF, run its generator (`tools/gen_*.py`, which writes the table as Rust) into
a scratch file, then `python3 -I tools/rows_tsv.py table <scratch.rs> data/tables/<name>.tsv`.
