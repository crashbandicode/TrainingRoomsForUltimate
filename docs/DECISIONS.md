# Decisions for the operator

2026-11-01. This page lists every operator decision of the menu proposal, one illustration per decision. In each
picture the recommended option has a **green outline**. The images are in [`decisions/`](decisions/) and are drawn
by `tools/render_decisions.py` in the menu mockup's style. Sources: `MENU-DRAFT.md` §4 (the menu questions keep
their numbers), the specs' *Open questions* sections, `PROVENANCE.md` *Open questions* and `specs/SPEC-core-08` §10.

**Answered 2026-11-01 (late evening ET):** the operator approved the proposal with the answers in the *Decision*
column below, and implementation may begin. Items marked **open** were not answered; their recommendation is the
working default until they are. The new requirements the answers add are collected in
[*New requirements from the answers*](#new-requirements-from-the-answers) at the end, with their mockups.

| # | Decision | Recommendation | Decision (2026-11-01) |
| --- | --- | --- | --- |
| Q1 | Number of pages | five, as drawn | **as recommended** (plus the hidden Developer page, Q8) |
| Q2 | Dropdown input | Left / Right cycle in place; A opens the list | **as recommended** |
| Q3 | Random-among-a-set delays | keep the checklists | **both:** checklist and min–max range; using one disables the other |
| Q4 | Overrides, chords, swap groups | one editor row each | **as recommended** |
| Q5 | First page on open | the last page used | **as recommended** (2026-11-02): the last page used; the first open ever shows CPU |
| Q6 | Item on load (older saves) | keep the row for now | **drop the row;** the import converts older saves once |
| Q7 | Replay capture in the menu | add the toggle | **as recommended** |
| Q8 | Preload Samples | diag build only | **Developer page:** hidden unless an INI enables it (the operator's own INI enables it) |
| Q9 | Menu shortcuts | read-only; the INI is the editor | **B: editable in the menu;** the menu writes `button_config.ini` |
| Q10 | Help text | two-line panel | **as recommended** |
| Q11 | P1 CPU fallback without the pilot | fallback rows, as drawn | **as recommended** |
| Q12 | Direction grid weights | weight digit on the chip | **as recommended** |
| Q14 | Grid paging | 4 × 8 items, 4 × 4 stages (the fork) | **B: denser, items and stages** (stages added 2026-11-02); in handheld mode 4 × 4 pages |
| Q15 | Mii rows | list with the bust where one exists | **as recommended** |
| Q16 | Fighter portraits and mods | read at run time, like stages | **as recommended** |
| Q17 | Which chips show the game's DI / SDI visual | DI and SDI only | **as recommended;** the SDI spark's white streaks drawn longer, closer to a match (×2.0 confirmed 2026-11-02) |
| Q18 | When a CPU setting change takes effect | follow the fork: apply at the next CPU frame | **as recommended (A)** |
| S1 | Five import details | rename to `settings.json`, import both INIs, the rest as proposed | **as recommended, except item 3:** `replay_captures/` is imported in full (it is the Replay Window's crop source) |
| S2 | Corrupt settings file | keep a copy, load field by field | **as recommended** |
| B1 | Plugin name | `training_rooms` | **as recommended** |
| B2 | Panic policy | log it, no pop-up | **as recommended** |
| B3 | Another mod's training HUD layout | inject anyway, document it | **as recommended** |
| B4 | P1 driven by an AI | CPU settings on the CPU only; recording refused | **as recommended** |
| B5 | Input playback and the CPU settings | the recording wins | **the recording wins by default;** an advanced row lets the settings override it (default Off) |
| B6 | Ice Climbers state | one state per fighter | **as recommended** |
| B7 | In / Out for Airdodge and Shield Tilt | toward the opponent, everywhere | **as recommended** |
| B8 | Behaviour changes from the fork | accept the batch | **as recommended, except:** SDI not set means no SDI (the fork's rule stays); the respawn line becomes a toggle |
| B9 | Upstream developer-only code | drop it from the shipped build | **Developer page** (with Q8): shipped, hidden unless the INI enables it |
| B10 | README media | reuse the GIFs that show no menu | **as recommended** |
| B11 | Ledge options changed while the CPU is already on the ledge (fork and clean room) | re-roll on menu change, like SDI | **as recommended** (2026-11-02): re-roll on menu change, in both trees (the fork gets it in its next build) |

## Menu

### Q1 · Number of pages

Five pages, four (Saves and Display & Controls merged) or six (Stages & Lobby split out)?
**Recommendation: five.** Each page has one theme, and no page is very long or nearly empty. Cost: none; the
mockups already show five.

![Q1 pages](decisions/01-pages.png)

### Q2 · Dropdown input

Should Left / Right change a dropdown's value in place, or must A open the list first?
**Recommendation: in place, as drawn.** A still opens the full list. It is one press per change instead of
three. Cost: a stray Left / Right changes the value.

![Q2 dropdown](decisions/02-dropdown.png)

### Q3 · Random-among-a-set delays

Aerial, fast-fall, throw, pummel and ledge delays, OoS offset, reaction time and the slot sets: keep them as
checklists, or use a min–max slider?
**Recommendation: checklists.** They keep today's meaning, so saved files load unchanged. A range slider
cannot express a set like {0, 5, 10}. Cost: a checklist is a longer popup than a slider.

![Q3 delays](decisions/03-delays.png)

**Decided 2026-11-01: both.** Each delay row offers the checklist and a min–max range; while one holds a value the other is greyed, and Y clears the active one. See *New requirements* N3.

### Q4 · Situation overrides, Button chords, Swap groups

Should these three be sub-screens (one row each), or should all their rows sit on the page?
**Recommendation: sub-screens.** Putting the rows on the page adds 12 rows to CPU, 6 to Fighters & Stages
and 5 to Display & Controls. Cost: one more press (A) to reach them.

![Q4 sub-screens](decisions/04-subscreens.png)

### Q5 · First page on open

Should the menu open on the last page used (today's behaviour) or always on CPU?
**Recommendation: the last page used.** A drill loop (record, test, adjust) comes back to the same page.
Cost: none; this is today's behaviour.

![Q5 first page](decisions/05-first-page.png)

**Decided 2026-11-02: the last page used;** the very first open (no saved page yet) shows CPU.

### Q6 · Item on load (older saves)

Keep this legacy row for saves made before the fork's item snapshot, or convert those saves once at import
and drop the row?
**Recommendation: keep it for now.** It carries no risk, and new saves never use it. Revisit once the import
has been tested on real cards. Cost: one rarely used row on Saves.

![Q6 item on load](decisions/06-item-on-load.png)

**Decided 2026-11-01: drop the row.** The first-boot import sets an older save's item snapshot from this setting once (N6).

### Q7 · Replay capture in the menu

Should there be a menu toggle that writes `replay_capture.ini` and matches the Deluxe Plugins row, or should
the setting stay INI / Deluxe only?
**Recommendation: add the toggle.** It can then be changed in game, and all three places always agree. Cost:
one row, and one small INI write when the row changes.

![Q7 replay capture](decisions/07-replay-capture.png)

### Q8 · Preload Samples

Ship this diagnostics row under *Loading (advanced)*, or put it in the `diag` build only?
**Recommendation: diag build only.** Players never need it, and the playbook's `diag` feature exists for
exactly this. Cost: measuring on a card then needs the diag build.

![Q8 preload samples](decisions/08-preload-samples.png)

**Decided 2026-11-01: a Developer page** (with B9), hidden unless an INI flag enables it; the operator's own INI enables it, so capture and test rows need no diag build (N1).

### Q9 · Menu shortcuts

Show the INI shortcuts read-only in the menu, or make them editable there with the INI as the file of
record? (This also answers `PROVENANCE.md` open question 5.)
**Recommendation: read-only.** No second chord editor is needed, and the behaviour matches the fork
(restart to apply). Cost: changing a shortcut means editing `button_config.ini`.

![Q9 menu shortcuts](decisions/09-menu-shortcuts.png)

**Decided 2026-11-01: B, editable in the menu.** The two shortcut rows open the chord editor; the menu writes `button_config.ini` and the new chord applies at once (N7).

### Q10 · Help text

Keep one help line at the bottom, or use a two-line panel?
**Recommendation: two lines.** The fork's longer help texts (the group rows, for example) then fit without
being cut. Cost: the item list shows one row fewer.

![Q10 help text](decisions/10-help-text.png)

### Q11 · P1 CPU fallback when the pilot plugin is absent

Show the two hidden fallback rows (as proposed), make it INI only, or drop the TM-side control and require
the pilot?
**Recommendation: the fallback rows.** P1 control still works on a card without the pilot. With the pilot
installed, only the info row shows. Cost: two rows, shown only when the pilot is absent.

![Q11 P1 fallback](decisions/11-p1-fallback.png)

### Q12 · Direction grid weights

Show a weight digit on each selected chip (today's counts), or use plain on/off chips with an even pick?
**Recommendation: weights.** They keep today's semantics, and saved files carry the counts. Cost: a small
badge per chip, and A adds weight instead of only toggling.

![Q12 weights](decisions/12-direction-weights.png)

### Q14 · Grid paging

Keep items at 4 × 8 per page (two pages) and stages at 4 × 4 (eight pages), as the fork does, or use denser
pages?
**Recommendation: as the fork.** This layout is tested on the console, and the item names stay readable.
Cost: L / R to reach items 33–38.

![Q14 paging](decisions/14-grid-paging.png)

**Decided 2026-11-01: B, denser, with a handheld rule.** Docked: all 38 items on one page. Handheld: three pages. Stages (operator, 2026-11-02): the denser paging applies to the stage grid too (N5).

### Q15 · Mii rows

Lists with the fork's bust thumbnail beside each saved Mii where one exists, or text only?
**Recommendation: busts where they exist.** They make saved Miis easy to tell apart, and the fork already
makes them. Cost: one small texture per visible Mii. (The picture uses a neutral silhouette in place of a
bust.)

![Q15 Mii rows](decisions/15-mii-rows.png)

### Q16 · Fighter portraits and mods

Should the portrait grid read each fighter's own UI portrait from the game at run time, as the stage grid
does (so character and costume mods show), or keep fixed stock icons as the fork does?
**Recommendation: read at run time, falling back to a text tile.** This is the same rule as stages and
DI / SDI chips: nothing is baked or committed, and mods show. Cost: the run-time read has the same open
unknowns as the stage thumbnails (`SPEC-core-07` §4.5, U1–U8), plus choosing the right costume, so it needs
one more development probe.

![Q16 portraits](decisions/16-fighter-portraits.png)

### Q17 · Which chips show the game's DI / SDI visual

Should only the DI and SDI chips show the game's own visual, with Airdodge Direction and Shield Tilt keeping
the clean room's arrows?
**Recommendation: yes, DI and SDI only.** The game's visual shows launch influence; on an air-dodge or shield
chip it would suggest something those options do not do. Cost: none; this is how the mockups are drawn.

![Q17 chips](decisions/17-di-sdi-chips.png)

### Q18 · When a CPU setting change takes effect

**The fork (stage M5o):** a changed SDI Direction or Shield Tilt now applies at the next CPU frame, with a
new weighted roll; DI was already read live at each hit. (Before M5o, SDI Direction applied only at the next
Training reset, and the menu did not say so.) **Clean room:** should it follow the fork?
Options: (A) follow the fork: apply at the next CPU frame, one roll that holds until the next reset or the
next change of that row; (B) apply the same way and also roll a new SDI direction every hitlag episode
(`SPEC-cpu-02` questions 1 and 9); (C) keep reset-only and label the row "applies on next reset".
**Recommendation: A, follow the fork.** It is built and was checked in Eden (the SDI Up → In change, and the
Shield Tilt change, applied with no reset). The menu then means the same in both plugins, and Shield Tilt and
the other per-reset rolls follow the same rule. Cost: with two or more directions selected, SDI keeps one
direction until the next reset or change, which upstream's own source marks as a bug. B would vary it every
hit at the price of one roll per hitlag episode and a parity difference from the fork; it stays open if you
want it.

![Q18 apply on change](decisions/18-apply-on-change.png)

Fork evidence: `artifacts/…/parked-reuse175/refactor/c1162/di-sdi-apply/RESULT.md` (Eden rows R-APPLY1–3
passed on the patch's own builds; `PATCH.patch` is in the same folder); the picture's rolls are examples.

## Files

### S1 · Five import details (`SPEC-core-08` §10)

The storage root and the import are decided. Five details remain; each has a numbered tag in the picture.

- **1 · settings file name:** rename to `settings.json`, or keep the fork's name `training_modpack_menu.conf`?
  *Rename*; the loader also accepts the old name once.
- **2 · the two INIs** (`button_config.ini`, `replay_capture.ini`): import them too? *Yes*, under the same
  rules, so shortcuts and the capture choice survive.
- **3 · `replay_captures/`:** not imported (the Replay Window starts empty), or imported below a size cap
  (for example 64 MiB)? *Not imported*; the README explains how to copy it by hand.
- **4 · TOML-only cards** (the fork before its JSON port): *not converted*; the fork's
  `tools/port_save_states.py` is the documented offline path.
- **5 · re-running the import:** by deleting `migration.json` only, or also from a menu row? *The marker only*;
  no menu row.

Cost: item 2 adds two small files to the copy list. The others cost nothing.

![S1 import details](decisions/S1-import-details.png)

**Decided 2026-11-01: as recommended, except item 3.** `replay_captures/` holds the companions the Replay Window crops macros from (`docs/reference/named-macros.md` in the fork), so it is imported in full, with no size cap (N8).

Corrected 2026-11-02: the source is `training_modpack_menu.conf`; `training_modpack.json` is the updater record and is not imported.

### S2 · Corrupt settings file (`SPEC-core-04` question 2)

Delete the file and start from defaults (upstream), or keep a renamed copy and load it field by field?
**Recommendation: keep a copy, default per field.** One bad field no longer wipes every setting. Cost: one
renamed file on the card until the player deletes it.

![S2 corrupt settings](decisions/S2-corrupt-settings.png)

## Behaviour and product

### B1 · Plugin name (`SPEC-core-01` question 6)

Name the plugin `training_rooms` (`libtraining_rooms.nro`), or keep `training_modpack`?
**Recommendation: `training_rooms`.** The two plugins can then be told apart on a card and in logs. They must
never be installed together, because they hook the same sites. Cost: none.

![B1 plugin name](decisions/B1-plugin-name.png)

### B2 · Panic policy (`SPEC-core-01` question 1)

Show the error pop-up on a panic (upstream: the game is unusable until it is closed), or write the panic to
the diagnostics file and the Skyline log with no pop-up?
**Recommendation: log it, no pop-up**, as the playbook's plugins do. Cost: the player sees no message on
screen; the reason is in the logs.

![B2 panic](decisions/B2-panic-policy.png)

### B3 · Another mod's training HUD layout (`SPEC-core-05` question 2)

When another mod ships its own `info_training` layout, should the plugin inject its layout anyway and
document the conflict, or detect it and skip injection (the other mod's HUD stays and our menu is off)?
**Recommendation: inject anyway, document it.** The install line notes the other layout. Cost: that mod's
training HUD does not show while this plugin is installed.

![B3 layout mods](decisions/B3-layout-mods.png)

### B4 · P1 driven by an AI (`SPEC-cpu-01` question 7, `SPEC-input-01` question 7)

When the pilot or the fallback drives P1: should the CPU settings (Mash, DI, SDI, ledge, tech, shield …)
apply to P1 too, and what should Record do?
**Recommendation: the CPU settings apply to the CPU role only, and Record is refused with a notice.** Two
drivers never fight over P1, and no new game site is needed. Cost: recording the AI's line would need the
alternative (reading P1's controller slot).

![B4 P1 AI](decisions/B4-p1-ai.png)

### B5 · Input playback and the CPU settings (`SPEC-cpu-02` question 7, `SPEC-cpu-04` question 6, `SPEC-cpu-06` question 7)

While a recording plays on the CPU, should the DI, SDI, air-dodge, tech and throw settings override it
(upstream), or should the recording win?
**Recommendation: the recording wins**, the same way shield and stick already yield to playback. Cost: none;
the settings apply again when playback ends.

![B5 playback](decisions/B5-playback-vs-settings.png)

**Decided 2026-11-01: the recording wins by default,** and an advanced row *Settings override playback* (default Off) restores the upstream rule when switched On (N4).

### B6 · Ice Climbers state (cross-cutting; `SPEC-cpu-02` … `-06` question 4 or 5)

Should Popo and Nana share one set of CPU counters and cases (upstream), or have one each?
**Recommendation: one per fighter.** Nana's SDI count, DI case and delays then stop leaking into Popo's.
Cost: a few bytes per CPU fighter.

![B6 per fighter](decisions/B6-per-fighter-state.png)

### B7 · In / Out for Airdodge and Shield Tilt (`SPEC-cpu-02` question 2, `SPEC-cpu-05` question 2)

Upstream's air dodge mirrors only when the player is on the CPU's right, and its shield tilt is absolute.
Should both use the opponent-relative rule that DI and SDI use?
**Recommendation: yes, toward the opponent everywhere.** The direction grid labels its chips In and Out, so
they should mean the same on every row and on both sides. Cost: a visible change against the fork when the
player stands on the CPU's left.

![B7 in out](decisions/B7-in-out-convention.png)

### B8 · Behaviour changes from the fork, as a batch

The specs recommend thirteen small user-visible fixes (top of the picture) and six behaviours to keep as
the fork has them (bottom).
**Recommendation: accept the batch.** Any single line can be reversed by naming it. Each line's spec
question has the details. Cost: each fix is a small parity change against the fork, and the release notes
list them.

![B8 behaviour changes](decisions/B8-behaviour-changes.png)

**Decided 2026-11-01: the batch is accepted with two changes.** *SDI not set* keeps the fork's rule: with no SDI Direction selected the CPU does not SDI (the "native CPU SDI" line is withdrawn). *Natural deaths in Training* becomes the toggle *Respawn invincibility* (N2). The other twelve changes and five kept lines stand.

### B9 · Upstream developer-only code (`PROVENANCE.md` questions 3 and 4)

Drop `dev_config`, the debug hooks and `raygun_printer` from the shipped build, or keep them as diag-only
behaviour? Should `training_mod_sync` (36 lines of lock helpers) be kept, or should the code use
`parking_lot` directly?
**Recommendation: drop them, and add a diag version only if a developer asks; use `parking_lot` directly.**
Cost: none for players.

![B9 dev code](decisions/B9-dev-only-code.png)

**Decided 2026-11-01: the Developer page** (N1) holds the developer-only rows with Preload Samples; they ship hidden. `parking_lot` is used directly (the helper crate is not kept).

### B10 · README media (`PROVENANCE.md` question 2)

Reuse the fork's walkthrough GIFs now (some show the old menu), reuse only those without the menu, or ship
no media until the new menu is built?
**Recommendation: reuse the GIFs that show no menu now, and re-record the menu GIFs after the build.**
Cost: the README has no menu pictures until then.

![B10 README media](decisions/B10-readme-media.png)

### B11 · Ledge options changed while the CPU is already on the ledge (`SPEC-cpu-03`)

**Applies to both the fork (unchanged there) and the clean room.** Found while checking Q18: a CPU rolls its
ledge option once per hang and holds it until it leaves the ledge; the held roll also survives while the
Ledge Options row is empty. A CPU already parked on the ledge (for example with Wait) therefore ignores a
newly selected option until it leaves the ledge and grabs again. The throw option behaves the same way for a CPU already
holding someone. The fork's SDI and Shield Tilt rolls no longer have this problem (Q18).
Options: (a) re-roll the ledge options (and the throw options) when the menu changes them, like SDI
Direction: the CPU takes the new option during the same hang; (b) apply on the next ledge grab, which is
what the fork does today and what `SPEC-cpu-03` currently says.
**Recommendation: (a).** The menu then does what it says without a second hang, and it follows the same
rule as Q18. Cost: one more roll cleared on a menu change; a CPU already hanging can act differently the
moment the row changes; the fork keeps (b) until it gets the same change. Source: the fork's
`c1162/di-sdi-apply/PROGRESS.md`, "For the primary" item 2.

![B11 ledge change](decisions/B11-ledge-change.png)

**Decided 2026-11-02: (a), re-roll on menu change,** for the ledge options and the throw options, in both trees. The fork gets it in its next build; the clean room implements it from `SPEC-cpu-03` / `SPEC-cpu-06` in package 2.

## Already answered (not repeated above)

| Item | Answer | Where |
| --- | --- | --- |
| MIT copyright holder | the handle (`Copyright (c) 2026 intpa`) | `LICENSE`; `PROVENANCE.md` question 1; `MENU-DRAFT.md` §4 |
| Storage root and import | `sd:/ultimate/TrainingRooms/`, one-time copy of the fork's data | `specs/SPEC-core-08`; `MENU-DRAFT.md` §4 (also closes `SPEC-core-01` Q2, `SPEC-core-04` Q3, `SPEC-input-01` Q15, `SPEC-state-01` Q9) |
| Q13 grid picture source | stage thumbnails read at run time, never baked; item grid uses the fork's own icons | `MENU-DRAFT.md` draft 5; `SPEC-core-07` §4.5 |
| Placeholder chips (Kirby Hats, Solo Popo, items) | neutral placeholder until the icons are designed | `MENU-DRAFT.md` draft 4 |
| Origin tags on screens | removed | `docs/MENU-PROPOSAL.md` v6 |
| DI / SDI chips | the game's own visual, read at run time | `MENU-DRAFT.md` draft 6; `SPEC-core-07` §4.6 (Q17 asks only whether Airdodge / Shield Tilt keep the arrows) |
| Who drives P1 | the standalone CPU pilot owns P1; only the fallback is open (Q11) | `MENU-DRAFT.md` §2/§3; `specs/INDEX-cpu.md` |
| Menu shortcuts (`PROVENANCE.md` question 5) | merged into Q9 | above |
| Upstream HEAD `077a8e43` inverted reset gate | not adopted | `SPEC-cpu-07` Q5, `SPEC-cpu-01` Q8 |
| Auto-update, Menu Open Start Press | dropped from the menu; old keys read and ignored | `MENU-DRAFT.md` §3; `SPEC-core-02` Q4 |
| Pushing `review/menu-proposal` | docs-only pushes allowed (standing order); any other push, the Forgejo release and the Switch install still need an explicit order | `TRAININGROOMS-HANDOFF.md` |

## Left to the lead

The remaining *Open questions* in `specs/` are technical: site guards, hook placement, measurements, RNG
source, cost-only changes and Eden rows to run. Each has a written recommendation that stands unless the
operator objects; they are not repeated here. The operator-facing ones are pulled into Q18 and S1–B11 above.

## New requirements from the answers

Added 2026-11-01 by the lead from the operator's answers. Each one is folded into `MENU-DRAFT.md` (draft 9), the
affected spec (its *Open questions* answer and §9 test-provenance rows) and the mockups named here
(`docs/menu-mockup/`, the renderer's style).

| # | Requirement | Where | Mockup |
| --- | --- | --- | --- |
| N1 | **Developer page.** A sixth page, *Developer*, after Display & Controls. It is hidden, and its rows do nothing, unless `sd:/ultimate/TrainingRooms/developer.ini` has `developer_page=on` (read at boot; missing file or any other value = hidden). It holds Preload Samples (Q8, moved from *Loading*) and the developer-only rows of B9, rebuilt from `SPEC-core-01`'s description of them. They ship in the normal build, so capturing or testing them needs no diag build; the playbook's `diag` feature keeps only the Eden stand-ins. The operator's card and the Eden test profiles carry `developer_page=on`. | `SPEC-core-01`, `SPEC-core-02`, `SPEC-core-08` | `20-developer.png` |
| N2 | **Respawn invincibility** (toggle, default Off = the fork: instant respawn, no invincibility). On: after a natural death in Training the fighter (P1 or CPU) respawns with the game's own respawn invincibility and, where the game's own respawn path can be used, on the respawn platform as in a match. Greyed while *Auto-load on death* is On (a save state is loaded instead). If the platform cannot be had through the game's own path, that limit goes to the operator before it ships. | Saves › Save states, after *Auto-load on death*; `SPEC-state-01` | `02-saves.png` |
| N3 | **Delay rows: checklist and range** (Q3). Aerial Delay, Fast Fall Delay, Throw Delay, Pummel Delay, Ledge Delay, OoS Offset and Reaction Time open one popup with both controls. A ticked checklist greys the range; a set range (any step value from min to max, uniform) greys the checklist; Y clears the active one. Saved files without a range load as checklists. The three slot sets keep the checklist only (confirmed 2026-11-02). | `SPEC-core-02`, `SPEC-cpu-01`/`-03`/`-05`/`-06` | `19-delay-popup.png` |
| N4 | **Settings override playback** (toggle, *advanced*, default Off). Off: a playing recording wins over DI, SDI, air-dodge, tech and throw settings (B5). On: those settings override the recording, as upstream does. | Recording › Playback; `SPEC-cpu-02`/`-04`/`-06`, `SPEC-input-01` | `03-recording.png` |
| N5 | **Item grid paging** (Q14). Docked: one page, 5 × 8 tiles (38 used), short names. Handheld (the console's operation mode when the grid opens): three pages of 4 × 4 (16, 16, 6) with full names. Stages (2026-11-02): docked 5 × 8 tiles per page (All + 117 = 118 tiles, three pages: 40, 40, 38) with short names; handheld 4 × 4 (eight pages), as the fork. Same tile geometry as the item grid. | `SPEC-core-03`; item grid (carried over) | `11-item-grid.png`, `18-item-grid-handheld.png`, `12-stage-grid.png`, `22-stage-grid-handheld.png` |
| N6 | **Item on load dropped** (Q6). The first-boot import writes the item snapshot of each save that has none, from the old setting, once; the row and its setting are gone. | `SPEC-core-08`, `SPEC-state-04` | `02-saves.png` |
| N7 | **Editable menu shortcuts** (Q9 B). *Menu shortcut 1 / 2* open the chord editor; a change applies at once and is written to `button_config.ini`, the file of record. The permanent `B + D-pad Up` stays fixed. | Display & Controls › Controls; `SPEC-input-04` | `05-display-controls.png` |
| N8 | **Import all replay captures** (S1 item 3). `replay_captures/` is copied in full to the new root on first boot, without blocking the boot; the originals stay. | `SPEC-core-08` | `S1-import-details.png` (as drawn, item 3 changed) |
| N9 | **SDI not set = no SDI** (B8). With no SDI Direction chip selected, the CPU does not SDI, as in the fork. | `SPEC-cpu-02` | – |
| N10 | **SDI chip streaks longer** (Q17). The SDI chips draw the spark's white streaks longer, closer to a match. | `SPEC-core-07` §4.6 | screens `01`, `16`, `17` |

**Answered 2026-11-02 (~02:4xZ):** Q5 and B11 as recommended; the SDI ×2.0 stretch, the Developer page (N1) and the
delay popup (N3, slot sets checklist only) as proposed; N5 extended to the stage grid. Nothing in this list is open.