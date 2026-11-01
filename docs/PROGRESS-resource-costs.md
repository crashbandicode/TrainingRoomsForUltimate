# PROGRESS — resource-costs.md (research worker)

Started 2026-11-01. Output: `docs/resource-costs.md`. Read-only research; no code, no builds. Only
`nro_size.py` (playbook) was run, on NRO files already in the record, plus one `zlib` size estimate of two
`layout.arc` files.

## Status: DONE (2026-11-01)

- [x] Context read: README, handoff, provenance table (upstream-identical/mixed `src/` rows), playbook WORKFLOW §3–4, `nro_size.py`.
- [x] Whole-plugin image record (Table A): 60+ NRO headers measured with `nro_size.py` (footprint-experiments, every `switch-before-r*` card copy, 7.1.2 before/after, C108–C116.1 card collections, every refactor stage release and diag NRO); documents cross-checked (REVISION29/32/33, INTEGRATION.md:15, RC-SUMMARY.md, SWITCH-INSTALL-C1xx.json, C115-CAMPAIGN.md).
- [x] Per-feature rows (Table B): upstream `97cbe9b` hook/static/thread inventory read from source (66 hooks itemised); fork measurements attached where they exist (input display host bench, TOML/JSON persistence crate cost, layout archive bytes and lifetime, r32→r33 removal delta, 7.1.2 image delta).
- [x] Context table (Table C): 3285 MiB budget, admission rule, 176-kind cost table, PreloadMetrics format, Eden preload table, console headroom samples, hot-swap timings, fork heap statement, diag overhead, fork archives and thread stacks.
- [x] Gaps and methods (9 methods) and a contradictions section.

## Findings worth relaying

- No upstream-only NRO exists in the record; `libtraining_upstream33w.nro` has the fork's 32w header. The
  footprint-experiments `default` variant (1,769,472 B) is the revision 33 tree on the r32 `z`+cgu1 profile
  (`build-default.log`, `post-build-check.json`), not an upstream build.
- The only per-feature numbers the fork measured are: input display renderer work (host fixture), the TOML
  persistence crate (110,592 B on B122), the layout archive (4,213,268 B inflated; ~175–194 KB compressed), the
  analytics+updater+dialog removal (−405,504 B resident, upper bound), and the 7.1.2 input-display image delta
  (+12,288 B). Everything else about upstream-origin features is "not measured".
- Contradictions: NRO-header bss (28,672) vs ELF .bss (26,432) in the B122 heap figures; COMPLIANCE-FINAL.md:9's
  "+758 KB for 's'" is the s-vs-opt3 saving, not s-vs-z (+471,040 B); results.json silently mixes r31 and r33 trees.
