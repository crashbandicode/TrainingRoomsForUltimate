# Test fixtures

| File | What it is | Origin | sha256 |
| --- | --- | --- | --- |
| `fork-menu-switch-before21.json` | the earlier fork's settings file (`training_modpack_menu.conf`, JSON), from a physical Switch SD backup | `artifacts/switch-before21/ultimate/TrainingModpack/training_modpack_menu.conf` | `9f8598346282292407161bce7efb1a9fcfb91e2baf3d0c1b6e3a18022f661630` |
| `fork-menu-console-crash78.json` | the same file from a later fork build (more rows: rosters, cycles, macro source) | `artifacts/enhancements/console-crash78/logs/TrainingModpack/training_modpack_menu.conf` | `405e37e3789df9f81d137494d128d538545f67e4b285878a6350b94a2f8aaefa` |

Both are copies, byte for byte, of files the fork itself wrote (the fork's own data, ours). They are
the byte-exact samples SPEC-core-04 question 1 asked for.
