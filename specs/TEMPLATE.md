# <Feature name> — behavioural specification

- **Spec id:** `SPEC-<area>-<nn>`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft | reviewed | accepted
- **Spec writer:** <agent id / model>, <date>
- **Upstream reference read (spec writer only):** `<upstream path>` at commit `97cbe9b9` (and `077a8e43` where it differs); fork changes at `<stage path>` lines `<ranges>` from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

What the player gets, in plain words.

## 2. User-visible behaviour

- Menu items: name, type (toggle / multi-select / slider / dropdown), values, default.
- Button chords and when they are live (Training only, waiting room, both).
- Visible outputs: HUD text, notifications, effects.

## 3. Inputs

Every input the feature reads: menu settings, controller state, game state
(fighter status, frame counters, positions, damage), files on the SD card.

## 4. Outputs and native state touched

For each write into the game, one row:

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |

Name the game functions and structures by their role; give the 13.0.5 address
where the fork has one. The implementer will build the site table from this
section; every site needs a 64-byte guard and a card scan (playbook).

## 5. Timing and frame rules

When in the frame the feature acts (which hook: once-per-frame, status change,
input poll, HUD draw), ordering against other features, and what happens on
the frame a setting changes.

## 6. Edge cases and interactions

Fighter-specific cases (Pokémon Trainer, Ice Climbers, Kirby, Mii, Aegis,
Steve, Olimar, …), save-state interaction, menu open/closed, lobby practice,
game pause, L+R+A reset, hot swap, CPU-as-P1.

## 7. Persistence

What is saved, where (`sd:/ultimate/TrainingRooms/…`), format (JSON), and what
happens with a missing or malformed file.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | | | `tools/build/nro_size.py` per-feature build |
| per-frame work | | | diag counters |
| heap allocations in hooks | | 0 | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |

Every row in §2 and §4 must appear here before implementation starts.

## 10. Open questions

For the operator or the lead, numbered.
