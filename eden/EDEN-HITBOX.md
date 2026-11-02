# Eden results — hitbox visualizer (SPEC-core-06), stream `hitbox`

2026-11-02, Yoga (`memento-claw-yoga-ubuntu`), Eden-GIF `d483c082…`, harness `eden/tr_eden_hb.py` (own profile
`/home/intpa/trainingrooms-eden-hb/profiles/hb`, display :96, muted through the CLI before boot), route `eden/route_hb.py`
(Pokémon Stadium 2, Steve as P1; the CPU fighter is the game's pick), driver `eden/hb_drive.py`. Every run: our diag NRO + CPU pilot v0.2.1
(`e3033426…`) + tile cursor (`b8313ca2…`), `settings.json` `{"menu":{"hitbox_vis":[1,0]}}`. Outputs (screenshots, `practice-diagnostics.log`,
`events.log`, `console.log`, `run.json`, `memory.peak`, PREP/LAUNCH receipts) under `eden/runs/<label>/` (gitignored). Cost numbers: the diag
`[Hitbox] stats` lines (one per 600 fighter passes; ticks at 19.2 MHz).

| Run | Build | What | Result |
| --- | --- | --- | --- |
| hb-01 | diag `193ec03b` (`6a4010d`: respawn every frame, hurtboxes added) | install, idle, slow-motion jab (speed 5 %), grab (R), shield | 16 rows `stock`, hooks `far`; install line cut at 8,186 B (no feature fields: root cause = 8 KiB stack buffer, fixed); Steve's 19 hurtbox spheres visible (`111735-training-idle.png`), jab spheres along the arm (`111948-slow-jab-2.png`); the CPU (Ridley) 55 hurtbox spheres never visible; grab boxes (2+4+2) not visible; after the grab no sphere at all (`112313-idle-full-speed.png`); pass 8,644 ticks avg (450 µs), max 50k; `hook_stats` delta 0 after the scene load; console 0 matches |
| hb-02 | proof diag `dfd3001e` (`099ae93` + a `req_follow` refusal counter) | idle 30 s, Steve vs Sonic | `requests=14700 refused=9657` per 600 passes (65 % of the sphere requests answered with no effect): the respawn design runs the game's effect pool dry (lead condition 1); the CPU's spheres never shown (`121403-idle.png`); console 0 matches |
| hb-03 | diag `19c74e53` (`64d439f`: persistent spheres by handle) | idle, walk, shield (ZR, L), grab (R) in slow motion, KO, menu | both fighters' hurtboxes visible (Steve 19, Luigi 23; `121958-idle.png`); `failed=0` on every line; held 42 (max 54 with a hitbox); idle pass 225–231 ticks avg (12 µs), 715 at the start, max 19,984 when spawning; shield: our 19 spheres killed by handle, the red shield bubble alone (`shield-zr`); grab boxes now visible at the tether (`122115-grab-2.png`) but re-issued `CATCH` id 0 piled up (8→10→12 held): fixed after the run (one sphere per key); the KO blast and confetti (game effects) all appear with 54 held (`ko-*.png`); menu: Display & Controls › Hitbox Visualization shows the new help "On: hitboxes, grab boxes and hurtboxes are drawn." and toggles to OFF; closing the menu froze the game (`game_fps 0`), the known integrated-tree menu-close hang (owner ui-screens), recorded, not this stream's; install line 9,574 B complete (`hitbox=installed result=installed heap_allocations=0`); console 0 matches |

Open rows (hb-04 / hb-05, queued): Ridley + Steve with the grab-key fix (persistent count against headroom), a jab that connects (hit spark),
the setting Off from boot (no game call), cost re-check.

By design (not a regression): while the setting is On, the fighter's animation-command effects are asleep (SPEC-core-06 §2 and §5 step 5,
`MotionAnimcmdModule::set_sleep_effect(true)` outside the grab, tread and `DOWN` statuses): a smash attack's charge glint, dust and the other
script effects do not show, as upstream; the KO blast, the shield bubble and hit sparks come from the game's own systems, not the script.

Allocation note: every run shows hook allocations once during the scene load (hb-01 7+15, hb-02 and hb-03 31), in the window where the CPU core
resolves its constants and the diag build dumps `cpu_ids`; `delta=0` on every later line while the pass runs 120 times a second. The visualizer's
paths allocate nothing (fixed storage); the exact source of the scene-load allocations is not attributed yet.
