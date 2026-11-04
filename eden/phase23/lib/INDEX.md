# Phase 2/3 Eden route library

One line per reusable piece: file — what it does — start state → end state — host(s) proven on. Read this before
building a route; add to it as soon as something works (playbook WORKFLOW "Running agent workers without churn").

- `../rows-A/segments/tr-menu-open.edenmacro.json` — opens the TrainingRooms menu (B+Up, same frame) — Training HUD → menu open — Yoga
- `../rows-A/segments/tr-menu-close.edenmacro.json` — closes the menu — menu open → Training HUD — Yoga
- `../rows-A/segments/intro-cancel.edenmacro.json` — skips the intro — boot intro → title — Yoga
- `../rows-A/segments/tr2-fsmash.edenmacro.json`, `tr2-jump-nair.edenmacro.json` — P1 attacks for hit rows — Training HUD → Training HUD — Yoga
- `../rows-A/segments/lr04-return-cpu-pick.edenmacro.json`, `lr10-p1-short.edenmacro.json`, `hat-next.edenmacro.json` — carried lr routes — see the files — Yoga
- Ally same-frame chords: `bc-*.edenmacro.json` in `C:\Eden\glm-bc\` on the Ally (to be copied here by the Ally worker) — Ally