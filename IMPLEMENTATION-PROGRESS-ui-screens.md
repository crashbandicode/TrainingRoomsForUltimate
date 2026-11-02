# Implementation progress, ui-screens stream (planning copy; moves into the worktree once it exists)

## Current successor checkpoint (2026-11-02, thread 01a19ccb)
- Latest Rust source cd09454 (8532ebc waiting archive/registration; b4e497e separately authorized HUD+native-only notice gate; cd09454 request-off visibility). Yoga src stagedcd09454, tar SHA256c0803f9d0fbcddf7114bcf5b8aeeae94a5d0590f25d779f2cc0ca90daab61478. Yoga diagnostic and Surface host-test queue entries name this snapshot. No active claim/compile/Eden; previous ACTIVE lines below are historical and released.
- Evidence-gate improvement: reused eden/hook_stats.py (no new harness/analyzer tool) for per-load archive counters + exact matching name/size releases and >=5000ms grace. Skips, missing/truncated evidence, early/unmatched/unretired hand-outs fail. ui11 retains complete hook+archive lines and applies both gates. Bounded `python3 -I tests/eden/test_analyze.py HookStatsGateTests LayoutArchiveGateTests` passed9/9, including5000/4999ms boundary and missing coverage. New Python/docs-only slice leaves queued Rust source unchanged. Final runtime rows in EDEN-UI-SCREENS.md explicitly pending; historical Unmapped dismissals are not acceptance.
- Yoga builder-test RELEASED15:51Z:11passed0failed/10.40s compile, own warnings0, pinned binrw future-compat notice. No current claim. Current queue Surface host test and Yoga diagnostic compile still8532ebc; update source identities after the authorized HUD slice below before claiming. Remote src staged8532ebc, prior builder source retained as src-44177d8; archive SHA256ebeee434b819d989b3f0609356c1fdb6415dfb75b56f468eb04d239f2749c3e8.
- Lead LEAD-INBOX interface decisions15:4xZ authorizes UI's narrow hud_draw::draw waiting admission/container hunk (pkg4 diff empty; preserve at integration), separate commit. Implemented waiting=requested admission and rootTrModPort while keeping Training scene gate. Native-only Combo Display requirement needs the one-line notices::hud_tick predicate too; no general pkg4 ownership transfer. Full hud_draw read and relevant notices/scene/input_display paths inspected. Training/waiting transition Eden evidence pending.
- UI tick additionally hides the whole waiting host when lobby request ends (native layout can remain alive), with null check for uninjected/refused archives. Builder/pure model tests do not establish native pointer/transition safety; final rows must cover this.
- ACTIVE 15:50Z Yoga CLAIM for one builder-test at44177d8 (session51072), /home/intpa/trainingrooms-build-ui-screens-astra/logs/44177d8-builder-test.{log,json}. Release/remove queue immediately on exit. Full parser+raw ledger empty; queue head; remote no compiler/Eden, disk240GB; remote roots28GiB plus Surface targets19GiB before this small builder test. No rig mutation. Current local HEAD8532ebc; Surface plugin cargo test queued separately at tail, not started.
- Waiting tick follow-up (ungated): registration generation now advances before HUD dispatch, so same-address recreation invalidates cached containers before use. Waiting readiness includes notice/input-log roots. Texture retirement advances only on regular UI passes, not the extra registration calculate. Fully read SPEC-input-02/05 for these host/registration seams; pkg4 callback admission remains a shared-file request in QUESTIONS/inbox.
- 15:35Z RELEASED Yoga builder claim, exit0/18.71s, own warnings0 (pinned binrw future-incompat dependency notice). No live compiler/Eden. Requeued builder tests at tail after slim/pkg4, own fairness until15:38Z. Local raw build log/receipt and recipe reports: eden/runs/ui-waiting-build-44177d8/.
- Generated assets/info_waiting.tra1 SHA256599c7d6fe61568efae283356e237eac05784f817c8b2c02db12682b873a35261; 65,784B, output1,378,180B FNVc2130180a0050a73 from stock1,284,116B SHA2567dcc45360bd84dadffcfe8b47771f5d6b40e44468c6a54cbd3dbb6b4142eea64. Private stock/target stay uncommitted. All184 archive members preserved; only blyt/info_waiting.bflyt payload changed, other183 byte-identical. All63 stock pane records byte-identical;549 new panes, total612; menu/notice/log each parentTrModPort, host parentRootPane. Untouched writer roundtrip is144B shorter; final payload/pane preservation verified, exact untouched container delta not yet inspected and no byte-identical archive claim. Need recipe host tests and Switch/runtime gate.
- Historical 15:34Z Yoga CLAIM (released15:35Z) Codex01a19ccb/ui-screens: ONE builder release compile at44177d8, tool session54133; remote /home/intpa/trainingrooms-build-ui-screens-astra/src and builder-target, logs/44177d8-builder-build.{log,json}. Release immediately on exit, remove own queue line, then requeue at tail for builder tests (3min fairness). Archive tar local/remote SHA2569bcee64a79efbe7de435f003209e72c63a244e86a09874fad26f4318d334a3b2. Build helper adapted from mandated Yoga build.ps1, records raw log and exit receipt (not last3 lines only). 6G/1800s, incremental0; disk240GB; all remote build roots+targets28GiB upper bound. No Eden/profile mutation.
- Full ledger reconciled through shared harness225d16bd9d7f7645f2c18058373f3331624b6c94 pure parser (ledger.py/params.py read completely) plus raw noncanonical entries; Yoga has no outstanding claims. Historical no-Yoga/withdrawn parenthetical notes cause parallel_turns false positives; inspected queue shows no actual parallel Yoga request. Lead authorized this exact harness via LEAD-INBOX, no merge.
- Uncommitted archive candidate now selects training/waiting recipe+path/dump via one static descriptor table, logs actual layout name, sizes the SAME two buffers to max stock/output of both recipes. Generated waiting asset is now present. Vanilla waiting path remains to verify from owned game/runtime; no runtime or allocation pass claimed.
- Resumed ~15:2xZ after checkpoint088367d. Exact live model reverified Astra/medium; one existing watcher2926069 (owner2869927), no mismatch sentinel. Earlier handoff text below is historical, not a request to stop.
- Waiting builder root fix: all three top-level panels (menu, notices, input log) now parent to TrModPort; internal hierarchy unchanged. Added regression in layout-builder. Stock waiting input located at artifacts/training-performance-followup-20261015/stock-info-waiting.arc in campaign root. Recipe not generated yet; builder test/build remain queued/ungated.
- ui-11 evidence bug fixed: old row passed merely on finding two counters, even when allocations increased. Reuses existing eden/hook_stats.py gate over complete retained log lines, requires ui_archive/menu_draw coverage and every delta/final total zero, preserves roster-hook-stats.log, exits nonzero on any failed row. No parallel analyzer added. Runtime acceptance pending. Existing HookStatsGateTests passed 5/5 via bounded python3 -I tests/eden/test_analyze.py HookStatsGateTests (15:2xZ).
- Current queue: ui-screens Yoga after pkg4 then f6; pkg4 claimed15:21Z and removed its queue line after its run. No own claim/compile/Eden or profile changes. Full ledger retrieval exceeded output cap; NOT claimed fully reconciled. Must summarize all entries and inspect unmatched/noncanonical releases before claiming; do not rely on the truncated output or stale12h helper.
- Objective unchanged: all ui-screens done criteria, then READY. Latest committed HEAD44177d8 on stream/ui-screens; current dirty waiting archive/tick/asset/docs candidate is not gated. Inherited a445046 and checkpoint088367d remain in history.
- Exact thread 01a19ccb-937e-71c0-b7cf-9447b5365d99 verified from sqlite and latest turn_context: gpt-6-astra / medium. Watcher PID 2926069, owner 2869927, /tmp/ui-model-01a19ccb-937e-71c0-b7cf-9447b5365d99.py (change sentinel .py.status). No subdelegation (brief prohibition).
- Playbook followed: ae87443 (pull --ff-only already current). This successor read README, every docs file, tools/PROVENANCE completely. The full M/C checklist below remains the acceptance checklist; historical passes are not fresh evidence. No new gate passed yet.
- Required reads completed: worker brief, routing/model instructions, STREAM-RULES, HANDOVER-ASTRA, entire stream progress and git history; BUILD, DECISIONS, MENU-DRAFT, implementation-packages, CARRY-OVER, IMPLEMENTATION-PROGRESS, TEMPLATE, INDEX-core/input/state, SPEC-core-02/03/04/05/07. Parent AGENTS and current first85 handoff lines read; older historical handoff not claimed fully read. Remaining specs/interfaces read before their changes.
- Live ledger tail checked: Ally held by tm-f8-sol-eden from15:10:09Z. Yoga queue has pkg4 before inherited ui-screens a445046 test entry. No own claim, build or Eden launch, no profile changes. No source exposure outside clean-room boundaries.
- Next: reconcile full ledger and predecessor remote build state before adopting queue; validate candidate host build, complete waiting recipe/runtime archive and Mii native lifecycle/bust bridge in coordination with pkg6; enforce all hook deltas zero in retained Eden rows. Two archive buffers retained.

### Handoff at context-hygiene checkpoint
- No implementation changed by this successor. The inherited waiting builder diff is included in the checkpoint commit as UNGATED WIP: build-waiting uses info_waiting.bflyt, native pic_bg_main_00/set_txt_status templates, and adds TrModPort as parent of menu root. The historical add_container function was unused and removed by the predecessor. Template evidence to inspect before execution: /tmp/ui-screens-stock-waiting.bflyt and /tmp/ui-screens-waiting-templates.txt. No recipe has been generated here.
- Source reads this successor: runtime/ui/archive.rs, tick.rs, thumbs.rs, editors/mii.rs and layout-builder main.rs complete; worker.rs first100 only. Archive uses std::sync::Mutex + Vec pool/parked and Thread::unpark; do not assume static file path alone removed all archive-hook allocation. Waiting injection still absent from archive.rs. Candidate tick readiness and texture-retirement changes require lifetime/concurrency review and runtime proof.
- Tools read completely here: eden/ally/ledger.py and layout-builder main.rs. ledger.py has a 12h stale filter and excludes its own owner, so its open output alone does NOT meet the instruction to refuse ANY unreleased claim; inspect full ledger, including historical nonstandard releases. Do not run its claim path as sole admission check.
- Exact next work: read remaining owned input/state specs as needed for entry wiring and provider interfaces; finish worker/menu_app, texture/chips and pkg6 Mii provider review; consult current pkg6 progress for native lifecycle split and pkg4 progress for design C. Full core specs02/03/04/05/07 already read, no need to repeat within this thread.
- Queue entry still names predecessor 01a19c9f on Yoga behind pkg4, source a445046; no claim made. Before a new build update/adopt that one entry, inspect the full live Yoga ledger and remote trainingrooms-build-ui-screens-astra state, and confirm source identity. Do not merge moving branches: the one authorized ebfcb98 merge already happened at9343e05.
- Read-only next commands (each through /usr/bin/pwsh -NoLogo -NoProfile -NonInteractive -Command, under systemd memory/time bounds): git status --short --branch; git log -5 --oneline; read current TRAININGROOMS-EDEN-QUEUE.md; ssh -o BatchMode=yes -i /home/intpa/.ssh/id_rsa memento-claw-yoga-ubuntu 'cat /tmp/deluxe-surface-1007/OWNER-COORDINATION.md'. Remote Linux native helper shell is authorized. Every cargo needs its own admitted claim and CARGO_INCREMENTAL=0.
- No claims to release, no profile to restore, no compilation subprocess or Eden owned by this successor. Watcher reaped before yielding; rearm after model verification on continuation. No push/tag/release/card/device action. READY remains unmet, goal remains active; this checkpoint is not an acceptance handoff.
- Final cleanup supersedes the queue note above: removed only the inherited ui-screens Yoga queue line at this checkpoint so a stopped successor does not reserve the host. Requeue at the tail on continuation; do not reuse its former position. Watcher2882337 stopped with no mismatch sentinel.

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


## Handover (2026-11-02 ~14:08Z, operator order: lead role to the Codex GPT-6 Astra lead)
- Branch `stream/ui-screens`, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-ui-screens`, head = the "WIP: handover to Astra lead"
  commit on `237b857` (roster grid gated). Everything before it is gated; the handover commit adds only this section.
- Targets (incremental dirs deleted): Ally WSL `/home/intpa/c1161-targets/TrainingRoomsForUltimate-ui-screens-ally` 1.5 GB (the gate target);
  local `cargo-targets/trainingrooms-ui-screens` 1.1 GB, `-sw` 0.3 GB. Evidence NROs: `/tmp/ui-screens-watch/ui-out3/` (roster slice: release
  c4205a5b, diag 1eaa4619), `ui-out1/` (slice 1: 57bc5127 / 45860626).
- Proven:
  - menu-close hang fixed at the root (stream/sync, hos-lock + published Live): Eden sync-01/sync-02 (46 + 45 closes, named macro load and
    playback, publish from the diag thread, 13 saves + load through the writer mailbox, 60 fps, hook allocations unchanged):
    `TrainingRoomsForUltimate-sync/eden/EDEN-SYNC.md`, runs under `eden/runs/sync-0{1,2}`;
  - ui-10 (slice 1): menu adds 0 hook allocations per open (growth is `roster_preload`), Give item, Lobby Stages, Save slot, 0 Unmapped:
    `eden/runs/ui-10/`, report in this file;
  - roster grid: host-tested (`editors/roster.rs` tests, `tests/core02_menu.rs`), gated (tests 9256 + core02 203/0, hook_locks green with
    one PENDING row, clippy 0, no-default-features, Switch release/diag 0 warnings). NOT yet Eden-proven.
- Handed to integrator-3: `71ba1f7` (slice 1 + merge 8b73694 + sync slice 3). After it: `237b857` (roster grid) is ready for the next merge.
- NEXT:
  1. Eden ui-11 on the lent Ally rig when the queue allows (tm-f8-sol held the Ally at 14:05Z; interleave rule for tm-f8-terra*/sol*):
     `ui_lent.py stage ui11-diag /tmp/ui-screens-watch/ui-out3/libtraining_rooms.diag.nro`; `python3 -c "import ui_lent as u; u.stage_mode('-Mode','setaside')"`;
     `ui_lent.py boot ui11-diag`; `ui_lent.py launch live-ui-11` (wait for run.json state ready); `ui_lent.py route live-ui-11`;
     `rows_ui11.py live-ui-11` (roster rows + the install line read whole); then kill, `ui_lent.py restore` (mismatches []), release.
  2. Fighter portraits on the roster tiles through u4's texture path (one texture path; text tiles are the Q16 fallback meanwhile).
  3. Y = Solo Popo on the roster grid (not implemented; pkg6's Solo Popo kept rows).
  4. Mii lists; waiting-room recipe + tick (set_ui_ready / set_native_ui_busy); B3 stack path Eden proof + the archive-hook allocation +
     the 5 s grace measurement (lead: one-buffer option only if no normal flow reloads within the grace).
  5. pkg4 design C lands on top (its PENDING hook_locks row must then disappear; the RC gate needs PENDING empty).
- Blockers: none of mine. Open claims: none (Ally released 14:02Z). Queue lines: deleted. Lent rig: restored 13:32Z (`mismatches: []`),
  untouched since. Watcher `/tmp/ui-screens-watch/watch-opus.ps1` stopped.

## Astra continuation checkpoint (2026-11-02)

Objective: complete this stream's brief, all runtime rows and final gates, then mark READY. NOT READY.
Exact branch/worktree: stream/ui-screens / /home/intpa/smash-training-lobby/TrainingRoomsForUltimate-ui-screens.
Starting and current code HEAD: 0a09ea569188d364a9f8cc1693f74c9e2fd9f653; clean at takeover. No merge or code edit yet.
Thread: 01a19c9f-0f91-7fa3-9a01-dce8c4590865, verified exact sqlite row and latest turn_context: gpt-6-astra / medium.
Playbook followed: ae87443cbb801472e474b3f0c20b5f9c3f5f1f77 (pull --ff-only: already up to date).
Watcher: PID 2602023, owner Codex PID 2589234, temp script /tmp/ui-screens-model-01a19c9f-0f91-7fa3-9a01-dce8c4590865.py.
Change sentinel: same basename .status (absent at last check). Polls exact rollout model/effort and owner liveness; 12-hour bound. Reap before yielding; verify and rearm after compaction.
No host claims, queue entries, builds, Eden launches, profile mutations or external publication performed by this successor.
No subdelegation: stream brief forbids it; request through lead. No clean-room boundary exposure by this successor.

### Reads completed by this successor
- Full worker brief, DELEGATION_ROUTING.md, MY_MODEL.md, STREAM-RULES, HANDOVER-ASTRA, this progress file including historical handover.
- ssbu-mod-playbook skill, README, every docs/ file (CHECKLIST, WORKFLOW, EDEN, MACROS, TILE-CURSOR, GIFS, HOOK-SHARING), tools/PROVENANCE.md.
- Repo BUILD, DECISIONS, implementation-packages, IMPLEMENTATION-PROGRESS, MENU-DRAFT, CARRY-OVER, specs/TEMPLATE, INDEX-core, SPEC-core-02/03/04/05/07.
  Large output truncations were followed by targeted reads; MENU-DRAFT lines 221-250 still need checking before claiming the entire draft read.
- Shared INTEGRATION-OPEN and EDEN-QUEUE. Parent AGENTS.md and first 90 lines of its historical TRAINING_LOBBY_HANDOFF.md (669060 bytes; not claimed fully read).
  Parent handoff records later operator size ruling: rustfmt-normalized target >=15% under fork is NEXT-version target, not RC blocker; L2 two hos-lock allows approved. Ask lead to reconcile outdated size wording if needed.
- Source read: src/editors/roster.rs; runtime/ui/{thumbs,archive,worker,tick}.rs; model/ui/frame.rs; model/ui_texture.rs.
- Tools read: eden/lent/ui_lent.py and rows_ui11.py only. Their dependencies MUST be read before running. No playbook tool scripts have yet been run; read them fully first.
- Remaining preimplementation reads: relevant input/state specs/indexes for remaining owned screens; required existing provider/model/runtime interfaces; read tools used completely.
- No repo-level AGENTS.md found by rg --files -g AGENTS.md <worktree>; parent AGENTS applies.

### Concrete findings to act on
1. Roster grid already has 88 panes, 13x7, retained release c4205a5b / diag 1eaa4619. ui-11 unrun. Historical claims of gates are not this successor's final evidence.
2. src/runtime/ui/thumbs.rs uses ONE AtomicU64 BOUND yet p::TILES is 88. is_bound(i) shifts 1 << i with no word indexing; roster pictures require fixing representation at root and testing cells 63/64/87. Existing active path is stage-only.
3. RosterEditor currently uses Y to remove; approved contract is Y Solo Popo on Ice Climbers and R remove. Worker/model dispatch and hint must be checked together. No changes made.
4. Frame has tile_n, tile_stage_grid, tile_sig; portrait source must integrate with u4 texture path, not create another provider.
5. archive.rs already reads vanilla with direct nn::fs static NUL path (no std File open); zero-allocation runtime evidence still owed. Uses std::sync Mutex<Vec> pool/parked; inspect first-lock allocation as possible archive hook root cause rather than assuming read_vanilla is still the source. Two buffers retained. No proposed workaround coded.
6. tick.rs handles info_training only; waiting recipe and TrModPort tick absent. Must coordinate existing lobby readiness/native busy and pkg6 single init.
7. rows_ui11.py hook-allocation row merely checks counters parse, does NOT require delta zero. Final regression must enforce every hook delta zero plus 0 Unmapped.
8. No shared eden/harness directory in this stream yet. Brief authorizes ONE start merge ebfcb98, not moving work branch; git diff HEAD ebfcb98 shows roster slice absent there but merger should retain branch changes. Inspect ancestry before merge.
9. Shared queue has foreign tm-f8 and imgui lines; no admission attempted. Live Yoga ledger still unread. Queue alone never authorizes claim.

### Own acceptance checklist (all pending unless explicit evidence below)
Prior predecessor checklist stays above as historical evidence, not fresh passes.
| Requirement | Tag | Result | Evidence / next proof |
| --- | --- | --- | --- |
| cdylib only and resident measurement | M | pending | inspect Cargo.toml; final release nro_size |
| release profile, skyline pin, toolchain | M | pending | BUILD read; verify manifests |
| pure model + host tests, Switch cfg | M | pending | final source/test review |
| host test, clippy zero, no-default, release, diag | M | pending | no successor compile yet |
| no hook heap incl archive and all hook deltas | M | pending | final Eden regression |
| fixed buffers, static writer stack, stack paths | M | pending | source + runtime proof |
| root fixes; one abstraction; approved exceptions | M | pending | audit final patch |
| version guard | M | pending | final install line |
| every called/hooked/relied site 64-byte guard | M | pending | final site audit |
| card-copy site scan | M | pending | reuse scan evidence if no new sites; scan any additions |
| hook sharing if possible collision | C | pending | determine sites first |
| specific hook after prologue / call instead | M | pending | final site audit |
| no diag code/strings in shipped NRO | M | pending | final compliance |
| release writes install line only | M | pending | final release row |
| logging plugin data exception | C | N/A: not logging product | UI stream |
| online read-only review | C | pending | waiting-room path applicability / lead ownership |
| replay A/B | C | pending | coordinate pkg7 applicability |
| two-console online-only path | C | pending | waiting-room row / lead ownership |
| claim before Up, release after Down | M | pending | no runs yet |
| one Eden, >=10GiB, own profile/display | M | pending | admission proof |
| hardlink identical, real copies writable | M | pending | rig receipt |
| one target/copy, disk >=40GB, target cleanup manifest | M | pending | final manifests |
| gated waits | M | pending | run receipts |
| abort failed precondition, new label retry | M | pending | run records |
| shortest adequate ruleset | M | pending | Training/lobby row evidence |
| restore profile and release | M | pending | restore verification |
| console scan, raw logs, run.json, memory.peak | M | pending | retained runs |
| bounded local commands | M | pending | initial bootstrap reads were unwrapped; subsequent calls memory 1G/time30; builds need6G/1800 |
| outputs under labels | M | pending | retained runs |
| README paragraph, GIF, install/build/license/disclaimer | M | pending | coordinate lead repo-level deliverable |
| dated evidence | M | pending | EDEN-UI-SCREENS update |
| computed numbers from committed tested script | C | pending | fix allocation delta checking and grace analysis |
| no operator quotes / claims beyond tests | M | pending | final review |
| current handoff | M | pass | this checkpoint |
| release version/tag/assets | C | not authorized | brief forbids release/tag |
| tagged rebuild identity | C | N/A: no release authorized | stream only |
| Switch install/readbacks | C | not authorized | brief forbids Switch/card/DBI/USB |
| user owns connection/mount lifecycle | M | pass | none touched |
| nothing pushed/tagged/card-written without order | M | pass | none performed |
| Forgejo, commit message UTF8 file, identity | M | pending | checkpoint commit then final review |
| exact model gate + watcher | M | pass | exact thread astra/medium, watcher above |
| spawned workers own playbook read/compliance | M | N/A: subdelegation forbidden | no workers spawned |

### Exact next actions
- After compaction verify exact thread model/effort per MY_MODEL.md; reconcile watcher before rearming.
- Finish outstanding required reads in bounded chunks; avoid dumping huge historical handoff.
- Inspect start merge ancestry, preserve owned roster changes; merge only authorized ebfcb98 with UTF8 message file if useful.
- Read current live Yoga ledger via existing transport and read existing harness/driver dependencies before any claim.
- Queue ui-11 on Ally respecting tm-f8*/imgui interleave and 3-minute fairness; use retained hashed diag if appropriate, mute before boot, restore+release on all exits.
- Implement portraits + Solo Popo; then Mii lists/busts and waiting archive/tick; B3 attribution/grace; coordinate pkg4/pkg6 via progress/lead.
- Never mark READY until all done criteria, final host/Switch gates and compliance report with final release NRO are met.

## Astra active checkpoint — Solo Popo slice (2026-11-02)
- Current integration base: 9343e05b893e359e221545d9a5af2be180d9ff95, clean authorized no-ff merge of fixed ebfcb98; only integrator-3 progress documentation arrived.
- Model reverified after compaction: exact thread 01a19c9f-0f91-7fa3-9a01-dce8c4590865, gpt-6-astra/medium. One hidden watcher PID 2648540, owner 2589234; no mismatch sentinel.
- In-progress source: role-specific Solo Popo kept preferences, Y on Ice Climbers cycle tile; R removes roster members; role-specific help, S label. Pure editor apply owns snapshot updates; regression cases cover role independence and grid input forwarding. NOT compiled or Eden-proven yet.
- Queued Surface host test behind pkg6/slim and required imgui interleave. No compile/Eden claim held, no remote profile mutations.
- Portrait path/costume mapping remains SPEC-core-07 open question 6; use existing u4 diagnostic probe and owned game resources to resolve before implementation. No guessed mapping or baked image.
- Texture review found 64-bit bound mask versus 88 panes and 64 retirement slots: must resolve when enabling portraits, preserving GPU grace rather than immediate release on overflow.
- Continue Mii catalog integration through existing pkg6 CatalogMenu/context; do not duplicate native providers. READY remains unmet.

## QUESTIONS
- 15:3xZ waiting consumer seam for pkg4/lead: runtime/features/hud_draw.rs::draw returns unless name==info_training and scene::is_training(), so notification/input panes now generated beneath TrModPort are not yet drawn in waiting. Please assign/authorize that small shared-file hunk: admit info_waiting only when lobby::requested(), resolve notice/log below TrModPort, retain native-only Combo Display clearing per SPEC-input-05 §6. UI tick now publishes registration generation before hud::run (old order could use cached stale panes on same-address registration), and waiting readiness verifies notice/log roots too. No edit to pkg4 hud_draw yet; this is an acceptance dependency, not an operator blocker.
- Lead/pkg6 coordination: this stream will consume QUICK_MENU_APP::take_catalog and the existing CatalogMenu/context, and dispatch its PendingSelection/EditorRequest to pkg6. Please identify any single-init interface changes required beyond this fixed base; no second Mii provider will be added.
- Portrait mapping probe is still required by SPEC-core-07 Q6. If another stream has already established costume-specific path facts from the owned game, please point to its evidence; otherwise ui-screens will run the u4 probe in its next batched Eden.- Concrete pkg6 interface finding: mii_saved_alpha::{menu_open,menu_ready,menu_close} currently have no UI callers at this base. menu_ready reads mii_backend::current (native game-mode data), while the catalog broker request allocates a sync_channel and capture publishes a cloned Snapshot from the game callback. The UI worker cannot read native state, and calling these wholesale in a hook would violate the required zero-allocation deltas. Please coordinate the native-side facts/request split with pkg6's R5 work; ui-screens is continuing the pure CatalogMenu/list and rendering integration, not adding a second catalog provider or suppressing allocation accounting.
### Active implementation checkpoint 14:48Z
- Solo Popo slice committed as f09e278; still ungated (Surface queue has pkg6/slim and parallel-project interleave ahead).
- Mii list extends the existing CatalogMenu through manifest transforms, adds src/editors/mii.rs and worker rendering/catalog consumption. Tests cover synthetic saved CPU selection/close action and waiting Add New for all three classes. Native opening/closing and Add New dispatch are NOT wired yet; the zero-allocation native/provider split in QUESTIONS is outstanding. Busts pending.
- Carry tool fully read before use; carry + ledger + verify completed, all 264 files match frozen m5q transforms. CARRY-OVER.md records provenance. No stage/fork edits.
- Thumbnail bound state changed from one u64 to one atomic flag per pane (88 tiles); retirement capacity/lifetime still to resolve before portraits enabled.
- Waiting tick now admits info_waiting when lobby practice requested, resolves beneath TrModPort, publishes readiness and uses native option-menu x plus existing popup_busy provider. Waiting archive recipe/graft and runtime acceptance remain pending.
- REA skill read, MCP tools absent, doctor found aligned registration and working Ghidra; using existing rea CLI without setup changes. Bounded 300-second/4GiB decompile of owned main-13.0.5.elf key builder 0x3238a70 is live (exec session13503), output /tmp/ui-screens-portrait-key-evidence.json. No REA finding claimed yet.
- No compiler/Eden claim or rig mutation. One model watcher PID2648540. Next: complete native Mii bridge coordination, portrait key evidence/probe, waiting recipe, then queued gates and batched Eden; no READY claim.
### Portrait implementation checkpoint (still ungated)
- REA CLI session13503 ended exit1, write EPIPE, zero output; no REA evidence claimed. No setup modified.
- Reused owned 13.0.5 assembly artifact `refactor/hud-costume-lobby/re/3238a70-icon-hash.asm` plus carried `swap_model::presentation` facts: the native key builder accepts tagged ui_chara, entry costume, picture type; handles patch paths and ROB regional costume mapping. Existing seam rows already guard 0x3238a70 and fighter-kind conversion0x138c9b0. No new guessed pathname table.
- Manifest transform extracts one shared `fighter_picture_hash` from existing waiting-icon preload and exposes UI-thread `menu_portrait_hash`. u4 texture loader now requests roster tiles using it (type1, existing U4 chara_1 evidence). Frame carries kind/role and invalidates picture set on form/role change; native reads remain on UI job thread. Text fallback retained. Needs runtime costume/mod proof.
- Retirement capacity derives from all88 tile panes + one chip texture +5 probe textures per frame, times9 generations, rather than64 slots. Added worst-rate grace regression; single clock in layout tick (removed duplicate probe clock), layout loss retires pictures/chips with grace. Resident delta must be measured by build gates.
- The static bound proves ordinary retire calls fit; final Eden still must measure repeated pages/layout destruction, exact delta0 and0Unmapped. No runtime acceptance claimed.
