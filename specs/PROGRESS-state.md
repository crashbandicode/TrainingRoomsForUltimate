# Spec progress — area `state` (save states, reset, charges, buffs, items, hazards)

Spec writer: Claude agent (Fable 5.1), 2026-11-01. Sources read: upstream `97cbe9b9` (diffed against `077a8e43`), fork stage M5i, fork docs, provenance table. All seven specs and the index are written; status of every spec is **draft** pending lead review.

| Spec | Status | Notes |
| --- | --- | --- |
| SPEC-state-01-save-load-core.md | written (draft) | 11 open questions; fork fixes folded: marker reset on file read, `init_pos` for grounded restores, merged `EffectModule::req` hooks, null-safe role resolution, session cancel |
| SPEC-state-02-charges.md | written (draft) | full per-fighter table; Olimar line order; Sora rotation; 3 open questions |
| SPEC-state-03-buffs.md | written (draft) | per-fighter steps and the five supporting hooks; fork fixes folded: record-keyed bookkeeping, session clear; 6 open questions |
| SPEC-state-04-character-items.md | written (draft) | 45-entry item table, three grant paths, turnip face hooks; fork snapshot stated as interface with precedence; 4 open questions |
| SPEC-state-05-fighter-specific-restore.md | written (draft) | Pokémon Trainer, Kirby copy + copied-charge table, Pikmin autonomy, Steve, keep-list, Sora/Shulk sounds, status equivalences; fork fixes folded: decide-hook descriptor mapping, copy-setup opponent entry id, Kirby status interval; 6 open questions |
| SPEC-state-06-training-reset-and-lra.md | written (draft) | LRA register rule, reset hub, upstream HEAD inversion flagged; fork fixes folded: native-Training gating; 4 open questions |
| SPEC-state-07-stage-hazards.md | written (draft) | scan arithmetic, pinned 13.0.5 addresses from the fork, per-request lobby policy as interface; 4 open questions |
| INDEX-state.md | written | per-spec upstream files and fork-range classification (feature / fix / gate) |

Not determined from the sources (stated as open questions rather than guessed): true stage centre offsets for Smashville / Town and City / Yoshi's Island; the names behind the Min Min, Kirby, Mewtwo, Lucario, Robin and Sephiroth raw hash40 constants; when exactly the game reads the hazard rule byte; the Pokémon descriptor layout at the decide site (fork observation only); the copy-module offset `+0x17398`.
