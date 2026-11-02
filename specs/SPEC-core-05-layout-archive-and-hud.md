# Layout archive injection, UI tick entry and HUD toggle — behavioural specification

- **Spec id:** `SPEC-core-05`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), spec-writer role, 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/ui/mod.rs` (draw hook, archive hook, helpers), `src/training/ui/display.rs` (notification panel draw and the native combo-display toggle), `src/common/mod.rs` (the native pause-menu structure), `src/common/offsets.rs` (`DRAW`, `LAYOUT_ARC_MALLOC`), `Cargo.toml` (`layout_arc_from_file`, `include-flate`, `sarc`) at commit `97cbe9b9` (unchanged in `077a8e43`); fork changes at stage M5i `src/training/ui/mod.rs` `7-8, 11, 14-16, 18, 21-23, 25, 27-102, 138-152, 154-182, 185-186, 195, 199-201, …`, `src/training/ui/display.rs` `28`, plus the fork-only `src/training/ui/{anchors,layout_registry,archive_name,archive_retirement,waiting_archive,native_menu}.rs`, `training_mod_macros/src/ui_anchor.rs`, `src/hang_trace.rs`, `docs/REVISION29_DRAW_DECOUPLING.md`, `docs/REVISION30_HOOKPROOF_PROLOGUE.md`, `docs/REVISION33_STARTUP_MEMORY.md`, from `docs/provenance/provenance-files.md`.
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

The plugin's on-screen elements (menu, notifications, input log, damage readouts) live in panes added to the game's own Training HUD layout. To get them there, the plugin substitutes its own version of the Training layout archive at the moment the game decompresses the original, so the game builds the plugin's panes as if they were native. Every frame, at the point where the game is about to recompute each layout, the plugin runs its UI work for that layout; for the Training HUD that means drawing the menu and panels, and for the game's HUD layouts it means hiding them when the player turned the HUD off or the menu is open. The fork's REVISION33 change, which releases the decompressed archive a few seconds after the game copied it instead of keeping 4.2 MB resident for the whole session, is required behaviour.

## 2. User-visible behaviour

- Menu item **HUD** (`hud`, Misc tab, On/Off, default On): Off hides the game's Training HUD overlays — player percentages/names (`info_melee`), the player cursors and the item cursor (`info_playercursor`, `info_playercursor_item`), the off-screen radars (`info_radar_a`, `info_radar_b`) — while a match is running. The plugin's own panels (menu, notifications, input log, frame advantage) are unaffected by this setting.
- The game HUD is also hidden while the plugin menu is open, whatever the setting.
- The native Training pause-menu option "combo display" (the game's own setting) controls whether the plugin's notifications appear: when the player turns the game's combo display off, queued notifications are discarded and the panel is hidden (SPEC-input-* notifications owns the panel; this spec owns how the native setting is read).
- Nothing else is visible: the archive substitution is invisible when it works. When it does not (archive mismatch, guard failure) the Training HUD is the stock one and the plugin's panels do not exist; the plugin must then not try to draw them (upstream panicked on a missing pane: "Could not find pane …").

## 3. Inputs

- The decompressed layout archive the game is about to copy: pointer in register x21, byte size in x1 (also held in x23 and x24) at the hook site inside the UI holder constructor (§4). The archive is a SARC (magic `SARC`, byte-order mark at +6, header size at +4, file size at +8, data offset at +12; `SFAT` node table with `count` 16-byte nodes: name hash, attributes with a name-offset in the low 24 bits when bit 24 is set, data start, data end; `SFNT` name table of NUL-terminated 4-byte-aligned names).
- Which archive it is: the plugin inspects only the archive's **directory** (never the payloads) for the member name `blyt/info_training.bflyt` (fork: also `blyt/info_waiting.bflyt` for the waiting room). Upstream parsed the whole archive with a SARC library, cloning every texture and animation payload just to answer that question (fork fix: borrowed directory lookup; fold in).
- `is_training_mode()` (SPEC-core-01) at the hook; `is_ready_go()` (the FighterManager's "match is running" predicate) for the HUD rule.
- The `hud` setting and the "menu active" flag.
- Each layout's name (`Layout::layout_name`, a C string) and root pane at the UI tick.
- The native pause-menu combo-display byte: upstream read it from a retained pointer to the native pause-menu object (captured as x22 at the stale-move inline hook, structure `PauseMenu`: `+0xb60` stale-move toggle u32, `+0xb6c` combo-display toggle u32); the fork reads the freshly loaded value at two inline sites instead (§4.1, fold in).

## 4. Outputs and native state touched

### 4.1 Sites

| Native state | How it is reached (`main+0x…`, 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| Layout archive buffer pointer and size | inline hook at `main+0x3774704`, inside the UI holder constructor `main+0x3773770`, immediately after the decompression call (`bl`), where x21 = decompressed bytes, x1 = size (copies in x23, x24). The hook sets x21 to the plugin's archive and x1/x23/x24 to its size; the game then allocates that size and copies from that pointer (REVISION33: "the game allocates the size the hook reports and then copies from the pointer the hook reports"). Upstream needle 16 bytes starting at the site: `e3e606 94 / a00500 b4 / e10315 aa / e20317 aa` (bl; cbz x0; mov x1,x21; mov x2,x23) | every layout archive load (only Training's is replaced) | 64-byte guard (fork row `needle.layout_arc_malloc`, inline mid) |
| Per-layout UI tick (fork, replaces upstream draw hook) | inline anchor at `main+0x375efa8` inside the UI manager's per-frame pass `main+0x375e260`: the holder is in x23; the site's five words are `f9422ee0 f9400008 f9404d08 aa1303e1 2a1f03e2` (`ldr x0,[x23,#0x458]`; `ldr x8,[x0]`; `ldr x8,[x8,#0x98]`; `mov x1,x19`; `mov w2,wzr`), followed by `blr x8` (the layout's virtual Calculate). Locator needle (28 bytes) starts at site+4 and is unique in `.text`. | once per live layout per frame, on the UI job thread (worker 1) | 5-word pin inside the 64-byte guard; locator must resolve exactly at `main+0x375efac` |
| Registration UI tick (fork) | inline anchor at `main+0x3784ad4` inside the manager insert `main+0x37849c0`: holder in x21; words `f9422ea0 f9400008 f9404d08 320003e2 aa1303e1` then `blr x8`; locator (28 bytes) at site+4 | once per newly registered layout that asks for an initial calculate, on the main thread | as above; locator exactly at `main+0x3784ad8` |
| UI manager global (fork, read only) | `adrp x28, …; ldr x8,[x28,#0xa90]` at `main+0x375e280` (words `b0010efc f9454b88`) decode to the slot `main+0x593ba90` (`.bss`); the manager is `**slot`; its holder registry is a vector `[begin, end)` of holder pointers at manager+0x0/+0x8, sorted by priority (holder+0x64c); holder+0x458 is the holder's `nn::ui2d::Layout*`; the pass prologue `main+0x375e260` is **reported, not required** (Deluxe ≥ 0.3.1 function-hooks it with a 16-byte stub); the fork locates the pass with a 24-byte needle starting 0x14 bytes into it (`stp x20,x19 / stp x29,x30 / add x29,sp,#0x90 / adrp / ldr / ldr x9,[x8]`) | boot (decode), used from the anchors | relied-on words (64-byte guard, row `training.ui_manager_global`) |
| Upstream draw hook (not to be used) | replace hook at `main+0x4b620`, the `nn::ui2d::Layout` vtable `+0x98` entry (vtable address point `main+0x4f3f280`; also present in `LayoutEx` `main+0x4f3f568` and the Smash subclass vtable `main+0x523f6f0`). On 13.0.5 this is the layout's **Calculate**, not its draw; its third argument is a `bool force`, not a command buffer. Called virtually from exactly the two sites above. | — | superseded; another plugin may still replace it (§6) |
| Game HUD layouts' root pane | for layouts named `info_playercursor`, `info_playercursor_item`, `info_melee`, `info_radar_a`, `info_radar_b`: set root pane flag `InfluencedAlpha` (bit `1 << PaneFlag::InfluencedAlpha`) and `visible = hud == On && !menu_active` (fork adds: and no replay panel / reload in progress) — only when Training and `is_ready_go()` | every tick of those layouts | — |
| `info_training` root pane and children | `frame_counter::tick_real()`, input log draw, notification draw, menu draw (SPEC-core-03) | every tick of `info_training` (in native Training only; the fork also hosts it in `info_waiting` under `TrModPort` when lobby practice is active) | panes must exist (archive injected) |
| Damage panes in `info_melee` etc. | `damage::draw` (SPEC-input-*/SPEC-cpu-* frame advantage/damage display) | every tick, all layouts (name-gated inside) | — |
| Native pause-menu combo-display (fork, fold in) | inline observers at `main+0x13e549c` (value in x20) and `main+0x13e54d8` (value in x8), right after the game's `ldrb` of `PauseMenu+0xb6c`; verified words `396db114 6b14013f 540002c0 34000234` at `main+0x13e5498` and `396db108 3908c268 320003e8 3908be68` at `main+0x13e54d4`; the copied byte (0/1) is kept in an atomic, no pointer retained; defaults to "enabled" when the observers are not installed | native Training only | 64-byte guards (rows `training.native_menu_update_load`, `training.observe_update`, `training.native_menu_initial_load`, `training.observe_initial_update`), all-or-nothing |
| Upstream `TRAINING_MENU_ADDR` (not to be used) | the stale-move inline hook (`main+0x13e88c4`) stored x22 as a pointer to the native pause-menu object and `display::draw` dereferenced `+0xb6c` every frame | — | retained native pointer across scenes: replaced by the observers above |

### 4.2 The injected archive

- Upstream shipped `src/static/layout.arc` (4,213,268 B, SARC) embedded DEFLATE-compressed in the image (`include-flate`), inflated lazily on first use into a static buffer that stayed resident for the whole session. A development feature (`layout_arc_from_file`) instead read `sd:/ultimate/TrainingModpack/layout.arc` into a 5 MiB static at every Training load (so a layout artist can iterate without rebuilding).
- The archive is the game's stock `info_training` layout archive with additions: five new layout files `blyt/training_mod_menu_button.bflyt`, `blyt/training_mod_menu_button_row.bflyt`, `blyt/training_mod_menu_slider.bflyt`, `blyt/training_mod_input_icon.bflyt`, `blyt/training_mod_input_log.bflyt`; Parts panes added to `blyt/info_training.bflyt` referencing them by file stem; new pictures in the combined texture `timg/__Combined.bntx`; the stock animations untouched (upstream note: panes copied from stock parts must be **renamed**, or the stock animations keep overwriting their colours). The full member list and the pane inventory are in SPEC-core-07.
- Fork (REVISION33, required): the archive stays compressed in the image (both fork archives together about 195 KB deflated), is inflated **per load** into a heap buffer, handed to the game with a timestamp, parked, and **released by the heartbeat thread once it is at least 5 s old** (`[LayoutArchive] handed out info_training bytes=…` / `released … after …ms`). Rationale: the game copies the buffer during the hook's own call sequence and never reads the plugin's pointer again. A development feature overwrites the buffer with `0xA5` before freeing so a stale reader would show visibly. Retirement takes the queue lock only to remove entries (try-lock, bounded to the queue length at entry), and frees payloads after unlocking, so a concurrent hand-out never waits.
- Fork (own, carry over): the waiting-room archive is reconstructed per load from the game's stock `info_waiting` archive plus the inflated Training archive plus a 92,398-byte recipe (`TMW1`: lengths and FNV-1a fingerprints of both inputs and of the output, then copy/literal operations; output 8,698,500 B, max 9 MiB; mismatching inputs refuse). The fork's Training archive on disk is 7,820,940 B (roster grid, stage grid and item icons added after REVISION33).

### 4.3 Threads

| Thread | Role |
| --- | --- |
| UI job (game worker 1) | per-frame pass: the UI tick runs here for every layout; an inline trampoline adds about 0x310 bytes (`InlineCtx`) plus the tick's own frame |
| Main thread | layout registration: the tick runs once for a newly inserted layout |
| Heartbeat (plugin, 64 KiB stack) | releases parked archives every 5 s (SPEC-core-01 §4.3) |

## 5. Timing and frame rules

- **Archive load**: happens on the loading screen of a scene that builds the Training UI (Training entry; re-entry after leaving; the fork also on waiting-room entry). The hook must return quickly: upstream's full SARC parse plus a 4.2 MB inflate cost "a few tens of milliseconds" (REVISION33); the clean room inspects the directory only and inflates once.
- **Per-layout tick position**: after the holder's own animate and animation-list update, immediately **before** the layout's virtual Calculate for that holder, once per holder per frame in registry (priority) order. This position matters: the plugin's pane writes override the values the animations just wrote for that frame (`damage::draw` forces digit alphas, the HUD rule sets visibility, the menu colours native-derived panes). A tick before the whole loop would be overwritten by animate; a tick after the loop would land one frame late (REVISION29 §4).
- Registration tick: the same work once when a layout is inserted and asks for an initial calculate; the fork invalidates the input-log pane cache for `info_training`/`info_waiting` there.
- Frame counter: `tick_real()` advances the "real frame" counters (menu close counter, SPEC-core-03) once per `info_training` tick.
- Order inside the tick: (fork) lobby update, keyboard service; waiting-host readiness; HUD rule; `damage::draw`; if `info_training` in native Training (or the waiting host when lobby practice is active): `tick_real`, input log, notifications panel, menu; then the replay-capture after-draw (fork).
- Setting change: `hud` takes effect at the next tick of each HUD layout after the menu commits (the HUD is hidden while the menu is open anyway).

## 6. Edge cases and interactions

- **Archive not injected** (guard failed, foreign archive, memory): the Training HUD is stock; `TrModMenu` and the other panes do not exist. Required: the tick must check that the plugin's root pane exists (or that the injection succeeded for this layout) and skip drawing; never `expect` a pane (upstream panicked). The menu must also refuse to open (SPEC-core-03) when its panes are absent, or open as text-only — Q1.
- **Another plugin replacing `main+0x4b620`** (the old draw hook) or hooking the UI pass entry: unaffected; the anchors sit inside the callers. Another plugin inline-hooking one of the two anchor words: HOOK-SHARING `branch`/`chain`/`foreign` rules; the fork reports `site_owned_by_another_plugin` when only the first word differs and skips both anchors together (menu not drawn, game runs).
- **Deluxe 0.3.1+** hooks the pass entry `main+0x375e260` with a far jump installed before our `main`; the manager global must be read from the `adrp`/`ldr` at `+0x20`, past the stub, never from the entry words (REVISION30; the fork panicked at boot on the console before this fix).
- **ARCropolis / other archive replacers**: if another plugin also replaces the Training layout archive (e.g. an ARCropolis mod with its own `info_training` layout), the decompressed bytes at the hook are theirs; the directory check still finds `blyt/info_training.bflyt` and the plugin overrides it, losing the other mod's layout. Q2.
- **Scene re-entry**: a new archive load per entry; with upstream's static buffer the second load reused the resident copy; with the fork's per-load inflate the first copy is long released. Save-state load, L+R+A reset and hot swap do not reload the layout.
- **HUD toggle outside a running match**: the rule applies only when `is_ready_go()`; during the entry animation the HUD is left alone. The fork widens the condition to lobby practice scenes and to "a replay reload is in progress".
- **Combo display off in the game's pause menu**: the notification queue is cleared every tick while the game setting is off (SPEC-input-*). The fork reads the value only in native Training and resets it to "enabled" on scene change.
- **Layout name encoding**: the name is a C string; upstream converted with a lossy-free `from_c_str` that panics on invalid UTF-8; the fork keeps the panic deliberately ("preserve Skyline's invalid UTF-8 panic") — Q3 recommends comparing bytes instead.
- **Hot swap / lobby (fork)**: the fork's `is_training_mode()` wrapper returns false during a swap; the tick still runs (it must keep hiding/showing panes) but the menu does not open.
- **Memory pressure**: REVISION32/33 attribute the 2026-10-13 console freezes to game-heap exhaustion; the Skyline heap pays for the NRO file, `.bss`, thread stacks and every inflated archive. Releasing the archive after copy is therefore required, not optional.

## 7. Persistence

None (the `hud` setting is part of the menu file, SPEC-core-04). The development override path `sd:/…/layout.arc` is not carried into shipped builds.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| Embedded archive in the image | upstream archive 4,213,268 B inflated; about 195 KB deflated for the fork's Training archive + waiting recipe together (REVISION33) | deflated size of the clean-room archive ≤ the fork's; report in `docs/resource-costs.md` (W5) | `nro_size.py` (`.rodata`) |
| Resident archive memory | upstream: 4,213,268 B for the whole session (plus a second ~4.2 MB for the waiting room); fork: 0 between loads, one inflated copy for ≤ 5 s after each load; the fork's current Training archive is 7,820,940 B inflated | released within 5 s of every load | `[LayoutArchive] released` lines; heap meter in diag |
| Archive hook cost | directory scan only (no payload copies); one inflate per load, "a few tens of milliseconds" on a loading screen | same | diag timing |
| Per-frame work | one tick per live layout per frame (≈ 120/s `ui_pass` at 60 fps with two practice layouts in the fork's heartbeat), name compare + the HUD rule + name-gated draws | same | heartbeat/diag counters |
| Stack | inline trampoline ≈ 0x310 B + tick frame on the UI job thread | measure once | diag |
| Heap allocations in hooks | archive inflate (one `Vec`) per load; upstream also allocated `String`s for the layout name and pane names per tick | 0 per tick (compare bytes, static pane names) | counting allocator |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| SARC directory lookup finds the member name, both endiannesses, never scans payloads, rejects truncated/invalid headers | `core05_sarc_dir_lookup`, `core05_sarc_dir_invalid` | — | — |
| Archive hook replaces pointer and the three size registers only for the Training archive | `core05_hook_register_plan` (model) | `core05-E1` Training HUD shows plugin panes; `handed out` line | `docs/EDEN-RESULTS.md` |
| Parked archive released after ≥ 5 s, never while a hand-out holds the lock; HUD intact after release | `core05_retire_expired_grace`, `core05_retire_trylock` | `core05-E2` `released … after …ms`, HUD still drawn; re-enter Training → second pair of lines | `docs/EDEN-RESULTS.md` |
| Anchor pins and locators (5 words, 28-byte locators at site+4, unique) and manager slot decode from `adrp`/`ldr` (position independent) | `core05_anchor_pins`, `core05_decode_adrp_ldr`, `core05_elf_pins_match_owned_dump` (ignored unless the owned dump is present) | `core05-E3` install line `ui_pass=…/near`, `registered layout=info_training registry_hit=true` | `docs/EDEN-RESULTS.md` |
| Pass entry hooked by another plugin: prologue reported, anchors still installed | `core05_prologue_verdicts` | `core05-E4` with Deluxe ≥ 0.3.1 staged | `docs/SAFETY.md` |
| Anchor site owned by another plugin → both anchors skipped, logged, game runs | `core05_anchor_skip_together` | `core05-E5` `foreign=inline` stand-in | `docs/SAFETY.md` |
| HUD rule: the five layouts hidden when `hud`=Off or menu open, only when ready-go | `core05_hud_rule_table` | `core05-E6` toggle HUD; open menu | `docs/EDEN-RESULTS.md` |
| Tick order and name gating (`info_training` work only in native Training) | `core05_tick_dispatch_order` | `core05-E7` | — |
| Combo-display observers: value copied, defaults enabled, no retained pointer; notifications cleared when off | `core05_combo_display_observer` | `core05-E8` turn combo display off in the game's menu | — |
| Missing plugin panes → no draw, no panic | `core05_missing_pane_skip` | `core05-E9` stock archive forced | — |
| **B3** (decided) A non-stock `info_training` layout from another mod is overridden anyway; the install line names the foreign layout; no skip | `core05_foreign_layout_injects_and_notes` | `core05-E11` profile with a layout mod | `docs/EDEN-RESULTS.md` |
| Resident memory before/after release | — | `core05-E10` heap meter diag lines | `docs/resource-costs.md` |

## 10. Open questions

1. **Behaviour when injection fails.** Skip all drawing and refuse to open the menu (silent), or show a notification through the stock HUD? Recommendation: log and refuse; the install line already says `layout_arc=skipped`.
2. **Coexistence with layout mods.** Another mod's `info_training` layout is overridden. Recommendation: document as unsupported; optionally detect a non-stock archive by size/fingerprint and skip injection (menu unavailable) — operator decision. **Decided 2026-11-01 (B3): inject anyway and document it;** the install line notes the other layout, and the other mod's training HUD does not show while this plugin is installed. The optional detect-and-skip is not adopted.
3. **Layout name handling.** Compare the name bytes against ASCII constants instead of converting to a string (no allocation, no panic on invalid UTF-8). Recommendation: do that.
4. **Draw entry choice.** The fork's two anchors are the only non-virtual positions that see every holder at the right moment; the upstream vtable-entry replace hook is simpler but collides with any plugin that replaces the same vtable entry and runs after animate only by accident of install order. Recommendation: anchors (fork design), with the playbook's sharing rules.
5. **Archive grace period.** 5 s is generous; the copy completes inside the hooked call sequence. Recommendation: keep 5 s (cheap) unless memory pressure requires less; never free synchronously inside the hook.
6. **`info_playercursor_item`** is hidden with the HUD although it carries the item cursor the player uses to pick items in Training's item menu. Confirm this is intended (upstream behaviour) or exclude it when the game's item menu is open. **Decided 2026-11-01 (B8 batch, kept line "HUD Off: item cursor hidden too"): kept as the fork does it.**
