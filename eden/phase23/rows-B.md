# Phase 23 — worker B's Eden rows (prepared 2026-11-04, branch `p23/b4-hats`)

Prepared rows for the batched validation boot (one boot per STATUS batching rules). Rows read the
game's state via the plugin's diag lines, never fixture indexes. Build: the phase candidate's diag
NRO (worker B's build: release `cc2bf0a0…`, diag `6bea7449…`, branch `p23/b4-hats`); rows 3 and 4
need the diag build (the `role_body` line is a `[KirbyHats]` shipped-tag line, but the trace build is
the one the batch boots anyway). Pin `libarcropolis.nro` from the card when a row depends on mods in
`ultimate/mods` (COMMON-RULES). Ruleset: 1 stock, 1:00, items/hazards/FS off.

## B4-1 · Record refusal notice under the P1 pilot (IN01-E19, `SPEC-input-01:239,249`)

- **Setup:** fixture `TrainingRooms/settings.json` with lobby practice/scene state as in glm-03's
  fixture flow; Training mode, ready-go; the P1 pilot armed (pilot feature with its fallback active —
  the row runs once with the pilot driving P1, the AI visible acting on the display).
- **Inputs/macro:** the record chord (L+R chord per the recorder's trigger: Special+… — the recorder's
  `trigger_chord` row), one attempt.
- **Pass:** `notice_draw header="Input Recording" body="P1 is controlled by an AI: recording is off"`
  in the diag log within ~1 s of the chord; no `record_lockout` event for that chord; the phase tint
  never shows; a following playback of the target slot refuses (slot empty).
- **Evidence:** the chord-window diag lines (notice_draw + absence of record_lockout), the row's
  screenshot at the notice, `out/<row>/` copies.

## B4-2 · CPU settings never entry 0 under the pilot (`SPEC-cpu-01:241,251`, DECISIONS B4)

- **Setup:** same fixture state as B4-1; CPU role entry 1 (Training) with Mash set to a visible
  action and DI set; P1 driven by the AI.
- **Inputs/macro:** none (observe ~10 s of live frames), or one mash-triggering situation via the
  CPU's own play.
- **Pass:** the CPU body (entry 1) executes the mash/DI; P1's body never receives an injected write:
  `input_hook_counts … inject=0` stays for the whole window and P1's actions remain the AI's own
  (`[CpuPilot]` lines show the pilot's decisions driving P1, no `Kirby`/mash notice for P1).
- **Evidence:** the `input_hook_counts` lines over the window, the CPU-side behaviour line(s), the
  screenshot pair (P1 idle of mash, CPU mashing).

## B4-3 · CPU-Kirby hat switch, Behavior=Normal (P1's chord drives the CPU's hat)

- **Setup:** fixture roster: P1 a non-Kirby fighter, CPU Kirby (catalog Kirby for the CPU cycle) —
  the glm-03 fixture `glm03-kirby-both.json` shape, but P1's catalog entry is a fighter whose hat is
  not needed, so a P1 hat chord is never confused with this row. Training mode, ready-go, HUD settled.
- **Inputs/macro:** P1's right chord: Special held + right stick click (the glm-03 proven chord),
  one attempt per press.
- **Pass (two outcomes, both recorded):**
  - the request forms: `[KirbyHats] request role=1 kind=0x… source_slot=…` then
    `[KirbyHats] confirmed role=1 …` and `notice_draw header="CPU Kirby Hats" body="<hat>"`; or
  - the new diagnostic lands instead: `[KirbyHats] role_body role=1 case=kind kind_seen=0x… …` or
    `[KirbyHats] role_body role=1 case=accessor …` — the row then records the case and the kind it
    saw (that is the root-cause fact this boot exists to capture), and the row is deferred, not
    passed.
- **Evidence:** the diag lines for the chord window, the trace JSONL if `trace-B.md` points were
  armed, the screenshot at the notice (or its absence).

## B4-4 · CPU-Kirby hat switch, Behavior=Control (the operator's own controller)

- **Setup:** as B4-3, plus the CPU's Behavior set to Control and the operator controller bound
  (`practice::cpu_operator` path, `handle_cpu_operator_input`); the plugin's menu shows the operator
  role.
- **Inputs/macro:** the operator controller's own-fighter alias with the left stick: Special held +
  left stick click (the operator path's left-click alias), one attempt.
- **Pass:** the same two outcomes as B4-3, on the operator latch (`[KirbyHats] request role=1` from
  the operator path, or the `role_body role=1 case=…` line). This path is unexercised — its chord
  sample runs the same `sample_shortcut`, so the row proves the alias wiring reaches the same lookup.
- **Evidence:** the diag lines naming the operator window, screenshot.

## B4-5 · P1 hats regression check (RC9 `5205a8f` still holds on the new build)

- **Setup:** P1 Kirby (any CPU), Training, ready-go; a hat selected in the menu so the cycle has a
  target.
- **Inputs/macro:** P1's left chord (Special + left stick click), one attempt.
- **Pass:** `[KirbyHats] request role=0 …` then `[KirbyHats] confirmed role=0 …` and
  `notice_draw header="Kirby Hats" body="<hat name>"` (glm-03 proved this on diag `5da1c5f1`; this
  row re-proves it on the phase candidate, including after worker B's `role_lookup` addition, which
  only runs on the failure path).
- **Evidence:** the diag lines, the notice screenshot.
