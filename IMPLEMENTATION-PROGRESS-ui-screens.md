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
