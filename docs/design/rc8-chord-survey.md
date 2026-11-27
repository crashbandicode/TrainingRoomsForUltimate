# Card survey for the rc.8 button chords

This read-only survey on 2026-11-11 informed the [rc.8 bind decision](rc8-binds.md) at 20:01Z.

## Sources checked

| Source | Extent |
| --- | --- |
| Card configs | `/run/media/intpa/SWITCH SD/ultimate/`, as exposed by the user. |
| Plugin binaries | Strings from 19 NROs in `atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`. |
| Tools | `nro_chords.py`, `nro_grep.py`, `nro_context.py`. |
| Search receipts | `artifacts/phase23/feature-gifs-rc8/survey-tools/`, with the README searches. |
| Documentation | Plugin READMEs available on the machine that day. |
| Scope | R with D-pad Up, Down, Left and Right. No mounting, unmounting or card writes. |

## Overlaps on the surveyed card

| Plugin | Control on 2026-11-11 | Finding |
| --- | --- | --- |
| Online Deluxe 0.10.4 | L + R + Down: next window page | Flipped Input display (R + Down) on both `fx-record-refused` takes. |
| `libssbu_online_deluxe_rewrite.nro` | README and NRO named the page chord. | Both sources agreed. |
| Online Deluxe, Performance | L + R + Up: 30-second capture | Could open Game speed (R + Up). NRO string at `0xf201c`. |
| Online Deluxe | L + R + X: toggle framed icon/profile | No R-and-D-pad overlap. README and NRO string. |
| Online Deluxe | ZR + Left/Right: edit; plain Up/Down: select | No R overlap. README and NRO string. |
| TrainingRooms, rc.7 | ZR + Left/Right | Already used for Play on Player 1 and Recording popup. |
| CPU pilot 0.2.3 | L + R + Down; ZR + Left/Right; L + R + X | Same Deluxe-window overlap, per `libssbu_cpu_pilot.nro`’s README. |
| Stages + Counterpick | L + R + Down; ZR + Left/Right | Same overlap on its Deluxe-window page, per README. |
| `libelite_stages_counterpick.nro` | `elite-legal-stages-public/settings.toml` named L+R+Up. | Stale comment. Only Deluxe’s NRO contained that string. |
| Smash Replay to MP4 | L + R + A + Plus: quit playback | No R-and-D-pad overlap. README and inventory. |
| Smash Replay to MP4 | X + Down: hide display | No R-and-D-pad overlap. README and inventory. |
| QuickPlay cancel fix | No chord in six source files | Its `config.ini` contained only `hold_seconds`. |
| TrainingRooms on survey card | Minus, configured menu shortcut; B + Up, permanent shortcut | From `button_config.ini` and `model::chords`. |
| TrainingRooms on survey card | L + R + A, optional reset; eight ZL/ZR + D-pad rows | Neither overlapped the two new defaults. |
| TrainingRooms rc.8 | R + Down, Input display; R + Up, Game speed | The two new rows were the survey subjects. |

## Plugins without a documented chord on the survey date

| Sources checked | Plugins |
| --- | --- |
| NRO strings, configs and local READMEs | ARCropolis, css_preserve, elite_rematch, gsp_visualizer, imgui_smash |
| Same sources | instant_back, memory_guard, nro_hook, param_hook, smash_gamepad_overlay |
| Same sources | ssbu_desync_detector, ssbu_quiet_elite_switch, ssbu_refresh_clock |

A compiled button mask could exist without any string; these plugins’ sources were unavailable on the survey machine.

## Decision and later behavior

| State | Result |
| --- | --- |
| rc.8, L held | Protected both default R + D-pad chords regardless of press order. |
| rc.8, L released but R held | Pressing Up/Down still matched TrainingRooms’ default chords. Deluxe documented plain Up/Down for selection. |
| Shipped overlay handling | The detector also checks whether Deluxe owns row selection while its overlay is open. |
