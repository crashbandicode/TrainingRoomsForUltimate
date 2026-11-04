# Phase 2/3 batched Eden rows — Worker A's list (`rows-A.md`)

2026-11-04, worker A (phase2-closeout). Preparation only: no boot was made for this file. The rows
run on one continuous boot per MACROS "Test runs" rule 8, ordered cheap-and-independent first, state-changing
and wedge-risk rows last; the only second boot is S2's, which is inherently two boots (its first boot is the
last row of the main boot). Playbook commit followed: `634434c`. Baseline: this branch (`p23/phase2-closeout`,
`work/implementation` `42b510c` + salvage + the two core-07 tests). Eden: the pinned old build
`Eden-GIF/20261103-063f1b80bb2c-persistent-tracing-6a0c80d2b858/Eden.AppImage` (sha256 `6a0c80d2…`), muted
(`core.volume=0`), one Eden per host, claim in `OWNER-COORDINATION.md`, ≥ 10 GiB MemAvailable. Rows are run by
`tools/eden-test/eden_test.py new|up|run|down` with one `rows.json` per boot; every `setup` is fixture-first
(MACROS rule 13): the plugin's settings come from `eden/mk_settings.py` output pinned in `instances[].files`
(`to: sd:/ultimate/TrainingRooms/settings.json`), never hand-steered menus. Missing segments are recorded once
before `Up` ("Building a new route"), not driven by hand. Short ruleset: 1 stock, 1:00, items/hazards/FS off.

Boot A (one boot; ARCropolis `libarcropolis.nro` pinned from the card
`/home/intpa/ssbu-cpu-pilot-install/switch-mtp-v0101-v021/pilot/after/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libarcropolis.nro`,
sha256 `92171b2f…`, for rows 6–7; tile cursor NRO alongside):

1. **`p23-q7-native-write`** (Q7, SPEC-input-01 :238, IN01-E18).
   - Setup: fixture settings with `replay_capture` On (factory default On) and an imported
     `replay_capture.ini` (`enabled=on`, plus a `keep=7` line and a comment, so the owned-key repair is
     visible); via `fixture` (files pinned before `Up`).
   - Inputs: library `enter-training`; open the menu (B + D-pad Up), Named-macros group, toggle
     **Replay capture** Off, close (`pause-close` shape / menu-close segment), re-open, toggle On, close.
   - Pass: each confirmed change writes `replay_capture.ini` exactly once (the plugin's one write line per
     change in `events.log`; no write at boot or on open), the file parses to the chosen value, the unknown
     lines and comments survive, and the Deluxe Plugins page row stays in sync (screenshot after each toggle).
   - Evidence: `copy` of `sd:/ultimate/TrainingRooms/replay_capture.ini` after each change and after `down`
     (native write proof), the log lines, both Deluxe-page screenshots, row receipt.
2. **`p23-n3-delay-range-picks`** (N3, SPEC-core-02 :209,211; consumers SPEC-cpu-03/05/06).
   - Setup: fixture settings with `aerial_delay` = range [2,6] step 1, the other six delay rows empty
     (checklist cleared by the same popup rule); via `fixture`.
   - Inputs: `enter-training`; CPU attack drills: `cpu-next-frames`-paced P1 forward smashes on the CPU
     (recipe: `b:a:200` taps while the CPU is hit, 12 hits), input display visible (input02 row) to read the
     applied aerial delay.
   - Pass: every applied pick is one of {2,3,4,5,6} (uniform-pick rule; a fixed value outside the range or a
     checklist value fails), and with a checklist entry non-zero in a second fixture the range editor is
     greyed (screenshot of the popup).
   - Evidence: the input-display frames per hit (12 crops), the popup screenshot, receipt; the picks list in
     the row receipt.
3. **`p23-n4-di-during-playback`** (N4/B5, SPEC-cpu-02 :168; SPEC-input-01 :162,236–237, IN01-E17).
   - Setup: fixture settings: DI Direction = Up, SDI none, `settings_override_playback` Off; record slot 1
     first: press the Input Record chord (default ZR + D-pad Down, `record_trigger` Button Combination), P1
     holds a neutral-to-left stick sway for ~2 s, stop with the same chord; via `fixture` + one `record`
     (bounded, becomes a segment).
   - Inputs: hit the CPU (P1 forward smash, `b:a:400` after `ready`), playback slot 1 with the Input Playback
     chord (default ZR + D-pad Up) during the next hit; then fixture-swap the same settings with
     `settings_override_playback` On (second fixture file pinned; settings reload via the menu's saved copy —
     the plugin's settings-reload fixture route) and repeat the same hit+playback.
   - Pass: with the row Off the playing recording wins — no DI streak and the recorded stick motion on the
     CPU during playback; with the row On the DI override applies during playback (the blue DI streak shows,
     as in run A of `C1162/captures-ally`); after playback ends the overrides apply again in both cases.
   - Evidence: frames at +5…+15 after contact for both cases beside the no-DI baseline, the record/playback
     notification screenshots, receipts.
4. **`p23-n9-sdi-unset-none`** (N9, SPEC-cpu-02 :95,160,169; `cpu-02-sdi-unset-none`).
   - Setup: fixture A: `sdi_state` empty (all 0) with `sdi_strength` High; fixture B: `sdi_state` = Up with
     `sdi_strength` None; both staged as files, switched by the settings-reload fixture between the two cases.
   - Inputs: the same hit as row 3 (`ready`, P1 forward smash) twice per fixture.
   - Pass: no SDI in any hit — no white jagged burst on the CPU's head in +4…+17 (compared against the
     no-SDI control `sdi-none/`), no hitlag displacement toward a direction, and the plugin's SDI hook logs
     the unset path (diag line, if the build carries it) instead of an input; the native CPU's own SDI never
     appears (N9/B8 decision, SPEC-cpu-02 open question 3).
   - Evidence: frame crops per hit against the control set, receipts.
5. **`p23-n10-e7-sdi-streak`** (N10, SPEC-core-07 :223, E7).
   - Setup: fixture: `sdi_state` = Up, `sdi_strength` High; the host measurement
     (`tests/core07_assets.rs::core07_sdi_chip_streak_length`) is the ×2.0 proof and has passed before boot.
   - Inputs: `ui-sdi`/`ui-reopen-sdi` library segments to open the SDI Direction grid; one hit for the live
     spark (`sdiup_f+12` equivalent frame).
   - Pass: the SDI grid's chips show the game's white spark visual; the grid screenshot beside the capture
     crop (`c1162/captures-ally-sdi/sdi-up/frames/sdiup_f+12.png`, decoded with `bntx_view.py`-style tooling
     only for the game-side asset where needed) shows the drawn streaks visibly longer than the draft-8 chip
     (sheet 21's x2.0 column), and the DI grid is unchanged (row 3's DI frames serve).
   - Evidence: the SDI grid screenshot, the side-by-side sheet (grid | capture crop), the host measurement
     line from the test output, receipt.
6. **`p23-q16-portraits-mod-fallback`** (Q16 as amended for RC10, SPEC-core-07 :224, E8/E9).
   - Setup: the profile stages, before `Up`: the card's ARCropolis pinned NRO (above) and one owned
     temporary character/costume mod under `ultimate/mods` (staged with its own manifest, removed at
     `down`; the lent staging script preserves mods — never overwrite an existing mod), plus a deliberately
     unreadable portrait case (a VALID mod icon file whose host read is refused after the HUD settles; never a
     malformed file, which crashes the game's own loader) for the E9 half; via `fixture`.
   - Inputs: `ui-roster` library segment to the roster grid; cycle one fighter with a costume (the mod's
     row), then the refused-file row.
   - Pass (E9 amended 2026-11-04 by the operator, `docs/DECISIONS.md` Q16): the grids draw the game's own
     `chara_2` *stock icon* (`picture=2`) read at run time through the mod-aware loader. The modded
     character/costume shows its own icon (E8). An unreadable mod file shows the **stock** icon, because
     ARCropolis serves the original (E9). No baked picture is substituted (the archive stays the recipe's:
     row `core07_no_portrait_pictures` green on the built branch). The names-only tile is the texture
     helper's safety state, covered by host tests, not by this row.
   - Evidence: grid screenshots (mod case, fallback case) beside the stock case, the mod staging/removal
     manifest, receipts.
7. **`p23-b3-modified-archive-reload`** (B3, SPEC-core-05 :117,123; `EDEN-UI-SCREENS.md` "B3 fixture and
   reload preflight").
   - Setup: the private modified archives `runs/ui-builder-83d2c58/{training,waiting}/target.arc` staged as
     the info_training/info_waiting sources (identity and paths in `runs/ui-b3-preflight/fixtures.json`;
     Training sha256 `241150ca…`, waiting `326f8341…`), via `fixture`; driver flags `--archive-base vanilla
     --archive-loads 2`.
   - Inputs: `enter-training`, open and close the menu (one Training hand-out), pause-menu path that loads
     the Waiting screen (second hand-out), then close and reopen the menu (the reload).
   - Pass: every Training hand-out uses the vanilla base, zero per-load allocations, two covered hand-outs,
     every matching release after ≥ 5000 ms; the Waiting coverage separately passes the same gate; the
     modified archive's own layout shows while it is loaded and the reload re-injects without leak
     (`gate_layout_archives`).
   - Evidence: the archive gate lines from `events.log`/console, per-load screenshots (the modified layout
     visible), `run.json` and `memory.peak`, receipt.
8. **`p23-n7-capture-apply-menu-closed`** (N7, SPEC-input-04 :182–185, IN04-E08).
   - Setup: fixture settings with Menu shortcut 1 empty; via `fixture`.
   - Inputs: open the menu, open the Menu shortcut 1 editor, press the new chord (recipe: hold `b:zl:600`,
     tap `b:dpup:250`, confirm with `b:a:400`), close the menu; with the menu closed press the new chord
     (menu must open at once), then press the old default chord (no menu); repeat once.
   - Pass: the capture takes the physical presses while the editor is open (the chord readback matches what
     was pressed — the chord matcher's controller mapping, commit `2cc0f23`), the confirmed change applies
     at once with no restart, the old chord no longer opens the menu, and `button_config.ini` is written on
     the confirmed change only (`disabled` for the emptied slot).
   - Evidence: the editor screenshots before/after, the two closed-menu probe screenshots (new opens, old
     does not), a `copy` of `button_config.ini` after `down` with its parse-back, receipt.
9. **`p23-b7-airdodge-shield-tilt-both-sides`** (B7, SPEC-cpu-02 :162,168; SPEC-cpu-05 :103,151,159).
   - Setup: fixture: `air_dodge_dir` = Out, `shield_tilt` = In, DI none; CPU at tile 74 (library
     `cpu74-lucas` route family).
   - Inputs: side 1 (P1 left of the CPU): `ready-start`, hit the CPU into an aerial hitstun (P1 up-air
     recipe), observe the directional air dodge and the shield tilt when shielding; then `leave-css`,
     swap sides (cycle-player / the CSS segment family with the token moved to the other side), re-enter,
     repeat the same hit.
   - Pass: on both sides Out is *toward the opponent* and In away (opponent-relative, DECISIONS.md 309–317),
     and the shield tilt In leans away from the opponent on both sides — the frames show mirrored stick
     outcomes, not stage-relative ones.
   - Evidence: per-side frame strips (air-dodge start frame, tilt frames), the two `ready` screenshots
     proving the sides, receipts.
10. **`p23-n2-natural-death-invincibility`** (N2, SPEC-state-01 :29–30,37,94,251–255).
    - Setup: fixture A: `respawn_invincibility` Off, Auto Save States Off; fixture B: the row On (also with
      Auto Save States On for the greyed-row screenshot, where the stored value is kept); via `fixture`.
    - Inputs: natural death by walk-off (library `peer-walkoff` shape, P1 walks off; then the CPU walks off:
      `cpu-next-frames` walk-off recipe), twice per fixture.
    - Pass: Off — instant respawn, no invincibility (parity with the fork); On — the fighter (P1 and the CPU
      each once) respawns with the game's own respawn invincibility (the star sparkle / invincibility
      visible and the §4.1 parameter overrides absent while no load is in progress), on the respawn platform
      where the game's own path gives it; with Auto Save States On the row is greyed and a death still loads
      the state instantly and silently.
    - Evidence: respawn frames per case (the invincibility sparkle frames), the greyed-row screenshot, the
      stored-value round-trip after the fixtures swap, receipts. (Deaths can wedge state — this row runs
      last before S2.)
11. **`p23-s2-two-boot-confirm`** (S2, DECISIONS.md 244–248; `EDEN-R2FIX.md` :41–48 already proved the
    two-boot path once — this is the batched confirm row).
    - Setup: fixture: a `TrainingRooms` folder whose `settings.json` is partly corrupt (one bad field, S2's
      field-wise recovery) plus the migration marker absent; via `fixture`; **second boot required**.
    - Inputs: none beyond `enter-training` on both boots.
    - Pass: boot 1 loads the file field-wise (good fields kept, the bad field defaulted, no panic —
      `settings_file.rs` recover), the marker/import completes; boot 2 keeps the recovered settings and the
      menu shows the recovered values.
    - Evidence: both boots' settings copies, the boot-2 menu screenshot, console scans (no `panic`/`Unmapped`
      in the plugin's window), receipts.

**Boot discipline.** One `rows.json` covers rows 1–10; a failed row is fixed or deferred and the boot carries
on (MACROS rule 8). Row 11's second boot is the only reboot and it is inherent to the row, not a bug reboot.
Each run writes its ≤ 20-line `RESULTS.md`; the phase's compliance report collects them. Evidence naming,
`pace` checks and the `STOP-INPUT` rules are the runner's. No row assumes fixture indexes: stage/fighter
positions are read from the log or roster JSON (MACROS rule 11).
