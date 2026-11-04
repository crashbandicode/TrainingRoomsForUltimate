# Ally gbc-04 findings: in-fight fixture swaps and the one-press tile step (2026-11-04)

Observed on the Ally rig `C:\Eden\glm-bc` (Eden `a3a27006…`, TrainingRooms diag `c04a316c…`, plugin set bc2),
run `live-gbc-04`. Facts only; evidence `/home/intpa/eden-runs/gbc-04/` (ops.jsonl, practice-diagnostics logs, shots).

## In-fight fixture swaps (the log-gated way to set both fighters with zero hand inputs)

A `diag_settings_next` push carrying `player_character`/`cpu_character` (one-hot over the 87-entry catalog; Steve is
index 76, Ridley index 63) fires `Selection::Direct` requests per role, but only where the roster engine has a
practice scope AND has already observed the fight's entries:

- At the Training **CSS**: both requests dropped — `[CharacterRoster] request ignored role=… selection=Direct(…):
  not a practice scene`. The CSS is not a roster scope (`scope()` needs `melee == 0x20 && mode == 0x04040000`).
- In the fight, within ~2 s of the start: `[CharacterRoster] request rejected: no current practice entry` — the
  per-frame observer had not recorded the entries yet (`[CharacterRoster] observed owner=… entry=… kind=…` lines
  appeared ~1.7 s later).
- After the `observed` lines exist, the same push swaps both fighters, synchronously with preload Off:
  `[CharacterRoster] install owner=1 … from=0x0 to=0x42`, phase lines, then
  `[CharacterRoster] complete owner=1 target=Ridley kind=0x42 slot=None ticks=59 body_id=0x10002`.
  The Player role's completion goes through the ordinary-Mii owner when P1 is a non-Mii fighter: the line is
  `[CharacterRoster] request handed to the ordinary Mii owner role=Player selection=Direct(TrioFighter(88))`
  followed by `[CharacterRoster] HUD refresh owner=0 kind=0x58 delivered=1` — **no `complete owner=0` line**, so
  gate the Player half on the HUD refresh (or the gshot), not on `complete owner=0`.

Route (0 hand inputs, all log-gated): enter the fight → wait for `observed owner=0|1` → push a reset fixture
(Mario/Mario one-hots) → wait `diag_settings_reload … issues=0` → push the wanted pair → wait the reload → wait
`complete owner=1` / the Player HUD refresh. The reset step guarantees the second push differs from the live
settings (a push whose character values equal the live ones queues nothing).

## Fixtures (bc3-base derived; complete settings, merge-safe)

- `bc4-css.json` — bc3-base + `player_character`=Steve, `cpu_character`=Ridley, `cycle_preload_slots`=Off,
  `cycle_preload_group_swap`=Off (a synchronous Direct swap; the preload queue is what stalled CPU Kirby in gbc-02).
- `bc4-off.json` — same, `hitbox_vis` `[0,1]` (Off). `bc4-reset.json` — same, both characters Mario.
- Push with `["settings", <abs path on the rig>]` + `["waitplug","TrainingRooms/practice-diagnostics.log",
  "diag_settings_reload",30]` (wa.py ops).

## Tile steps: one L3+D-pad press per macro play

A macro carrying one L3+D-up press registered **two** steps on this rig (`step … from=none to=78` then
`from=78 to=67` — the hand overshot to Simon). `p23-step-up.edenmacro.json` is the corrected one-press form
(press f8, release f26, duration 40; L3 bit 16 | d-pad Up 8192 in the same frame). Verify
`settled screen=css at=<tile> hovered=<tile>` in `ssbu-tile-cursor/events.log` before pressing `a`.

## C5/C6 evidence shapes (hb1 route B)

- Shield (vis On): the shielding fighter's spheres are killed at the shield start (`[Hitbox] pass … killed=19`)
  and respawn on release (`spawned=19`); the game's own red bubble is the only shield visual. Off frame identical.
- Vis Off: all held spheres are killed at the push (`killed=19`, `killed=55`) and never respawn; **no new
  `[Hitbox] stats` line appears** (the pass early-returns before its frame accounting) — a 25 s `waitplug` for
  `\[Hitbox\] stats` timing out is the no-requests gate.
