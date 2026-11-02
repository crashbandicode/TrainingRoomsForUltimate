# Implementation progress, ui-screens stream (planning copy; moves into the worktree once it exists)

Stream `ui-screens` (claude-sonnet-5-5 / high; transcript `agent-a3ebfebc7cb9b0850.jsonl`, hidden watcher `/tmp/ui-screens-watch/watch.ps1`, pid 766009,
log `/tmp/ui-screens-watch/watch.log`), lead 0caa72b8. Playbook read in full at `ae87443` (README, all docs/, tools/PROVENANCE.md, SKILL.md).
Worktree (not yet created): `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-ui-screens`, branch `stream/ui-screens` from `work/implementation`,
target `cargo-targets/trainingrooms-ui-screens`, Eden labels `ui-NN`.

## Status 2026-11-02 ~09:30Z
- Read: stream rules, delegation routing, INTEGRATION-OPEN, STATUS, MENU-DRAFT, DECISIONS, MENU-PROPOSAL, pkg4/5/6/6b progress, SPEC-core-05, pkg5 EDEN notes, the
  pane table / frame / screens / grids / menu model / worker / tick / view / thumbs / archive, pkg4's named_macros / replay_capture / port / macro_panel / clip_panel,
  pkg6's roster_grid, character_items, lobby facade.
- `work/implementation` is at `e23e5de` (pkg4 `6e7ea3f`, pkg6 `41d046b`, r2fix `1391396`, pkg5 `7c2b916` merged) but its worktree has uncommitted edits
  (sites_pkg6.rs, game.rs, gen_sites_pkg6.py): the integrator is still gating. Lead's go-ahead notice for the worktree not yet received.

## Lead relay (package 6, checked in lobby.rs) - binding for item 3
- The `info_waiting` / `TrModPort` tick calls `lobby::set_ui_ready(true)` while the practice UI is up and `set_ui_ready(false)` when it goes away
  (`lobby::update` also clears it on a scene change).
- `lobby::set_native_ui_busy(bool)` while a native pause or dialog covers the waiting room: the single producer of `Owners::NATIVE_UI_BUSY`.
- The Give-item grid calls `character_items::request` (`unsafe fn request(choice: usize)`, runtime/roster/character_items.rs).
- Package 6 deletes the carried `practice::{role, visualizes, panel_entry, ...}` and `lobby::permits_entry` in favour of package 2's lobby scene + role
  table: do NOT call those.

## What exists (facts from the code, for the design)
- Pure: `model::menu::Menu` (Screen::{List,Checklist,Delay,Slider,Directions,SwapGroups,Overrides,Chords,ChordEntry,Grid,External(Editor)}); `Editor` has every row of
  this brief (`NamedLibrary, SaveSlotToLibrary, EditPlacement, ReplayWindow, PortraitPlayer, PortraitCpu, CharacterCycles, KirbyHats, GiveItem, MiiList(0..2), StageGrid,
  SetupSave, SetupLibrary`); `open_editor` already builds `Screen::Grid` for GiveItem / StageGrid (A/Y do nothing yet) and `Screen::External(editor)` for the rest (B pops).
- `model::frame::Frame` (fixed UTF-16 text) is the one frame type; `model::screens::frame_of` builds it; `model::panes` is the ONE pane table (COUNT panes: menu, 12 rows,
  11 chips, 40 tiles, notice, input log) and `tools/layout-builder` builds `assets/info_training.tra1` from it. Views: List, Chips, Tiles, Delay, Overrides.
- Runtime: `ui::worker` (the thread that owns the Menu; presses arrive from the P1 input hook through a ring), `ui::tick` (UI job thread: draws FRAME when FRAME_SEQ changes),
  `ui::view::Panes::draw`, `ui::thumbs` (stage thumbnails through `ui::texture`), `ui::menu_app` (holder; `compat::menu_app` re-exports it: DONE).
- Layering: `model::` is included by path in the by-path test crates and cannot see `engine`; editors that need the carried types (roster grid, LobbyStageOrder, Mii
  inventory, macro panels) must live in a module that sees `engine` (new `src/editors/...` declared in lib.rs like `grid_matches_catalog`), producing `model::frame::Frame`.
- pkg4 API: `named_macros::{open_library(Option<usize>), export_record_slot(Option<usize>), open_editor(), on_pad(Pad), panel_view() -> Option<macro_panel::View{title,footer,labels,selected}>,
  panel_visible(), service_keyboard() (already on the CPU body frame, hook scope), take_*}` and `replay_capture::{open_editor(), browser_view(), browser_input(Pad), clip_view() -> Option<ClipView>,
  on_pad, on_action(ClipAction), on_input(&Controller) (called from the P1 mapping hook while the clip editor is foreground; neutralises P1), panel_visible(), browser_visible()}`.
  `MacroUi` keeps its own panel state; the system keyboard is requested by `Pad::Confirm` on the naming page (`native_keyboard::request`).
  `port.rs` still answers "unavailable" for `roster::*` (named layer plays on the settled CPU), `save_states::request_preview_restore` (package 3), install line `menu_app=unavailable_until_pkg5_screens`.
- pkg6: roster editors edit kept keys (`player_character`, `cpu_character`, `player_roster`, `cpu_roster`, `player_kirby_hats`, `cpu_kirby_hats`, `lobby_stages`) of the typed
  `TrainingModpackMenu` (`compat::menu`), `RosterGrid` (pure, `engine::swap::roster_grid`: 13 x 7, 87 cells, role switch, forms, `confirm/remove`); the roster cycle reacts to the
  committed typed view (`character_roster::settings_changed`). Mii lists: `engine::mii_root::mii_menu::{CatalogMenu, View, ...}`, `compat::menu_app::App::take_catalog`,
  `mii_saved_alpha::{menu_open, menu_ready, menu_epoch, menu_close}`, `mii_catalog_request::request`, `native_keyboard`.
- B3 leftover: `ui/archive.rs::read_vanilla` uses `File::open(VANILLA_PATH)` which allocates (Switch std path) -> `load_allocations=1` (Eden s5-23). Fix: direct `nn::fs::OpenFile` /
  `ReadFile1` / `CloseFile` on a NUL-terminated stack/static path (prior art: `roster/practice_trace.rs::NativeFile`); sequential read, no seek, no fstat (s5-21/22 null reads).

## Design (to confirm with the lead before code)
1. **External editor host in the worker.** `Menu` stays pure. When the top screen is `Screen::External(editor)` (and for the grids that need engine types) the worker keeps an
   `Ext` value (module `editors`, sees `engine`) that takes the presses, produces the `Frame` and says Done (the worker then presses B on the model). Pure parts of each editor
   (cursor, ordering, labels) are host-tested in their own files. Settings edits go through the kept keys (`Settings::set_kept`) and the existing commit path
   (`published::publish` + `compat::menu::apply`), so the roster cycle sees them exactly as after any commit.
2. **Panels of pkg4 (Named library, Save slot, Edit placement, Replay window browser).** One generic `PanelView -> Frame` mapping (title -> crumb, labels -> windowed rows with the
   selected one focused, footer -> hint). The calls into `named_macros` / `replay_capture` read live game state (`practice_ready`, `acquire`): they are made on the worker thread
   (not in a hook, so R5's hook allocation delta stays 0; the layer's own `Vec`/`String` work is per input event). If reading the fighter manager from the worker proves unsafe,
   the fallback is a one-slot hand-off drained on the CPU body's frame (reported to the lead before use: it would be counted hook allocation).
3. **Clip editor (crop).** Own `View::Clip` frame (timeline bar, in / out handles, playhead, fighter line, help from `clip_help`) drawn with the delay popup's bar panes; it is
   foreground while the menu is closed, so the tick draws it when `replay_capture::panel_visible()` (new `state::PANEL_SHOWN` flag), input stays `on_p1_input` -> `replay_capture::on_input`.
4. **Pane table additions (one table, one recipe regeneration):** roster tiles (87) with per-tile P1 (red) / CPU (blue) badge text and a Solo mark, role / name texts, clip bar reuse; the
   recipe is rebuilt once per batch of table changes (Ally WSL `eden/ally/ally-build.sh recipe`, needs the stock archive from a diag boot).
5. **Portraits.** `ui::thumbs` becomes tile pictures for stage tiles and roster tiles (87 slots, paged requests like the stage grid); the path per fighter and costume is found by an offline
   probe on `data.arc` hash tables (candidate `ui/replace/chara/chara_<n>/chara_<n>_<name>_00.bntx`) and confirmed in Eden; text tile when absent (Q16).
6. **Give item.** A on a tile -> `character_items::request(choice)` from the worker; item icons are not part of this stream (placeholder text tile stays; the 38 icons were decided as the
   fork's own and are a layout-recipe item, tell the lead if wanted).
7. **Lobby Stages.** A adds/removes a stage in `LobbyStageOrder` (badges = rotation rank), Y cycles the form tag, All tile; `practice_stage_request::{request, request_in, menu_context}` for the
   in-lobby "go to this stage" action (to confirm the exact behaviour against SPEC and the carried `lobby_stages.rs`).
8. **Mii lists.** `Editor::MiiList(0..2)` opens the saved / replay Mii list from the published catalog (`App::take_catalog`), Add New... -> native keyboard / editor bridge, busts only where they
   exist (Q15, no texture needed: text rows first).
9. **Waiting-room host tick.** The fork hosts our panes in `info_waiting` under `TrModPort`; the clean room's recipe covers `info_training` only. Needs the waiting archive recipe
   (`tools/layout-builder` for `info_waiting` + our container), then the tick for `info_waiting` calling `lobby::set_ui_ready` / `set_native_ui_busy` per the lead relay.
   Scope check with the lead: if the waiting archive is a separate stream, I do the tick and the calls behind a resolved-panes check and report the archive as the missing piece.
10. **Leftovers.** (a) B3 direct `nn::fs` read (above). (b) measure in Eden whether any normal flow re-registers `info_training` within 5 s of the previous hand-out
    (`[LayoutArchive] handed out` timestamps, `no free archive buffer` count) across: boot to Training, back to menu and in again, stage change, mode change, hot swap, L+R+A reset.

## Gates (every screen commit)
host `cargo test` (one call), `cargo clippy --all-targets -- -D warnings`, `cargo build --no-default-features`, Switch release + diag builds, `nro_size.py`, Eden rows `ui-NN`
(screenshots compared to mockups `docs/menu-mockup/NN-*.png`, console.log scan, hook allocation delta 0, `Unmapped`), `eden/EDEN-UI-SCREENS.md`, compliance.py before READY.
No Eden or Switch build while another stream's claim is open on the host (ledger + `TRAININGROOMS-EDEN-QUEUE.md`).

## Next commands
1. Wait for the lead's go (merge gated), then
   `git -C /home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work worktree add -b stream/ui-screens /home/intpa/smash-training-lobby/TrainingRoomsForUltimate-ui-screens work/implementation`
2. Move this file to `IMPLEMENTATION-PROGRESS-ui-screens.md` in the worktree; append `## ui-screens` to CARRY-OVER.md only if something is carried (nothing planned).
3. Commit order: framework (external host + panel frame) -> Named library / Save slot / Edit placement -> Replay window + clip editor -> Give item -> Lobby Stages -> roster grids ->
   Mii lists -> waiting-room tick -> B3 + grace measurement.

## Lead rulings (2026-11-02, after the plan report) - binding
1. Threads: game-state reads (fighter manager, live entries) run only in game-synchronized context: the UI layout tick on the UI job (pkg5's tick) or the frame wrapper,
   never on the free-running menu worker thread. Do NOT move work to a thread to keep the allocation counter at 0: remove the allocations from those entry points
   (fixed buffers / Cow) or report the ones that cannot go. The worker thread keeps file I/O and pure model work. (Supersedes design point 2's worker-thread calls and the
   "fallback hand-off" wording: the panel entry points are driven from the tick or frame wrapper; allocation removal in the pkg4 entry points is part of this stream.)
2. Waiting-room archive is mine: build the `info_waiting` recipe (TrModPort with our panes) the way pkg5 built `info_training` (copy ranges of the stock archive + our panes,
   hash-checked, B3-style vanilla base), using the fork's ours-new waiting-UI design files as the reference for contents (check origins in provenance-files.json first); then the tick.
3. Item icons: yes. Q13: the item grid uses the fork's own 38 icons (our own assets, allowed in our recipe).
4. Design otherwise accepted. Branching waits for the lead's notice of integrator-2's final merge commit.

## Restart 2026-11-02 ~10:40Z (lead session ended 10:17Z; WIP c0dd217 split and re-committed)
- Model change: the stream was launched as claude-sonnet-5-5/high; after the resume the transcript shows claude-opus-5-5/high. Stopped and asked;
  the lead confirmed (operator's explicit choice, routing precedence 1). New hidden watcher `/tmp/ui-screens-watch/watch-opus.ps1` (pair opus-5-5/high,
  owner = the claude process of this session, log `watch-opus.log`).
- Restart rules: Ally rows only on the lent rig `C:\Eden\c1162-w5-ally` (stage_a.ps1, restore + verify after each run); never the trainingrooms-pkg2/-pkg5
  rigs, never answer a firewall prompt; batch rows per boot; commit and release every claim before stopping.
- Committed: `a67c349` (Recording page panels), `8658812` (install line rendered whole at boot; host test; lead carries it to work/implementation).
- Eden so far (pkg5 rig, before the ruling; evidence `eden/runs/ui-05/`): Named library empty and with a saved macro, Save slot refusal with an empty slot,
  Save slot name page, keyboard-confirmed save (diag `name`) written by the worker (`macros/evtest.json`), Edit placement refusal. The run froze behind a
  firewall prompt before the Replay window row. console.log: 10 `Unmapped` lines at 5 s at 0x109f5f3b7x: outside the plugin window, above 0x1000:
  attributed stock emulator noise (lead).
- In the tree, not yet gated: Give item A -> `character_items::request` from the tick (`ui::requests`), B3 vanilla read through `nn::fs` (no heap).

## Playbook (operator order 2026-11-02 ~12:0xZ)
Playbook followed: ae87443 (pulled 2026-11-02 ~11:40Z, already current). Read completely: README.md, docs/{CHECKLIST,WORKFLOW,EDEN,HOOK-SHARING,MACROS,TILE-CURSOR,GIFS}.md,
tools/PROVENANCE.md, and the tools run or assessed: tools/check/compliance.py, tools/build/nro_size.py, tools/eden-solo/{solo.ps1,gv.py}.

### Checklist (docs/CHECKLIST.md): M 33: pass 21, open 8, N/A 4. C 9: N/A 8, open 1
| Item | Tag | Result | Evidence |
| --- | --- | --- | --- |
| cdylib only | M | pass | Cargo.toml `crate-type = ["cdylib"]`; nro_size on the release NRO: open (next build) |
| release profile, skyline =0.6.0, toolchain 1.98.1 | M | pass | Cargo.toml [profile.release]; rust-toolchain.toml 1.98.1 |
| model pure + host tests, runtime cfg(switch) | M | pass | src/model/ui/{screens,flow}.rs, src/editors/*.rs tests; lib.rs `#[cfg(target_os = "switch")] mod runtime` |
| gates (test, clippy -D warnings, no-default-features, release, diag) | M | open | last full set at 97e8eb4; rerun on the next screen commit |
| no heap in hooks (heap_allocations=0) | M | pass (boot) / open (rows) | ui-06 install line `heap_allocations=0`; R5 row delta open (menu-close hang) |
| fixed buffers, writer thread, stack paths | M | pass | install_line::render on the boot thread; B3 `VANILLA_PATH_C` static path (Eden proof open) |
| root cause, one abstraction | M | open | menu-close hang: root cause found (parking_lot spin parker), one abstraction = stream/sync locks; not shipped yet |
| version guard 13.0.5 | M | pass | install line `game=13.0.5` (ui-09 console.log) |
| 64-byte guards | M | pass | install line `ui_*=stock/checked`; this stream adds no site |
| card scan for the site | M | N/A-as-M: no new site in this stream | sites are pkg5/pkg4's; no new hook |
| hook sharing | C | N/A | no new hook site |
| hook after prologue / call instead | M | pass | stream calls game functions only through pkg4/pkg5/pkg6 entry points |
| no diag strings in shipped NRO | M | open | compliance.py --nro on the next release NRO |
| shipped writes only install line | M | open | release NRO run not yet done for this stream |
| logging plugin data files | C | N/A | not a logging plugin |
| online read-only / replay A/B / two consoles | C x3 | N/A | the stream's screens run in Training and the lobby menu only; pkg6 owns online paths |
| Eden claim written and released | M | pass | ledger lines 10:52Z/11:04Z, 11:10Z/11:27Z RELEASED (Ally) |
| one Eden per host, >= 10 GiB, not profile a/:61 | M | pass | Ally free 12.8 GiB before ui-09; lent rig only |
| profile copy hard-linked | M | N/A (Windows lent rig) | lent rig is used in place with a verified set-aside/restore (stage_ui.ps1, mismatches []), lead's Ally rule |
| copies linked/copied (part of the profile-copy item) | M | N/A as above | no copied tree |
| one target dir per copy, df >= 40 GB, manifests | M | pass / open | cargo-targets/trainingrooms-ui-screens(-sw); df 173G; deletion manifest open at stream end |
| waits gated on a log line/screenshot | M | pass | ui_lent.py route GATE lines (ui-09) |
| precondition abort and retry under next label | M | pass | live-ui-09 relaunch after the failed launch (no run.json) |
| shortest ruleset | M | N/A | Training mode, no match end needed |
| profile restored, host released | M | pass | restore-verify 11:27:08Z `mismatches: []`, CLAIM back; ledger RELEASED |
| console.log scan, run.json, memory.peak kept | M | pass / open | ui-09 console.log + run.json in eden/runs/ui-09; memory.peak: open (wui.py records peak in run.json) |
| memory/time guard on local commands | M | pass | cg/sw wrappers: systemd-run MemoryMax=6G timeout 1800 |
| outputs under the label | M | pass | eden/runs/ui-0N/ |
| README paragraph + GIF | M | open (repo level, lead) | |
| dated evidence doc | M | pass | eden/EDEN-UI-SCREENS.md |
| numbers from tested scripts | C | open | R5 delta and grace numbers will need a tested log script |
| no operator quotes / claims beyond tests | M | pass | |
| handoff current | M | pass | this file, updated at checkpoints |
| release/tag/rebuild/MTP | C x3 | N/A | no operator order |
| user connects the Switch / nothing pushed or written | M x2 | pass | no push, tag, card, DBI or USB |
| Forgejo, push only on order, messages from a file | M | pass | origin git.syntaxbyexample.com; commits -F temp file |
| model/effort gate + watcher | M | pass | watch-opus.ps1 pid 1078155 |
| spawned workers read the playbook | M | N/A | no workers spawned |

### My own tools: replaced or kept
- `eden/lent/stage_ui.ps1` kept: set-aside/boot/restore of the lent Ally Windows rig (lead rule); playbook `gv.py prep` builds Yoga-local Linux profile copies.
- `eden/lent/wui.py` kept (pkg5's wa_s5.py re-rooted): launches/drives Windows Eden on the Ally; `gv.py`/`solo.ps1` are Yoga-only (AppImage, display :68, owner 88c0f449).
- `eden/lent/ui_lent.py` kept: ssh orchestration to the Ally; route uses the tile cursor (TILE-CURSOR) and screenshot gates (MACROS "Gate menu waits"). Segment macros: open.
- `eden/lent/rows_ui06.py` kept: this stream's row checks (pkg5's s5_walk walker).
- `eden/ally/ledger.py` kept: claim/release lines in the Yoga ledger from a non-Yoga runner; solo.ps1 only claims for its own local Eden.
- `/tmp/ui-screens-watch/{cg,sw,cgs,sws}.ps1` kept: the playbook's memory/time guard for cargo (no playbook tool for it).
- `pack/ally-switch.sh` kept: Ally WSL Switch build (lead-authorised host); no playbook remote-build tool.
- NRO size: replaced by playbook `tools/build/nro_size.py` from the next build on.

## 2026-11-02 ~13:30Z: merged work/implementation 48eebe1; design C; first slice in progress
- Merge `90341d2` (lead's order): sync slice 1, stack, the install line (fixed-buffer `render_into` replaces this branch's `render`),
  pkg6 5e0fab3. carry_pkg4 verify 42 files, carry_pkg6 verify 264.
- Menu-close hang: root cause and fix are stream/sync (hos-lock, `published::Live`), Eden-proven sync-01/sync-02 (46 + 45 closes). The
  diag probes of this branch went with the merge (published.rs); the worker/tick/input probes go in this slice.
- Design C (lead): the named-macro panel model has one owner, the menu worker. This branch: presses over the input ring (single
  producer), the worker runs the layer's entry points and builds the frame; `ui::state::FRAME` is now a `hos_lock::TripleBuffer`
  (worker writes, UI tick reads by reference: no lock, no skip); the worker calls `macros::port::worker_tick()` every wake (package 4
  points it at its layer service). pkg4 (a93ab): `MacroQueries` via `hos_lock::Published`, its own `hos_lock::Mailbox` for game-thread
  events and clip controls, playback buffers swapped by index. New hos-lock primitives here: `TripleBuffer`, `Mailbox` (host tests).
- Attribution (lead, u4b: 30-40 allocations per menu open): `HookId::{MenuInput, MenuDraw, MenuThumbs, MacroPanel}` nested scopes in the
  P1 menu adapter, the UI tick's menu part and the thumbnails; the `hook_stats ... allocs=` field names the step. Eden ui-10 measures it.

### Audit rows (ui-screens' part of the stream/sync table)
| Game-thread entry | Lock / wait | Other thread | Treatment |
| --- | --- | --- | --- |
| UI tick menu draw | `ui::state::FRAME` | menu worker (frame build) | `TripleBuffer`: worker writes, tick reads by reference; neither waits |
| P1 menu adapter | worker press ring | menu worker | lock-free SPSC ring (single producer: the P1 hook), counted drop |
| Give item (tick) | `ui::requests::GIVE_ITEM` atomic | menu worker | atomic handoff; the tick calls `character_items::request` |
| named-macro hooks (pkg4) | `UI`, `RECORD_SLOT`, `MOTION`, ... | menu worker (panel entry points) | design C: pkg4 moves hooks to published queries + mailbox; until then the worker holds them only for model steps |

## 2026-11-02 ~13:45Z checkpoint (restart-safe)
- Branch `stream/ui-screens`, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-ui-screens`, HEAD `4a979d2` (WIP sync slice 3) on
  `ade1dd9` (merge of work 8b73694: pkg4's providers + facts rule) on `1992045` (slice 1 gated) on `90341d2` (merge 48eebe1).
- Eden: ui-10 (diag 45860626): menu-open attribution = 0 menu allocations per open (`menu_input` 0, `menu_thumbs` 0, `menu_draw` 1 once);
  all growth `roster_preload` (pkg6 R5/F10); Give item, Lobby Stages, Save slot pass; 0 Unmapped. sync-01/sync-02 on stream/sync.
- Gates at 4a979d2: tests 9251 pass, clippy 0, no-default-features ok, Switch release dd4616c8 + diag badc76cf 0 warnings;
  hook_locks fails on 2 owner paths: (1) pkg6 preload `publish_applied -> load_cache -> assign(&MENU) -> store -> EDITED` on the
  publishing path; (2) pkg4 `worker -> recorder::with_recorder -> REC` (design C). Lead asked: owner follow-ups or temporary ALLOWED rows.
- No claim held; lent rig restored (13:32Z `mismatches: []`); Ally released 13:44Z.
- Next: lead's answer on the 2 paths -> green sha to integrator-3; pkg4 design C on top; then roster grid (13x7 = 87 cells; pane table
  40 -> 88 tiles, labels carry rank/role, est. +32-40 KB resident; portraits through u4's texture path), Mii lists, waiting room,
  B3 stack path + archive-hook allocation; stream/sync's uncommitted `tests/hook_locks.rs` filter refinement is superseded by this branch's.
- Own tools added since the playbook record: `eden/lent/rows_sync01(b).py`, `rows_sync02c.py`, `rows_ui10.py` (row drivers on the Ally lent
  rig; playbook drivers are Yoga-local), `ally-build.sh` modes `hoslock`/`hooklocks` (single-crate / single-test re-runs inside a claim).

## 2026-11-02 13:52Z roster grid: pane-table cost measured (Ally WSL layout-builder over the stock archive)
- 40 tiles: target 529,300 B, recipe 55,540 B (reproduces the committed `assets/info_training.tra1`, sha 225fdfd0). 88 tiles: target 549,780 B, recipe 64,700 B.
- Resident delta at 88: archive +20,480 x 2 boot buffers = +40,960; recipe .rodata +9,160; Frame +~2.9 KB x 4 static copies = +~11.7 KB; total ~+61.8 KB (> 40 KB: lead decides; options (a) paged on 40 tiles 0 KB, (b) 88 tiles, (c) 88 with tile text out of Frame ~54 KB). Nothing committed for it.

