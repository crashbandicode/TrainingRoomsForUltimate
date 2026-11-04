# lr15 visual review — 2026-11-04

Original screenshots are retained under `out/` and the raw archive; `media-index.json` gives hashes. Contact sheets were derived with Pillow, preserving the full captured frames at reduced size; `media_contact.py` reproduces them. No generated footage.

| Evidence | Observation | Disposition |
| --- | --- | --- |
| RC-HATS-P1 confirmed-0-0 through confirmed-0-7 (first five; exact paths in media-index) | Mario red cap, DK hair, Link green cap, Samus helmet, Fox ears/headset on pink P1 Kirby, with Kirby portrait and Lucas CPU | Five distinct visible hats supported |
| Last P1 confirmed-0-0 immediate screenshot | Turning bare Kirby; stale Fox notice | Transitional capture; cannot certify the repeat by itself |
| attempts/copy-loss-cursor/row/yoga-003514Z-060235-gate-training-hud-unpaused.png | Following gate frame shows Mario cap from behind on Kirby, still stale Fox notice | Repeat supported together with confirmed log488; use body, not stale notice |
| RC-HATS-copy-loss native-copy-loss / confirmed-0-0 | Bare pink Kirby after taunt; red Mario cap after reacquisition | Loss/reacquisition visually supported; trace34/35 binds native variant1 |
| RC-KIRBY-ROB seven settled frames | Kirby, Falcon, R.O.B., Lucario, Mr. Game & Watch, Steve, Kirby; corresponding body and P1 HUD portraits | Recorded body/HUD progression supported |
| RC-KIRBY-ROB copy / settled-copied-kirby-away / settled-normal-rob | Mario Kirby → Falcon → normal gray R.O.B., each with matching P1 portrait | Copy departure and R.O.B. regression scoped PASS |
| RC-TEARDOWN-slow36 continuation settled-Captain-Falcon | Falcon on Training stage with matching P1 HUD; Lucas CPU | Fresh-scene continuation visible; pending drain itself comes from diagnostics, not screenshot |

CPU hat visibility is not accepted: neither chord produced a request/confirmation. These stills do not establish cancellation, KO, exact lease-token lifetime, or three independent pending-build interruptions.
