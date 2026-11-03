# Boot, dependencies and install — behavioural specification

- **Spec id:** `SPEC-core-01`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), spec-writer role, 2026-11-01
- **Upstream reference read (spec writer only):** `src/lib.rs`, `src/common/mod.rs`, `src/common/offsets.rs`, `src/common/filesystem.rs`, `src/common/dev_config.rs`, `src/logging.rs`, `src/common/events.rs`, `src/common/release.rs`, `src/common/dialog.rs`, `src/training/mod.rs` (the install function and the once-per-frame drivers only), `src/hazard_manager/mod.rs` (install call only), `src/training/debug.rs`, `training_mod_sync/src/lib.rs`, `training_mod_metrics/src/main.rs`, `Cargo.toml` at commit `97cbe9b9` (and `077a8e43` where it differs: `src/training/mod.rs` +8/−1); fork changes at stage M5i `src/lib.rs` lines `3-20, 24, 27, 37-40, 43-46, 48-113, 115-147, 150, 154-159, 162-174, 176-209, 214-269, …`, `src/common/mod.rs` `8, 16, 28, 31-35, 38, 45-54, 57-61, 69-72, 75-79, 86-91, 100-106, 108-111, …`, `src/common/offsets.rs` `5, 42-90, 94, 96-101, 124-125, …` (the `SITE_*` constants and the table-backed needle), `src/common/filesystem.rs` `12-37, 39-40, 55-67, 70-91, 102`, `src/common/dev_config.rs` `4, 11, 37, 40-41, 48`, `src/logging.rs` `1-16, 20-284, 329-344`, `src/training/debug.rs` `4, 8-15, 105-115`, `src/training/mod.rs` `1197-1378` (the install function), plus the fork-only files `src/model.rs`, `src/hang_trace.rs`, `src/training/seams.rs`, `training_mod_consts/src/native_seams/rows_a3.rs` (addresses only), from `docs/provenance/provenance-files.md`.
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

When Super Smash Bros. Ultimate 13.0.5 starts with the plugin on the SD card, the plugin checks that the running game is the one it was built for, verifies every native site it intends to touch, installs its hooks, creates its data folder, loads the saved menu, shows a short welcome on the first Training HUD, and writes exactly one install line that says what happened. On any other game version it logs `result=unsupported_version` and does nothing else. Everything the player sees afterwards (menu, CPU behaviour, save states, hitbox visualizer) is driven by the per-frame entry points this spec installs.

## 2. User-visible behaviour

- No menu items of its own, except that it owns the *Developer* page's gate and rows (draft 9, decided 2026-11-01; below). (Upstream had an `Auto-Update` row, `update_policy`, under Misc: Stable / Beta / Disabled, default Stable. It is **dropped**, see §6 "Dropped upstream behaviour".)
- **Developer page (draft 9; `docs/DECISIONS.md` menu question 8, B9, N1).** The menu has a sixth page, *Developer* (layout and rows: SPEC-core-02 §2.8). It is hidden, and every row on it is inert, unless `sd:/ultimate/TrainingRooms/developer.ini` contains `developer_page=on`; the plugin reads that file once at boot (§3, §5.1 step 8). A missing file, an unreadable file, a missing key, a repeated key or any other value means hidden (fail closed). The plugin never creates or rewrites `developer.ini`: the operator creates it, and the operator's card and the Eden test profiles carry it. `developer.ini` is new in the clean room; the fork has no such file and SPEC-core-08 does not import it. The page ships in the normal build (no `diag` build is needed to capture or test it); the playbook's `diag` feature keeps only the Eden stand-ins. Rows, all toggles defaulting to Off, and the effective value of each is *stored value AND page enabled*:
  - **Preload Samples** (moved from *Loading (advanced)*): sparse timing and memory samples for the cycle-preload feature (fork carry-over; the diagnostics file receives them).
  - **Reload developer config**: the upstream `dev_config` behaviour rebuilt from §6 below, now behind the row instead of always on: while On and P1 holds L+R+A, the developer config file is re-read every frame (a file read per frame; the struct has no fields, so the read has no other effect). The upstream bodies are not copied. **Primary decision, 2026-11-03:** the synchronous-read wording is implemented as one counted request per qualifying held-L+R+A P1 frame, processed off-thread in bounded batches by the existing worker. No filesystem I/O or heap allocation occurs in the hook. Report requested/completed/pending/dropped counts and explicit lag/overflow diagnostics; cancel pending work when the row or page is Off. The empty struct has no configuration effect; actual read timing and the count at shutdown may differ. See docs/DECISIONS.md, developer reload processing decision.
  - **Debug hooks**: the upstream `debug_assertions` hooks rebuilt from §6 below (a replacement hook on the game's float-write `WorkModule::set_float`, `0x208db60`, that calls `is_visible_backshield`, `0x1655610`, for Kirby), now installed only when the page is enabled at boot and acting only while the row is On; each site keeps its 64-byte guard and card-scan entry; a site that fails is skipped and logged.
  - The upstream `raygun_printer` (PROVENANCE row 26) is named in `docs/DECISIONS.md` B9 but SPEC-core-01 never described it; no row exists for it until it is described (open question 5).
- Button chords: none. (`L+R+A` reloading a developer config file happens only while the *Reload developer config* row is On, see §6.)
- Visible outputs on a supported game:
  - When the Training HUD first draws, a notification queue shows, in order: header `Training Modpack` / text `Welcome!` for 60 frames; then `Open Menu` / the open chord as text for 120 frames; `Save State` / the save chord; `Load State` / the load chord; `Input Record` / the record chord; `Input Playback` / the playback chord (each 120 frames). Upstream queued the `Welcome!` notification twice (once before and once after the menu load); the clean room shows it once. Notification rendering itself is SPEC-input-* (notifications); the clean room must use its own name in the header, never "Training Modpack".
  - The chord texts are the `ButtonConfig` display form, which joins the selected button labels (for example `Pro ZL; GCC L + DPad Down`). The exact joiner is the `byteflags` display implementation; see SPEC-core-04 §3 for the value model and open question Q9 below.
- Visible outputs on failure:
  - Upstream: a Rust panic anywhere in the plugin opens the system error applet (error code 69) with the text "SSBU Training Modpack has panicked! Please open the details and send a screenshot to the developer, then close the game." followed by the panic message, source location, the game display version and the Atmosphere display version. The clean room keeps a panic hook that writes the same information to the diagnostics file and the Skyline log; whether it also opens the error applet is open question Q1.
  - Upstream, when the stage-hazard code could not be located, opened the error applet (error ids 1000/2000) saying hazard control is disabled for this launch. The clean room logs the skipped site instead (playbook: a site that fails is left alone and the log says which).
  - On an unsupported game version the clean room installs nothing and logs one line (§7). Upstream had no version guard: on another version the first offset search that failed panicked inside the error applet.

## 3. Inputs

- Game identity: the display version string from `nn::oe::GetDisplayVersion` (16-byte NUL-padded buffer; must read `13.0.5`) and, as the fork added, the GNU build-id note in the last page of the main module's `.rodata` (`namesz` 4, `descsz` 0x14, type 3, name `GNU\0`; the 13.0.5 note is the last 0x24 bytes of `.rodata`, at `main+0x4f16f3c`, descriptor `21450c647b8c5940ed5b68218ea128fd3c5d597d`). Both must match; the build id decides, the display version is a second check so a patched or mislabelled main never passes on one field.
- Skyline region addresses of the main module: `.text` start, `.rodata` start, `.data` start (`getRegionAddress(Text|Rodata|Data)`). All offsets in this spec are relative to `.text`.
- Presence of the two provider plugins, found through their exported functions (HOOK-SHARING table): `libnro_hook.nro` provides the NRO-load callback registration (Skyline `nro::add_hook`, export `add_nro_load_hook`, and the unload counterpart); `libparam_hook.nro` provides the parameter-load callback registration (`add_hook(params_main)` in the Skyline smash bindings). Upstream pinned nro-hook v0.4.0 and params-hook v0.1.1 in `[package.metadata.skyline] plugin-dependencies`.
- Game symbols looked up by name with `nn::ro::LookupSymbol`: `_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E` (FighterManager singleton slot), `_ZN3lib9SingletonIN3app12StageManagerEE9instance_E` (StageManager; upstream looked it up but never used it), `_ZN3lib9SingletonIN3app11ItemManagerEE9instance_E` (ItemManager), `_ZN3app9smashball16is_training_modeEv` (the game's "is Training mode" predicate, called every frame by almost every feature), `_ZN3app15sv_fighter_util15set_dead_rumbleEP9lua_State` (hooked; SPEC-state-*).
- Files on the SD card at boot: the menu settings file (SPEC-core-04), the fork's `button_config.ini` (menu shortcuts, SPEC-core-03 §6), the fork's `cycle_preload.ini` (fork carry-over), the new `developer.ini` (draft 9, §2; read once, never written), the development `dev.toml`/`dev.json` (§6), and the legacy folder `sd:/TrainingModpack` (migration, §7).
- The `byteflags`-encoded menu state `MENU` (a `Copy` struct behind a read-write lock) is read by every per-frame hook; the boot sequence publishes it once before any hook can run (§5).

## 4. Outputs and native state touched

Every row is a 13.0.5 `main+0x…` offset. The fork resolved each needle site at the documented offset and compared the bytes there (table-backed needle, `Needle::get`); a site that fails is skipped and logged, never dereferenced. "Form" is the Skyline hook form the fork used; "Group" names the owning spec. The clean room installs a site only when its 64-byte guard passes (playbook), and admits other plugins' hooks as HOOK-SHARING describes.

### 4.1 Guards and reads (no write to game memory)

| Native state | How it is reached | When | Guard |
| --- | --- | --- | --- |
| Display version | `nn::oe::GetDisplayVersion` | boot, first | must equal `13.0.5` |
| Build-id note | scan the last 0x1000 bytes of `.rodata` backwards, 4-aligned, for a GNU build-id note | boot, first | descriptor must equal `MAIN_BUILD_ID` |
| FighterManager / StageManager / ItemManager singleton slots | `LookupSymbol` by the mangled names in §3; the value is the address of a pointer to the manager | boot | a null result leaves every dependent helper returning "none" (fork fix; upstream dereferenced unconditionally) |
| `is_training_mode` | symbol `_ZN3app9smashball16is_training_modeEv`, called | every hook | called function: 64-byte guard, other plugins' hooks allowed |
| `get_battle_object_from_id` | `main+0x3ac560`, called (upstream needle 32 bytes) | on demand | Called row; refusal returns "none" (fork fix) |
| Emulator detection | `.text` region address equals `0x8004000` | boot | informational only (upstream skipped the auto-update check and dialogs on emulators) |

### 4.2 Sites written at boot (hooks, patches, vtable slots)

| Native state | How it is reached (`main+0x…`) | Form / placement | Group | When |
| --- | --- | --- | --- | --- |
| Fighter vtable `on_init` slot | slot `main+0x4f80240` (data, `.text`-relative), original entry `main+0x60dfa0` (needle 36 bytes) | data patch: replace the slot with a wrapper that calls the original then runs the reset check | SPEC-state-* (L+R+A reset) via this spec | boot |
| Fighter vtable `on_frame` slot | slot `main+0x4f80568`, original entry `main+0x614630` (no needle upstream; fixed) | data patch: wrapper calls the original then `once_per_frame_per_fighter` | this spec (per-frame driver) | boot |
| Final input mapping | `main+0x1750dd0` (needle 32 bytes; signature `(mappings*, player_idx, out MappedInputs*, controller_struct*, bool)`) | replace, entry | SPEC-core-03 (menu input), SPEC-input-* | boot |
| Custom-stage tab in Training | `main+0x184dcc8` (needle 20 bytes) | one instruction `nop` patch (stops the Stage Select from clearing the custom-stage flag in Training) | this spec (boot patch; behaviour belongs to SPEC-state-* stage) | boot |
| Stale-move toggle | `main+0x13e88c4` inline (needle 16 bytes), and the menu text at `main+0x13e88c0` inline (needle 16 bytes; sets x1 to the string at `main+0x42b415e`, "mel_training_on") | inline, mid; the two sites overlap, so **STALE_MENU must resolve before STALE is hooked** (upstream comment; the fork resolves both before installing either) | SPEC-state-* (the fork also captured the native pause-menu pointer here; see SPEC-core-05 §6) | boot |
| L+R+A reset check | `main+0x1378e50` inline (needle 32 bytes; clears x8 when the menu says reset off) | inline, mid | SPEC-state-* | boot |
| Layout archive allocation | `main+0x3774704` inline (needle 16 bytes) | inline, mid | SPEC-core-05 | boot |
| UI per-frame pass anchor | `main+0x375efa8` inline (5-word pin; locator 28 bytes starting at site+4) | inline, mid | SPEC-core-05 | boot |
| UI registration anchor | `main+0x3784ad4` inline (5-word pin; locator at site+4) | inline, mid | SPEC-core-05 | boot |
| UI manager global | `adrp`/`ldr` pair at `main+0x375e280` → slot `main+0x593ba90` (`.bss`) | relied-on words (decoded, never patched) | SPEC-core-05 | boot |
| Upstream draw hook (replaced by the two anchors) | `nn::ui2d::Layout` vtable `+0x98` at `main+0x4b620` (needle 32 bytes) | replace, entry (upstream only) | SPEC-core-05 | boot (upstream) |
| Hitbox: `ATTACK` animcmd | `main+0x2284b20` | replace, entry | SPEC-core-06 | boot |
| Hitbox: `CATCH` animcmd | `main+0x22ac290` | replace, entry | SPEC-core-06 | boot |
| Hitbox: `GrabModule::set_rebound` | `main+0x2027bb0` | replace, entry | SPEC-core-06 | boot |
| Stage hazard flag | hook at the `cbz` found by an `adrp; ldrsw; cbz` scan (`main+0x2311bec`), flag byte at `main+0x530a985` (`.bss`) derived from an `adrp; add; ldur; ldrb; ldr; sub` scan at `main+0x17408c4` (`(pc & ~0xfff) + adrp + add + 9`) | raw inline (`A64InlineHook`), mid | SPEC-state-* (hazards) | boot |
| Native pause-menu combo-display observers (fork) | `main+0x13e549c` and `main+0x13e54d8` inline, verified words at `main+0x13e5498` (`396db114 6b14013f 540002c0 34000234`) and `main+0x13e54d4` (`396db108 3908c268 320003e8 3908be68`) | inline, mid | SPEC-core-05 §6 | boot |
| CPU-behaviour hooks (lua_bind thunks, symbol-resolved) | `WorkModule::get_param_float` `main+0x208de50`, `get_param_int` `0x208de30`, `ControlModule::get_attack_air_kind` `0x2001710`, `get_command_flag_cat` `0x2001630`, `get_stick_x_no_clamp` `0x2001600`, `get_stick_y_no_clamp` `0x2001610`, `get_stick_x` `0x2001260`, `get_stick_dir` `0x2001030`, `get_stick_y` `0x2001280`, `check_button_on` `0x2001430`, `check_button_off` `0x2001440`, `MotionModule::change_motion` `0x205cb30`, `WorkModule::is_enable_transition_term` `0x208dd60`, `set_dead_rumble` `0x2281a20`, plus `CameraModule::req_quake`, `EffectModule::req_screen`/`req`/`req_follow`/`req_on_joint`, `SoundModule::play_se`, `ArticleModule::get_int`, `FighterSpecializer_Jack::check_doyle_summon_dispatch`, the SDI hit-stop delay command and the clatter start (addresses in the owning specs) | replace, entry; the 16-byte thunks sit inside each other's 64-byte windows, so **check every window before installing any hook** (fork lesson) | SPEC-cpu-*, SPEC-state-* | boot |
| Needle-located feature hooks | Cloud limit `main+0x8dc160`, add-damage `0x3ff9c0`, fighter `play_se` `0x4cf6c0`, star-KO `can_futtobi_back` `0x2610a40`, reused UI (Little Mac meter) `0x68cda0`; development-only `is_visible_backshield` `0x1655610`, `WorkModule::set_float` `0x208db60`; unused upstream needles `copy_setup` `0xba0e80`, `set_cpu_controls` `0x2da180`, `change_active_camera` `0x4ee480`, `set_training_fixed_camera_values` `0x3158e00`, `opcf` `0x6b7ffc`, `kirby_opff` `0xb971d0`, `activate_autonomy` `0x34b6cd0`, `pokemon_decide` `0x34cec44` | replace (hooks) or called | owning specs | boot |
| lua2cpp `common` NRO hooks | installed from the NRO-load callback when the module named `common` loads (skip if `isLoaded` already): `sub_guard_cont`, `correct_damage_vector_common`, `change_status` (symbols in that NRO) | replace, entry, in an image other than main | SPEC-cpu-*, SPEC-state-* | at `common` NRO load (every scene that loads fighters) |
| Common parameters pointer | the param-load callback receives the `CommonParams` table and stores its address | provider callback | SPEC-cpu-* (shield) | at param load |

### 4.3 Files and threads

| Output | Where | When |
| --- | --- | --- |
| Data folder | upstream `sd:/ultimate/TrainingModpack`; clean room `sd:/ultimate/TrainingRooms/` (README) | boot, `create_dir_all` |
| Legacy migration | upstream moved every entry of `sd:/TrainingModpack` into the data folder and removed the old folder | boot (Q2 for the clean room) |
| Install line | one line to `events.log` in the data folder and to the Skyline log | end of boot |
| Diagnostics file | fork: `practice-diagnostics.log` + `.previous.log`, 128 KiB each, written by a 50 ms drain thread (§7) | boot onward |
| Menu loader thread | 128 KiB (`0x20000`) stack, joined before boot continues (upstream comment: the default stack crashes in the JSON parser without a useful message) | boot |
| Config writer thread | upstream: permanent, default 2 MiB stack; fork: 64 KiB named `config-writer` | first write |
| Heartbeat thread (fork) | `practice-heartbeat`, 64 KiB stack: drains diagnostics every 50 ms, releases parked layout archives every 5 s (SPEC-core-05) | boot |
| Upstream analytics thread | `events_loop`, permanent, 2 MiB default stack — **dropped** | — |
| Upstream version-check thread | 512 KiB (`0x80000`) stack, joined — **dropped** | — |

## 5. Timing and frame rules

### 5.1 Boot order (required)

Skyline loads every plugin's NRO before running any plugin `main`, so provider exports and other plugins' hooks are visible from the first instruction of `main`.

1. Open the diagnostics file (rotate the previous one), write the session-start line.
2. **Version guard** (§3). On failure: log `result=unsupported_version reason=<display_version|build_id|build_id_missing>`, flush, return. Nothing below runs.
   2a. (draft 9) Read `developer.ini` once (one read capped at 8 KiB; parse rule as the shortcuts INI: `#` / `;` comments, case-insensitive key and value, fail closed) and publish the `developer_page` flag. It is read here, before step 5, because the *Debug hooks* site is installed only when the flag is on. The install line carries `developer_page=on` or `developer_page=off`.
3. Install the panic hook and the `log` facade (upstream level cap: Info; prefix `[TrainingModpack] [LEVEL]` with ANSI colours — the clean room uses its own prefix).
4. Queue the welcome notification.
5. Hitbox visualizer hooks (SPEC-core-06), then stage-hazard hook (SPEC-state-*), then the training group:
   a. symbol lookups (§3), param-hook registration (a missing provider is logged and leaves only the shield parameter rewrite off; fork behaviour),
   b. resolve the two stale-move needles in order STALE_MENU then STALE, then **check every per-site window** (all training rows) before installing any,
   c. custom-stage `nop` patch, vtable slot patches, the training hooks of §4.2, the EffectModule chain hooks (HOOK-SHARING "chain": another plugin's far jump at the entry is called as the original),
   d. feature `init`s in this order: items, input recording, UI (SPEC-core-05: native-menu observers, UI anchors, layout-archive hook), Pikmin, Pokémon Trainer, Kirby, tech, then force the config writer thread.
6. Register the NRO load (and unload) callbacks with the nro-hook provider.
7. Create the data folder; migrate the legacy folder (Q2).
8. Load the menu shortcuts INI (fork; SPEC-core-03 §6), then the menu file on the loader thread (SPEC-core-04) and join it; this publishes `MENU` and `DEFAULTS_MENU` before the first Training frame.
9. Queue the five chord notifications (§2).
10. Write the install line (§7) with every site's outcome.

Upstream order differed in two ways that the clean room must not copy: it registered the NRO hook after the training hooks but *before* creating the folder (fine), and it ran the auto-update check (network, dialogs, possible relaunch) between the menu load and the notifications.

### 5.2 Per-frame entry points

- **Per fighter, per frame** (`on_frame` vtable wrapper, after the game's own per-frame work for that fighter): shield parameter installer (must run outside Training too, to restore the shield damage multiplier: `077a8e43` comment); then, **only in Training mode** (`077a8e43` fix): poll the open-menu chord request and spawn the menu if set; for the CPU fighter: dodge-staling flag, input recording tick, in-game frame counters tick, tech hide; then combo, hitbox visualizer once-per-frame, save states, tech, clatter; then fast fall, ledge option, shield, DI. The order matters for features that read each other's state; the owning specs say what each step needs.
- **Per fighter, on init** (`on_init` wrapper): the reset check — `077a8e43` changed this to run only when `is_training_mode()` is **false**; see Q3.
- **Per controller, per frame** (final input mapping hook): after the game's mapping; returns immediately outside Training; then developer-config reload (only while the Developer page is enabled and its row is On, §2), menu input, chord requests, input delay, input log, input recording (SPEC-core-03 §5 for the order and why it matters).
- **Per layout, per frame** (UI anchors; SPEC-core-05): the UI tick.
- **Per NRO load** (`nro_main`): only the `common` module installs anything.
- Non-Training scenes: every hook calls the original and returns; the only standing work is the shield parameter installer and the hooks' `is_training_mode()` calls. The fork widened "Training" to "Training or lobby practice active, and no hot swap in progress" (`is_training_mode` wrapper: `busy()` → false; else native predicate or `lobby::active()`); carry that wrapper over as the seam every feature calls.

## 6. Edge cases and interactions

- **Missing `libnro_hook.nro`:** upstream and fork panic (`expect("Did not find nro_hook plugin!")`), which opens the error applet. Playbook rule: a missing provider is a logged skip. Recommendation: log `provider=libnro_hook result=missing`, skip the three `common` hooks and the unload callback, and run with the shield/DI/tech features that need them off; show one notification. Q4.
- **Missing `libparam_hook.nro`:** fork logs `provider=libparam_hook result=skipped reason=plugin_missing` and continues (the shield parameter installer null-checks the parameter pointer). Required behaviour.
- **Another plugin on the same site:** HOOK-SHARING rules. Known coexistence facts the fork measured: Deluxe ≥ 0.3.1 function-hooks the UI pass entry `main+0x375e260` with a 16-byte far-jump stub; the fork therefore never requires the entry words and locates the pass by a needle starting 0x14 bytes in (REVISION30). The two anchor sites, the layout-archive site and the fighter thunks were uncontested on the operator's card as of 2026-10-31; re-run the card scan before choosing sites.
- **Overlapping thunks:** the lua_bind thunks hooked at entry are 16 bytes apart (`0x2001600`/`0x2001610`, `0x2001430`/`0x2001440`, `req_quake`/`req_quake_pos`); a 64-byte check made after a neighbour's far jump would read as foreign. Check all windows first, then install.
- **Stale pair:** `0x13e88c0` and `0x13e88c4` overlap by 12 bytes; install STALE_MENU first (its needle still matches after STALE is hooked? No: the STALE needle covers the STALE_MENU site once STALE_MENU is hooked, so **resolve both before hooking either**, then install STALE_MENU, then STALE).
- **Unsupported version:** nothing installs; the game runs unmodified. Upstream panicked in the first failing needle (user-visible error applet, game unusable).
- **Fighters unloaded under a running hook:** background matchmaking unloads fighters while the UI still draws; upstream's distance and damage-digit helpers `expect`ed a module accessor and panicked. Fork fix (fold in): every accessor lookup null-checks the manager pointer, the entry, the accessor, and checks the accessor's object id and entry id round-trip; callers get "none" / `f32::MAX` / zero digits.
- **Panics:** upstream aborted the game through the error applet on any plugin panic. The clean room's `panic = "abort"` profile is the same; the hook must flush the diagnostics first. Q1.
- **Emulator:** upstream skipped the network version check and auto-answered dialogs when the `.text` base was `0x8004000`. With auto-update dropped nothing depends on it; keep the predicate only for diagnostics.
- **Developer config (`dev_config`):** a `DevConfig` struct with no fields, loaded from `dev.toml` (fork: `dev.json`), re-read **every frame while P1 holds L+R+A** (a file read per frame). Development aid only. **Decided 2026-11-01 (B9 and menu question 8): it ships as the *Reload developer config* row of the hidden Developer page** (§2), off by default, instead of the `diag` feature or being dropped. Q5.
- **Debug hooks (`training/debug.rs`):** behind `debug_assertions`: a replace hook on `WorkModule::set_float` (`0x208db60`) calling `is_visible_backshield` (`0x1655610`) for Kirby, and commented-out hooks on `on_flag`, `set_int`, `set_int64`, `is_flag`, `get_int`. **Decided 2026-11-01 (B9): the set_float / backshield hook ships as the *Debug hooks* row of the Developer page** (§2); the commented-out hooks are not implemented.
- **Dropped upstream behaviour (one paragraph each):**
  - *Analytics (`events.rs`, `training_mod_metrics`).* At boot upstream queued a `SMASH_OPEN` event and, at every menu close, a `MENU_OPEN` event carrying the full menu JSON; a permanent thread posted the queue every 10 s to a Firebase realtime database, keyed by a SHA-256 of the console's pseudo device id and the user id of the last opened account, plus a per-session hash, the mod version and the game version. `training_mod_metrics` was a desktop program (datafusion + plotters) that charted unique devices and sessions per day from an export of that database. Not wanted: it sends device and account identifiers off the console without consent, costs a 2 MiB thread stack from the game heap (REVISION33), and pulls `minreq`/`native-tls` into the image.
  - *Auto-update (`release.rs`, `config.rs` updater section).* At every console boot (skipped on emulators) a 512 KiB thread fetched the GitHub releases list of the upstream repository, compared the newest stable or pre-release `published_at` (RFC 3339 string order) with `last_update_version` in `training_modpack.toml`, asked the player through a system dialog, downloaded the release zip, extracted it over `sd:/`, rewrote the TOML and relaunched the game. Not wanted: it would offer to install the upstream build over this project, it needs network and TLS in the plugin, and it writes arbitrary files to the SD root. The `training_modpack.toml`/`.json` config file existed only for this; see SPEC-core-04 Q3.
  - *Dialogs (`dialog.rs`).* Thin wrappers over the `skyline-web` dialog applet (yes/no, no/yes, ok/cancel, left/right, ok) that returned a fixed answer on emulators. Only the updater used them. Not wanted: the applet blocks the game, and the only caller is gone.
- **Hazard dialog:** upstream showed an error applet when the hazard scans failed. Clean room: log and skip (playbook).
- **Lock discipline (`training_mod_sync`):** (decided 2026-11-01, B9: the helper crate is not kept; the code uses `parking_lot` directly) upstream read the whole menu struct by copy on every read (`read(&MENU)`, a `Copy` struct of a few hundred bytes) and used `RwLock` guards that panic when poisoned. Any hook that holds a lock while calling into the game risks a deadlock with the UI thread; the fork's rule "atomics only while the menu app lock is held" is the requirement.

## 7. Persistence

- Data folder: `sd:/ultimate/TrainingRooms/` (README). Created at boot. Upstream used `sd:/ultimate/TrainingModpack` and migrated `sd:/TrainingModpack` into it; whether the clean room reads or migrates a user's existing `TrainingModpack` folder is Q2.
- Install line: `sd:/ultimate/TrainingRooms/events.log`, one line per boot, also echoed to the Skyline log, in the playbook form: `<tick> plugin=<name>/<version> game=13.0.5 build=<id> result=installed|unsupported_version reason=<…> <site>=<entry>/<outcome> … heap_allocations=<n|unmetered>`. The fork's line is `[TrainingModpack] result=installed version=13.0.5 build=…` followed by every checked site's outcome, with `needle=<name> result=skipped reason=absent|ambiguous`, `seam=<label> result=skipped`, `provider=<name> result=skipped reason=plugin_missing` as the per-site vocabulary.
- Diagnostics file (fork, carry over as the documented exception to "a shipped plugin writes only its install line"): `practice-diagnostics.log` and `practice-diagnostics.previous.log` in the data folder, at most 128 KiB each, rotated at the limit; lines are `<unix ms> #<seq> <text>\n`; a shipped build writes only the install line, boot and panic breadcrumbs and sparse lifecycle events (an allow-list of leading `[Tag]`s); a `diag` build writes every line. Lines are formatted without heap allocation into a 512-byte stack buffer, pushed into a fixed ring (32 KiB shipped, 128 KiB diag; line cap 9 KiB / 12 KiB) under a short spin lock, and drained by the heartbeat thread every 50 ms with one `sync` per drain; a full ring drops the line and the file later says `ring_dropped=<n>`. Until the drain thread runs, or if it failed to start, callers write synchronously. The version guard and the panic hook flush before returning.
- Settings files: SPEC-core-04. Shortcut INI: SPEC-core-03 §6 and SPEC-input-04 §7.
- `sd:/ultimate/TrainingRooms/developer.ini` (draft 9): one key, `developer_page=on`; read once at boot (§5.1 step 2a), never created, never rewritten, never imported (SPEC-core-08). It is the operator's switch for the Developer page.
- Malformed or missing files never stop the boot; each loader falls back to its defaults and logs.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| NRO image (whole plugin, release) | 2,174,976 B at `opt-level="z"`, `codegen-units=1`, fat LTO (REVISION32; `.text` 1,171,456); the same sources at opt-level 3 were 3,379,200 B, at `"s"` 2,621,440 B, at `"s"`+1 CGU 2,646,016 B; the earlier 32w release was 2,494,464 B | see `docs/resource-costs.md` (W5) | `tools/build/nro_size.py` per build |
| Threads and stacks | heartbeat 64 KiB, config writer 64 KiB (upstream default 2 MiB), menu loader 128 KiB (joined); upstream analytics 2 MiB and updater 512 KiB removed (REVISION33) | no permanent thread with a default stack; at most the writer/heartbeat pair | thread list in the install line (diag) |
| Boot needle searches | one `memmem` forward and one backward over `.text` (about 57 MiB each) per needle without a known offset; the fork avoids the search when the bytes at the documented offset match | zero full-text searches on 13.0.5 (documented offsets + 64-byte guard) | diag timing line at boot |
| Per-frame work of this spec | the `on_frame` wrapper dispatch and one `is_training_mode()` call per hook | unchanged | diag counters |
| Heap allocations in hooks | the operator ruled the playbook's no-heap rule not applicable to the fork (menus, saves, Lua VM allocate by design; `heap_allocations=<calls>` is reported for build-to-build comparison) | `heap_allocations` reported; 0 in the per-frame hooks of this spec | counting allocator in the diag build, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Version guard accepts 13.0.5 display + build id, refuses other display, other build, missing note | `core01_verdict_matrix` (model) | `core01-E1` boot on 13.0.5 → `result=installed` | `docs/EDEN-RESULTS.md` |
| Build-id note finder returns the last 4-aligned GNU note in a `.rodata` tail | `core01_build_id_scan_tail` | — | — |
| Unsupported version installs nothing and logs one line | `core01_unsupported_logs_and_returns` (model: install plan empty) | `core01-E2` patched display version → `result=unsupported_version`, game unmodified | `docs/EDEN-RESULTS.md` |
| Every §4.2 site has a 64-byte guard and a card-scan entry; overlapping thunks checked before any install | `core01_site_table_guards_present`, `core01_overlapping_windows_checked_first` | `core01-E3` install line lists every site `=stock/near` | `docs/EDEN-RESULTS.md` |
| Stale pair resolved before either is hooked, STALE_MENU installed first | `core01_stale_pair_order` | `core01-E3` | — |
| Missing nro-hook / param-hook providers → logged skip, no panic | `core01_provider_missing_plan` | `core01-E4` profile without each provider | `docs/EDEN-RESULTS.md` |
| Another plugin's hook on a shared site (inline stand-in, far stand-in) | `core01_chain_entry_classification` | `core01-E5` `foreign=inline` / `foreign=far` diag drill | `docs/SAFETY.md` |
| Welcome and chord notifications queued once each with the stated lengths | `core01_boot_notifications` | `core01-E6` screenshot sequence on first Training HUD | `docs/EDEN-RESULTS.md` |
| Data folder created; legacy migration per Q2 | `core01_paths` | `core01-E7` fresh SD profile | — |
| Install line format and `events.log` location | `core01_install_line_format` | `core01-E1` | — |
| Diagnostics ring: admission list, drop counting, rotation at 128 KiB | `core01_diag_ring_admission`, `core01_diag_ring_rotation` | `core01-E8` file sizes after a long session | — |
| Per-frame driver order (§5.2) and Training-only gating (`077a8e43`) | `core01_once_per_frame_order` (model sequence) | `core01-E9` Quickplay match: no hitbox/reset side effects | `docs/EDEN-RESULTS.md` |
| Fighters unloaded by matchmaking → helpers return none, no panic | `core01_accessor_lookup_none_paths` | `core01-E10` background matchmaking from Training | — |
| No network, no dialog, no file outside the data folder (dropped features stay dropped) | `core01_no_network_symbols` (link-time: no TLS/HTTP crates) | `core01-E11` Eden network log empty | — |
| **N1** `developer.ini` gate: only `developer_page=on` shows the page; missing, unreadable, > 8 KiB, missing key, repeated key or any other value hides it; read once at boot; never created or rewritten | `core01_developer_ini_gate`, `core01_developer_ini_never_written` | `core01-E12` three profiles (no file, `developer_page=off`, `developer_page=on`): the page is absent, absent, present; the file's bytes and mtime unchanged after a session | `docs/EDEN-RESULTS.md` |
| **N1** Developer rows are inert while the page is hidden (effective value = stored AND enabled), even when the settings file stores On | `core01_developer_rows_inert_when_hidden` | `core01-E13` copy the operator's settings to a profile without `developer.ini`: Preload Samples writes no samples | `docs/EDEN-RESULTS.md` |
| **N1** *Reload developer config* reads the file every frame only while the row is On and P1 holds L+R+A; Off → no file read | `core01_dev_config_reload_gate` (model: read count per frame) | `core01-E14` diag file-read counter with the row On / Off | `docs/EDEN-RESULTS.md` |
| **N1** *Debug hooks* site installs only when the page is enabled at boot, behind its 64-byte guard; a failed guard is a logged skip; the install line says `developer_page=on` or `developer_page=off` | `core01_debug_hook_site_gated`, `core01_install_line_developer_flag` | `core01-E15` install line on the `on` and `off` profiles | `docs/EDEN-RESULTS.md` |

## 10. Open questions

1. **Panic policy.** Upstream opened the error applet on every panic (game unusable until closed). Playbook plugins log and skip. Recommendation: panic hook writes to the diagnostics file and Skyline log and then aborts as the profile dictates; no applet. Operator decision. **Decided 2026-11-01 (B2): as recommended;** log it, no pop-up.
2. **Migration of existing user data.** Should the clean room read `sd:/ultimate/TrainingModpack/*` (menu file, save states, recordings, INIs) on first boot and copy them into `sd:/ultimate/TrainingRooms/`? Upstream only migrated the much older `sd:/TrainingModpack` root. Recommendation: copy (never move) known file names once, log each, and never touch the old folder again. **Decided earlier (2026-11-01) by SPEC-core-08:** a one-time copy, originals untouched; its own details were answered with S1 on 2026-11-01.
3. **`077a8e43` reset-check gating.** The newest upstream runs the L+R+A reset check only when **not** in Training mode, which reads as inverted (the reset check is a Training feature). The commit title is "Add more training mode checks". Recommendation: SPEC-state-* decides; this spec records that the fork (`97cbe9b9` base) runs it unconditionally and that the sign of the new guard needs a test, not a copy.
4. **Missing nro-hook provider.** Panic (upstream/fork) versus logged degraded mode. Recommendation: degraded mode with one notification.
5. **Developer config and debug hooks.** Keep `dev_config` (per-frame file read while L+R+A held) and the `debug_assertions` hooks out of the shipped build; include behind `diag` only if a developer asks. Recommendation: drop both. **Decided 2026-11-01 (B9, with menu question 8): neither dropped nor `diag`-only; both ship as rows of the hidden Developer page** (§2: *Reload developer config*, *Debug hooks*; gate `developer.ini`, N1), and `training_mod_sync` is not kept (`parking_lot` is used directly). `raygun_printer` (named in `docs/DECISIONS.md` B9, `PROVENANCE.md` row 26) is not described anywhere in this spec: no row until it is described.
6. **Plugin name and Skyline main name.** Upstream `#[skyline::main(name = "training_modpack")]`, NRO `libtraining_modpack.nro`. The clean room needs its own name (`training_rooms`?) so both can be distinguished on a card; they must never be installed together (same sites). **Decided 2026-11-01 (B1): `training_rooms`** (`libtraining_rooms.nro`).
7. **Stale-move "on" text pointer** at `main+0x42b415e` is a fixed `.rodata` address with no needle (upstream TODO). It needs a guard (read the string bytes) or a different way to force "On" in the native menu text. Owning spec SPEC-state-*; listed here because it is the only fixed data address upstream patched a register with.
8. **`StageManager` symbol lookup** is unused upstream ("This seems unused?"). Recommendation: drop.
9. **Chord text in notifications** depends on the `byteflags` display joiner (`ButtonConfig::to_string`). The clean room's own encoding (SPEC-core-04 Q1) decides the text; confirm the joiner string from the upstream README screenshots or choose ` + `.
10. **Menu read cost.** `read(&MENU)` copies the whole settings struct per call, dozens of times per frame. Recommendation: one immutable snapshot per frame published at the frame start, or atomics per field; measure with the diag counters first.
