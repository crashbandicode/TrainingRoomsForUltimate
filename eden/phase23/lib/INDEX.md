# Phase 2/3 Eden route library

## Use these first (Phase 1 already solved navigation)

- **Character / stage select:** the tile cursor plugin `libssbu_tile_cursor.nro` (`b8313ca2`, pinned in every plugin set). L3 + D-pad = exactly one tile; `ssbu-tile-cursor/events.log` reports the tile. Playbook `docs/TILE-CURSOR.md`. Never stick-steer the CSS/SSS.
- **Routes:** `/home/intpa/ssbu-mod-playbook/tools/eden-runner/library/segments/` (`enter-training`, `cycle-cpu`, `cycle-player`, `leave-css`, CPU picks …).
- **Hooks:** `eden/lr03-shared` … `eden/lr16-shared` (roster/CSS segments, `pilot_gates.py` = P1 pilot arm/check, `base.py::reload_settings` = fixture merged onto live settings).
- **Settings by fixture:** with the MENU-CLOSE fix (`21046a1`) a fixture survives menu visits; set toggles, CPU lists and the menu's start page (`last_page`) by fixture, not by presses.

## Added in Phase 2/3
One line per reusable piece: file — what it does — start state → end state — host(s) proven on. Read this before
building a route; add to it as soon as something works (playbook WORKFLOW "Running agent workers without churn").

- `../rows-A/segments/tr-menu-open.edenmacro.json` — opens the TrainingRooms menu (B+Up, same frame) — Training HUD → menu open — Yoga
- `../rows-A/segments/tr-menu-close.edenmacro.json` — closes the menu — menu open → Training HUD — Yoga
- `../rows-A/segments/intro-cancel.edenmacro.json` — skips the intro — boot intro → title — Yoga
- `../rows-A/segments/tr2-fsmash.edenmacro.json`, `tr2-jump-nair.edenmacro.json` — P1 attacks for hit rows — Training HUD → Training HUD — Yoga
- `../rows-A/segments/lr04-return-cpu-pick.edenmacro.json`, `lr10-p1-short.edenmacro.json`, `hat-next.edenmacro.json` — carried lr routes — see the files — Yoga
- Ally same-frame chords: `bc-*.edenmacro.json` in `C:\Eden\glm-bc\` on the Ally (to be copied here by the Ally worker) — Ally
- `../lib/p23-step-left.edenmacro.json` (and `-right`, `-up`, `-down`) — one tile-cursor step (L3 + D-pad) as a same-frame macro; the tile cursor plugin still makes the step and logs it — any CSS token state → one tile — Yoga (`tr-p23-yoga-2b`: `step screen=css dir=left from=40 to=39 token=p2`)
- `../lib/p23-chord-zl-up.edenmacro.json`, `p23-chord-minus.edenmacro.json`, `p23-chord-zr-down.edenmacro.json`, `p23-chord-zr-up.edenmacro.json` — two-button chords (menu shortcut ZL+Up, Minus, Input Record ZR+Down, Input Playback ZR+Up) as same-frame macros (MACROS rule 14; sequential `input.button` calls straddle frames) — Yoga (`tr-p23-yoga-2b`)
- `../lib/p23-hit-playback.edenmacro.json` — fsmash with the Input Playback chord fired at frame 40 — Training hit → playback during the CPU's hitstun — Yoga (`tr-p23-yoga-2b`, fired; the DI evidence itself is open, see EDEN-YOGA-2-REPORT)
- `../lib/p23-hit-playback-air.edenmacro.json` — jump+nair with the Input Playback chord at frame 55, inside the CPU's aerial hitstun (DI applies only in the air) — Yoga (`tr-p23-yoga-2b`)
- `../lib/p23-walkoff-left.edenmacro.json` (sha256 `748529e7c409ebb2571b4c6a9d2c747786c73de546def31a589a842bdcd67b70`) — P1 walks off the left edge: a real natural death (the right-walk recipe pushes into the CPU and never dies) — Training HUD → death/respawn — Yoga (`tr-p23-yoga-2b`)
- Tracepoint recipe: `symbols.load {module: libtraining_rooms.nro, elf: <diag build's unstripped .so>}` then `trace.arm {"target": "libtraining_rooms.nro!cpu_sdi_hook", "registers": ["x0","x1","x30"], "on_return": true}` — entry+return pairs with x0 stream to the run's `yoga.trace.jsonl`; symbol offsets for this build: `cpu_sdi_hook` 0x1a0740, `state_get_param_int_hook` 0x1ad080, `cpu_attack_air_kind_hook` 0x19ce94, `cpu_di_hook` 0xdf2e0 — Yoga (`tr-p23-yoga-2b`)
- Route hooks: `../rows-yoga-2/steps2.py` (`q16` fixture-route menu navigation via the `last_page` fixture; `q7`/`n7` record their menu routes once in-boot via `_route`; `n2t`/`n9t`/`n10t`/`n3t`/`n4t` redo rows with tracepoints) — see EDEN-YOGA-2-REPORT for what passed and what is still open — Yoga