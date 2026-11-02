# TrainingRooms Eden on the Ally (dreamland-ally), 2026-11-02

Written by stream pkg2 (claude-sonnet-5-5, lead `0caa72b8`) after setting the rig up and running s2-01..s2-04 on it. Other
streams reuse it: copy the root's scripts, never write into another owner's root. All commands from the Surface go through the
MSI PowerShell 7 (`C:\Program Files\PowerShell\7\pwsh.exe`, EncodedCommand; never cmd.exe or Windows PowerShell 5.1);
nothing needs `$` inside a remote double-quoted string because every snippet is a single-quoted here-string.

## What is where
| Thing | Path |
| --- | --- |
| Owned root | `C:\Eden\trainingrooms-pkg2` (this stream's; another stream makes `C:\Eden\<stream>` the same way) |
| Portable Eden + profile | `<root>\profile\eden.exe` (sha256 `45af318c…`, the build the other Ally rigs pin), `<root>\profile\user` |
| Runner modules | `<root>\runner` (copy of the Windows port's `winplat/winjob/winlaunch/winaccept/winlimits`) |
| Adapter | `<root>\wa_s2.py` (`launch`, `ops`, `kill`; source `eden/ally/wa_s2.py`) |
| Claim file | `<root>\CLAIM` (the adapter refuses to launch without its owner text) |
| Plugin sets | `<root>\nro\release`, `<root>\nro\diag` (providers `libnro_hook`, `libparam_hook`, the CPU pilot, the tile cursor, ours; `sha256.txt` in each) |
| Macros, settings | `<root>\macros`, `<root>\settings` |
| Runs | `<root>\runs\live-<label>\` (`console.log`, `run.json`, `ops.jsonl`, shots); the plugin's own logs are in `<root>\profile\user\sdmc\ultimate\<plugin>\` |
| Windows build/dev tools | Ally WSL (Ubuntu): campaign toolchain `/home/intpa/smash-training-lobby/toolchain/tools` |

## 1. The owned root and profile (once): `eden/ally/ally-setup.ps1`
`scp` the script to the Ally and run it with MSI pwsh. It refuses while `eden.exe` runs or if the root exists. It copies the Eden
files (hash-checked), the runner modules, small profile trees (config, keys, cache, saves) from the read-only source rig
`C:\Eden\c1162-w5-ally`, hard-links `user\nand\user\Contents` (3.7 GB) and `user\nand\system` (NTFS, byte-identical, never
written), builds a fresh `sdmc` (Skyline's stock `subsdk9` and `main.npdm`, hash-checked, an empty plugins folder), rewrites the
qt-config to this root and **mutes it** (`volume=0`, `volume\default=false`, persisted), sets airplane mode, no LAN, no pause in
the background, and writes `CLAIM`. First launch of a new `eden.exe` path raises a Windows Firewall prompt (it does not block the
emulator or the screenshot path through Eden's own recorder; `dismiss-fw.ps1` of the m5q rig, copied as `eden/ally/dismiss-fw.ps1`,
presses Cancel but found no window from a scheduled task on 2026-11-02, so the prompt stayed up; nothing allowed it).

## 2. Claim (the Yoga ledger is the one ledger)
`s2_eden.ledger('CLAIMED: …')` / `…RELEASED…` (it appends over ssh to `/tmp/deluxe-surface-1007/OWNER-COORDINATION.md` on the Yoga).
Check the Ally is free first: `ally_ops.py ps '(Get-Process eden -ErrorAction SilentlyContinue | Measure-Object).Count'` and the queue
file's Ally section. One Eden at a time, and never compile in the Ally's WSL while `eden.exe` runs.

## 3. Build in the Ally's WSL: `eden/ally/ally-build.sh`
Pack the worktree (`tar --exclude .git --exclude target --exclude eden/runs --exclude dist`), `scp` the tarball, `src.sha256`,
`ally-build.sh` and `lua2cpp_common.nro` (research copy; `tests/cpu_common.rs` reads it) to `C:\Users\intpa\<stream>-pack\`.
`wsl -e bash ally-build.sh unpack <pack dir> <worktree name>` unpacks at the Surface's absolute path
(`/home/intpa/smash-training-lobby/<worktree name>`, so `../research/game-13.0.5/main-13.0.5.elf` resolves). Run the gates as a
scheduled task (a bare `wsl` child dies with the ssh session): `schtasks /create /tn trp2-gates /sc once /st 00:00 /tr "wsl.exe -e bash
<script> gates <worktree> <out dir>" /f` then `schtasks /run /tn trp2-gates`; `<out>\GATES.txt` lists each exit. The script runs
`cargo test --no-fail-fast`, `clippy --all-targets -- -D warnings`, `build --no-default-features`, `cargo skyline build --release` and
`--features diag` with the campaign toolchains (host 1.98.1; Switch: nightly-2026-02-14 with `skyline-v3`, `rustup` from
`/home/intpa/.cargo/bin` on PATH) and its own target dir per worktree. Notes: the Ally's cargo registry lacked `libc 0.2.191`, so the script
leaves the network on; WSL has no Pillow, so the one Python-script test is skipped by its own documented switch
(`SKIP_EDEN_SCRIPT_TESTS=1`, printed as SKIPPED; the Surface runs it). `eden/ally/ally_fetch.py <tag>` copies the NROs and logs
back, hashes and sizes them, builds the two plugin sets and pushes them to `<root>\nro\{release,diag}`.

## 4. Stage, launch, drive, stop
* `ally-stage.ps1 -Label s2-NN -Set <root>\nro\diag [-Settings <file>]` (on the Ally): checks `CLAIM`, no `eden.exe`, every NRO against
  `sha256.txt`; archives the previous plugins and plugin output folders under `runs\s2-NN-prev`; copies the settings into the plugin folder.
* `ally_launch.py s2-NN` (Surface): a scheduled task runs `wa_s2.py launch live-s2-NN` in the interactive session (a console Python there
  would open a window); prints the run state. Memory gate: more than 10 GiB available (the WSL VM holds memory; build first, run Eden after).
* `ally_ops.py ops live-s2-NN '[["mute"],["load"],…]'`: one ssh per batch. Ops: `mute` (core.volume 0, save, get; **before** `load`), `load`,
  `status`, `waitstatus`, `button`, `hold`, `stick`, `macro`, `gshot` (Eden's recorder; use this for screenshots, the desktop capture of
  `shot` shows whatever window is on top, here the firewall prompt), `plug`/`waitplug` (the plugin's own logs), `settings` (the diag
  settings reload), `count`, `mark`, `waitlog`. `ally_route.py live-s2-NN` walks title -> main menu (screen-gated against
  `eden/refs/ally-main-menu.png`) -> Training stage select -> Final Destination (tile 4) -> CSS (P1 Steve, tile 78) -> Training, every wait gated
  on the tile cursor's or the plugin's log; the CPU is a random fighter. `ally_log.py s2-NN [regex]` prints the plugin's new diag lines;
  `ally_rows_mash.py s2-NN` is the table-driven mash row.
* Stop: `ally_ops.py ps "& 'C:\Program Files\Python312\python.exe' C:\Eden\trainingrooms-pkg2\wa_s2.py kill live-s2-NN"` (Eden hangs at power-off
  on this rig; the Job kill is the normal ending, `run.json` then says `reason: crashed`, `exit_code: 137`), `schtasks /delete`, then
  scan `console.log` (`panic|abort|crash|Unmapped`) and `scp` the run folder and the plugin folders next to the repo (`eden/runs/<label>/`, gitignored).

## Timings and limits seen
Boot to the title with the game loaded: about 15 s; route to Training about 2 minutes; Eden 52-60 fps in Training, peak 5.5 GB in the Job
(limit 8 GiB); one `ops` batch costs 2-3 s of ssh. The diag ring keeps two 128 KiB files (`practice-diagnostics.log`, `.previous.log`): a long
session rotates, so read it while it runs (`ally_log.py`) and copy both files at the end.
