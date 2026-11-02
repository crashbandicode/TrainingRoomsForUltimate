# Compliance, package 3 (save states), 2026-11-02

`compliance.py` (playbook tools/check) over `stream/pkg3` at the commit named in the last row, with the shipped NRO.
Machine rows: 24 pass or info, 3 mandatory rows fail and are not package 3's to close: README GIF, build commands and
license line; a GIF under `docs/media`; `HANDOFF.md` (the lead's handoff file is `TRAININGROOMS-HANDOFF.md`).
Agent rows below.

| Requirement | Result | Evidence |
| --- | --- | --- |
| Gates run: test, clippy -D warnings, no-default-features, skyline release, diag build | pass | `cargo test --no-fail-fast` 29 binaries ok on the Surface; `cargo clippy --all-targets -- -D warnings` and `cargo build --no-default-features` clean on the Yoga at 4fdf560 and again at 13be87c (the Yoga was free of Eden claims for both); `cargo skyline build --release` and `--features diag` on the Yoga, `docs` IMPLEMENTATION-PROGRESS-pkg3.md |
| Fixes: root cause fixed, or each workaround approved | pass | Eden s3-03 crash: wrong argument type, fixed (1076ddc); s3-05 `heap_allocations=1`: a `Vec` in the hook, removed (13be87c). No guard, skip or tolerance was added |
| Card scan for the site (and vtable slot) over a dated card copy | N/A | no card work in this package; every site row is compared with the owned 13.0.5 ELF in `tests/state_sites.rs` and with the running game's bytes in the install line (Eden) |
| Hook sharing or chaining; both stand-in modes and load orders in Eden | pass for the rows run | `EffectModule::req_on_joint` is hooked here and called by the CPU pilot code; s3-03 to s3-07 booted with the pilot and the tile cursor and the install line reads `installed`; the stand-in mode is package 2's |
| Hook after the prologue or the function called instead | pass | one inline hook (Pokemon decide store, `main+0x34cec44`, guard row, host test); every other hook is a replace at the function entry |
| Shipped NRO writes only its install line (plus a logging plugin's data files) | pass | the shipped build has no `diag` feature (the checker finds no diag markers); the slot file, setups and the import's copies are the plugin's own data files under `TrainingRooms/`; s3-09 and s3-10 ran the release NRO and `check-migration.py verify` passed (nothing outside the plan was written, the fork folder unchanged) |
| Online rows | N/A | save states do nothing online (Training mode only, `is_training_mode` gate in every hook) |
| Eden: claim written and released, also after a failed run | pass | ledger lines for s3-03 (failed run, released), s3-04 to s3-07 |
| Eden: one instance per host, MemAvailable >= 10 GiB, not profile a or :61 | pass | the harness asserts them (`s3_eden.py up`): own profile `t3`, display :97, MemAvailable about 22 GiB |
| Eden: profile copy with hard-linked game content; only the copy changed | pass | `s3_eden.py prep` (copy of `ssbu-desync-1029/profiles/b`, read-only content hard-linked) |
| Copies | pass | as above |
| Disk | pass | one target dir per source copy (`cargo-targets/trainingrooms-pkg3`, the Yoga build folder, its host-target), `df` 277 GB free before each Yoga build and 40 GB asserted by `up` |
| Eden: waits gated on a log line or screenshot check | pass | `route_s3.py` gates on the install line and reference screenshots; `s3d.py wait` on log lines |
| Eden: failed precondition aborted and retried under the next label | pass | s3-03 crashed; the fix was built and the row re-run as s3-04 |
| Eden: profile restored, temp files removed, host released | pass | `down` after every run; the profile keeps the archived `*.before-s3-NN` folders |
| Eden: console.log searched for panic/abort/crash; run.json and memory.peak kept | pass | `s3_eden.py scan` after every run (s3-03: the crash; s3-04 to s3-07: 0 matches); `run.json`, `memory.peak` in `eden/runs/<label>/` (gitignored) |
| Every local command under the memory and time guard | pass | cargo through `systemd-run MemoryMax=6G timeout 1800` (`/tmp/pkg3-cargo.ps1`), the Yoga builds under `MemoryMax=8G timeout 3000` |
| Evidence doc states run labels, build, what was done, what the log showed | pass | `eden/EDEN-PACKAGE-3.md` |
| GIF caption | N/A | no GIF in this package |
| No operator quotes, no claims beyond the tests | pass | the rows not run (Olimar, Sora, Samus-hat charge, ledge raise) are listed as host-tested or unrun |
| Handoff file current | not package 3's | the lead's `TRAININGROOMS-HANDOFF.md`; this stream's state is `IMPLEMENTATION-PROGRESS-pkg3.md` |
| Release, card | N/A | nothing released, tagged, pushed or written to a card |
| Model/effort gate and watcher | pass | claude-sonnet-5-5 high as spawned by the lead; the queue watcher polls the Yoga ledger |
