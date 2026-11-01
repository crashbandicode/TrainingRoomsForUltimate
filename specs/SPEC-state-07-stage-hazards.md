# Stage hazards toggle — behavioural specification

- **Spec id:** `SPEC-state-07`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/hazard_manager/mod.rs`, `src/lib.rs` (install order), `training_mod_consts/src/lib.rs` (Stage Hazards row), `README.md` (Stage Hazards section) at commit `97cbe9b9` (unchanged at `077a8e43`); fork changes at `…/stages/m5i/stage/src/hazard_manager/mod.rs` lines 6-8, 35-43, 97-100, 102-104, 115-116, 149-155 and fork-only `…/stage/src/hazard_manager/policy.rs`, `…/stage/src/common/game_mode.rs` from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

Training mode has no rules screen, so stages always load with their hazards on. This feature lets the player turn hazards off (or on) in Training by writing the game's own "hazards" rule byte at the moment the stage-loading code reads it, so that tournament-legal hazardless layouts can be practised.

## 2. User-visible behaviour

- Menu item (Misc tab): **Stage Hazards**, setting id `stage_hazards`, toggle On/Off, default Off. Help: "Turn stage hazards on/off". README: "Set stage hazards on or off in Training Mode! Use this to practice on tournament legal stages with hazards."
- Takes effect when a stage is loaded (entering Training, changing stage from Training's stage select, or any other stage load the game performs); flipping the toggle mid-session does not change the stage already loaded (open question 1).
- Live in Training only. Outside Training the game's own rule value is never touched. In the fork's waiting rooms the setting is applied to the waiting stage through a different, per-request path (carried over, §6.3).
- If the two code locations cannot be found in the running game, upstream shows the game's error dialog at start ("Failed to apply stage hazard control mods." with the reason and "Hazard control will be disabled for this launch.") and the feature stays off for that launch.

## 3. Inputs

- `stage_hazards` (read inside the hook at each stage load).
- `app::smashball::is_training_mode` at the hook.
- Two addresses found at plugin start by scanning the game's text segment (§5.1); the fork measured the 13.0.5 results.

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| The game's global "hazards enabled" rule byte | one byte at `main+0x530a985` on 13.0.5 (computed as `page(ADRP) + ADRP imm + ADD imm + 0x9` from the matched instruction pair, see §5.1), written as `1` when Stage Hazards is On, `0` when Off | inside the inline hook, i.e. during stage loading, immediately before the game reads the byte | Training mode; both addresses found; the fork performs the write through a bounds-checked native-field writer (folded: every write needs a guard) |
| Hook site | inline hook at `main+0x2311bec` on 13.0.5: the `CBZ` of the matched ADRP / LDRSW / CBZ triple in the stage-loading routine | installed once at plugin start | both addresses found; 64-byte guard and card scan per playbook |

Nothing else is written. The hook does not alter registers.

## 5. Timing and frame rules

### 5.1 Finding the two locations

Upstream does not pin addresses; it scans the whole text segment once at start with an instruction iterator:

- **Flag byte**: the first run of six consecutive instructions `ADRP, ADD, LDUR, LDRB, LDR, SUB`. Taking the `ADRP` at position `p`: `flag = (p & ~0xFFF) + imm(ADRP) + imm(ADD) + 0x9`. The fork's replay on the owned 13.0.5 binary matched at `main+0x17408c4` and resolved to `main+0x530a985`.
- **Hook site**: the first run of three consecutive instructions `ADRP, LDRSW, CBZ`; the site is the `CBZ`. Fork: `main+0x2311bec`.

Either scan failing yields 0 and the validation of §2 fires: error id 1000 for one missing location, 2000 for both. Upstream's two message texts are attached to the wrong conditions (open question 2). This project's playbook replaces the dialog with the install line and a guard result; the recommendation is to pin the two 13.0.5 addresses with 64-byte guards and keep the scans only as a diagnostic cross-check (open question 3).

### 5.2 Install order

Installed before the Training hooks at plugin start (upstream order: hitbox visualisation, hazard manager, training mods). No dependency on the fighter callbacks.

### 5.3 Runtime

The inline hook runs each time the game's stage-loading code passes the site. If in Training: write the byte from the setting. Otherwise: nothing. The game reads the byte right after and builds the stage accordingly. The write is one byte; the hook never allocates.

### 5.4 Setting changed

The byte is written only at the site, so a change applies to the next stage load. The fork's direct practice-stage change reloads the stage and therefore picks up the new value (carried over).

## 6. Edge cases and interactions

1. **Which stages**: every stage whose layout or behaviour the game's "Stage Hazards" rule controls (transformations, moving hazards, stage bosses, etc.); the plugin flips the same rule, nothing per stage. Battlefield and Omega forms are unaffected either way.
2. **Save states** do not interact with hazards; the hazard state is part of the stage, not of the fighters.
3. **Lobby practice** (fork, carried over, interface only): the waiting room gets its hazard setting per stage request (`waiting_hazards(mode) → Some(0|1)` only for recognised waiting-practice scenes with Lobby Practice On, `None` otherwise, so Quickplay and local-wireless matches keep their own rule); the global byte is never written outside Training, so a hazards-On online match is never changed by a hazards-Off practice setting. The fork tests this with a mode/melee-byte table (`policy.rs`), carried over as its own host test.
4. **Fork seam gate** (carried over): the fork installs the hook only when its seam rows `hazard.hook_window`, `hazard.intercept`, `hazard.flag_scan` pass **and** both upstream scans found their addresses; the playbook's guard replaces this.
5. **Another plugin writing the same byte**: last writer wins; out of scope.
6. **Custom stages**: upstream separately patches the stage-select screen to show the Custom Stages tab in Training (`main+0x184dcc8`, a `NOP`); that is a different feature (training/stage-select area), not hazards.

## 7. Persistence

`stage_hazards` in the menu file (carried over). Nothing else.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | minimal; drop the text-segment scan if the addresses are pinned | `tools/build/nro_size.py` per-feature build |
| per-frame work | 0 (stage load only) | 0 | diag counters |
| heap allocations in hooks | 0 | 0 | counting allocator, install line |
| start-up time | two full text-segment scans in upstream | 0 scans if pinned (open question 3) | install line timestamps |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Flag address arithmetic from the ADRP/ADD pair (+0x9) | `state07_flag_address_arithmetic` | — | |
| Scanner finds the two sequences on the 13.0.5 text (fixture) and yields the pinned values | `state07_scan_matches_pinned` | — | |
| Hook writes 1 / 0 from the setting in Training; untouched outside | `state07_write_rule` | `E-state-07-hazards-off-ps2` | |
| Change applies at next stage load | — | `E-state-07-toggle-then-reload` | |
| Missing address → feature disabled, install line reports | `state07_missing_address_disables` | — | |
| Waiting-room per-request policy table (fork, carried) | `waiting_hazards` (existing fork test) | `E-state-07-lobby-no-global-write` | |

## 10. Open questions

1. When exactly the game reads the byte (stage load only, or also on transformations) is inferred from the hook site's location in the stage-loading routine. Recommendation: confirm in Eden that toggling mid-match has no effect until the next load, and document it.
2. Upstream's error texts are swapped: a missing flag address prints "unable to locate stage loading code" and a missing hook site prints "unable to locate the global hazard address". Recommendation: attach the right text to each condition.
3. Upstream scans the entire text segment twice at start to find the two sites; the fork pins the hook site (`main+0x2311bec`) but still runs both scans and requires them to succeed. Recommendation: pin both 13.0.5 addresses with 64-byte guards (the playbook's standard), keep the scans only in the diag build as a cross-check, and treat a guard failure as "feature off, install line records it" instead of a modal dialog.
4. Whether the byte at `main+0x530a985` is the rule's single authority or a per-load copy (which is why the write has to happen at the read site) is not stated by upstream. Recommendation: note it in the site table as "written at read site; do not write elsewhere".
