# Offline checks — 2026-11-04

- `python3 -B -I eden/lr15-shared/test_review.py`:3 tests PASS. Rejects mismatched body/role/thread/process/point, mutated manifest bytes, ordinary completed construction and residual ledger; nominal cases checked.
- `python3 -B -I eden/lr15-shared/test_timeline.py`:2 tests PASS for retained lr14 timeline scope.
- `python3 -B -I eden/lr15-shared/review.py <raw archive> --out <review.json>`: exit0; full trace pairs equal receipts, every executed hook revision retained, receipt manifests hash-bound, native cleanup and release ordering verified. Health lists empty.
- Canonical `runner.py dry-run eden/lr15-shared/rows.json`: READY/no errors/no missing segments, `final-dry-run.json`. This is the final executed same-boot plan, not a newly authorized fresh-boot route.
- `reconciliation.json`:21receipt bindings,4executed lead.py revisions, final manifest/hook byte equality,18-line RESULTS. Full raw inventory in raw-index.json.
- `compliance.py TrainingRoomsForUltimate-rc9 --nro <RC9 release> --out <raw>`: exit0/no machine failures;32manual rows disposed in COMPLIANCE.md. Explicit model-recheck sequencing failure retained.
- Final `git diff --cached --check`: exit1 on original diagnostic-log trailing whitespace and inherited strict_predicates.py EOF blanks, preserved byte-for-byte in root/attempt snapshots. Authored review/hooks/docs check exit0. No product code edits.

Commands were bounded by systemd memory caps256–512MiB and timeout30–120seconds. Source/build test counts are attributable to build-binding/AUDIT.json, not newly run by this lane.
