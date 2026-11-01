# Menu mockup screens (proposal v4, in-game look, 1280x720)

Rendered by `tools/render_menu_mockup.py` from MENU-DRAFT.md §2. Pages show the group that best shows the new controls; the help bar shows the focused item's text. The portrait grid and the stage grid use the real fighter icons and stage thumbnails, cropped from captures of the fork's own grid pickers (table below). The Kirby Hats, Solo Popo and character-item chips are a neutral placeholder: their final icons are open and will be designed once the functionality is settled. Item icons on the item grid are still stand-ins; the direction glyphs are the clean room's own vector art.

- `01-cpu.png` / `.svg` — page CPU (Defence group: direction grids)
- `02-saves.png` / `.svg` — page Saves
- `03-recording.png` / `.svg` — page Recording
- `04-fighters-stages.png` / `.svg` — page Fighters & Stages (Player 1 and CPU: portrait pickers, cycle preview, Solo Popo placeholder)
- `05-display-controls.png` / `.svg` — page Display & Controls
- `06-situation-overrides.png` / `.svg` — sub-screen Situation overrides (CPU > Reactions)
- `07-button-chords.png` / `.svg` — sub-screen Button chords (Display & Controls > Controls)
- `08-swap-groups.png` / `.svg` — sub-screen Swap groups (Fighters & Stages > Loading)
- `09-direction-grid.png` / `.svg` — sub-screen DI Direction: the 3x3 direction grid with absolute Left/Right and weights
- `10-character-grid.png` / `.svg` — sub-screen Character Cycles: the fork's 13x7 portrait grid with P1 (red) and CPU (blue) order badges
- `11-item-grid.png` / `.svg` — sub-screen Give item: 4x8 item grid, page 1 of 2
- `12-stage-grid.png` / `.svg` — sub-screen Lobby Stages: 4x4 stage thumbnail page with All, order badges and form tags
- `13-kirby-hats-grid.png` / `.svg` — sub-screen Kirby Hats: the same portrait grid as a two-role hat order
- `14-fighters-extras.png` / `.svg` — page Fighters & Stages (Extras group: Kirby Hats and Give item placeholders, Mii lists)
- `15-fighters-stage-rows.png` / `.svg` — page Fighters & Stages (Stages group: stage order preview)
- `16-chip-sheet.png` / `.svg` — review sheet: the placeholder chip at full and row size for its three uses, and the direction chip set

## Image sources (review mockup only, not shipped artwork)

The crops in `captures/` are game-asset captures (stock fighter icons, stage thumbnails) as the fork's own grid pickers show them in Training. They exist for this review mockup only; the plugin never ships them. Written by `tools/crop_menu_captures.py` (pins each source by sha256); the manifest is `captures/crops.json`. Eden captures are 1920x1080 window grabs with the game frame at (361,181) scaled 1198x674; the two `docs/feature-walkthroughs/` images are the fork's own 1280x720 frames (byte-identical in the fork worktree). Upstream's menu icons are not used (PROVENANCE §C).

| source | file | sha256 |
| --- | --- | --- |
| change-player | `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/gs-panic-pool14/evidence-m5f/surface/captures/072638-gsp9-p1grid.png` | `c0a2e5b48e3a80ba1a6244b1c617fbebfa385935e9a2baac1243242be5e878fb` |
| change-cpu | `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/gs-panic-pool14/evidence-m5h/surface/captures/121311-s40-cpugrid.png` | `50869bfcfa1f05e502ab7648a1049d9b9513149ebea88c446862b9b7bd39df53` |
| kirby-grid | `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/stages/m5k/stage/docs/feature-walkthroughs/kirby-grid.png` | `dda1ba4b0112ba92eb4106a0a51beac4a9efd0aa771428b5852fc8f3d363cba8` |
| stage-rotation | `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/stages/m5k/stage/docs/feature-walkthroughs/stage-rotation.png` | `84dbcd0e8da2e657c26251170843ff54bdb53e5f7b369e4977d1bdfa8092d412` |

Crop boxes (x0, y0, x1, y1 in source pixels). Portraits come from `change-player` except cells that carry the cursor or an order badge there (Fox, Sora: `change-cpu`) and the ALL cell (`kirby-grid`); Waiting Room has no picture in the fork's stage grid and none here.

| cell | crop | source | box |
| --- | --- | --- | --- |
| Mario | `captures/portraits/mario.png` | change-player | (438, 275, 494, 331) |
| Donkey Kong | `captures/portraits/donkey-kong.png` | change-player | (520, 275, 576, 331) |
| Link | `captures/portraits/link.png` | change-player | (602, 275, 659, 331) |
| Samus | `captures/portraits/samus.png` | change-player | (685, 275, 741, 331) |
| Dark Samus | `captures/portraits/dark-samus.png` | change-player | (767, 275, 823, 331) |
| Yoshi | `captures/portraits/yoshi.png` | change-player | (850, 275, 906, 331) |
| Kirby | `captures/portraits/kirby.png` | change-player | (932, 275, 988, 331) |
| Fox | `captures/portraits/fox.png` | change-cpu | (1014, 275, 1070, 331) |
| Pikachu | `captures/portraits/pikachu.png` | change-player | (1097, 275, 1153, 331) |
| Luigi | `captures/portraits/luigi.png` | change-player | (1179, 275, 1235, 331) |
| Ness | `captures/portraits/ness.png` | change-player | (1261, 275, 1318, 331) |
| Captain Falcon | `captures/portraits/captain-falcon.png` | change-player | (1344, 275, 1400, 331) |
| Jigglypuff | `captures/portraits/jigglypuff.png` | change-player | (1426, 275, 1482, 331) |
| Peach | `captures/portraits/peach.png` | change-player | (438, 340, 494, 396) |
| Daisy | `captures/portraits/daisy.png` | change-player | (520, 340, 576, 396) |
| Bowser | `captures/portraits/bowser.png` | change-player | (602, 340, 659, 396) |
| Ice Climbers | `captures/portraits/ice-climbers.png` | change-player | (685, 340, 741, 396) |
| Sheik | `captures/portraits/sheik.png` | change-player | (767, 340, 823, 396) |
| Zelda | `captures/portraits/zelda.png` | change-player | (850, 340, 906, 396) |
| Dr. Mario | `captures/portraits/dr-mario.png` | change-player | (932, 340, 988, 396) |
| Pichu | `captures/portraits/pichu.png` | change-player | (1014, 340, 1070, 396) |
| Falco | `captures/portraits/falco.png` | change-player | (1097, 340, 1153, 396) |
| Marth | `captures/portraits/marth.png` | change-player | (1179, 340, 1235, 396) |
| Lucina | `captures/portraits/lucina.png` | change-player | (1261, 340, 1318, 396) |
| Young Link | `captures/portraits/young-link.png` | change-player | (1344, 340, 1400, 396) |
| Ganondorf | `captures/portraits/ganondorf.png` | change-player | (1426, 340, 1482, 396) |
| Mewtwo | `captures/portraits/mewtwo.png` | change-player | (438, 406, 494, 462) |
| Roy | `captures/portraits/roy.png` | change-player | (520, 406, 576, 462) |
| Chrom | `captures/portraits/chrom.png` | change-player | (602, 406, 659, 462) |
| Mr. Game & Watch | `captures/portraits/mr-game-watch.png` | change-player | (685, 406, 741, 462) |
| Meta Knight | `captures/portraits/meta-knight.png` | change-player | (767, 406, 823, 462) |
| Pit | `captures/portraits/pit.png` | change-player | (850, 406, 906, 462) |
| Dark Pit | `captures/portraits/dark-pit.png` | change-player | (932, 406, 988, 462) |
| Zero Suit Samus | `captures/portraits/zero-suit-samus.png` | change-player | (1014, 406, 1070, 462) |
| Wario | `captures/portraits/wario.png` | change-player | (1097, 406, 1153, 462) |
| Snake | `captures/portraits/snake.png` | change-player | (1179, 406, 1235, 462) |
| Ike | `captures/portraits/ike.png` | change-player | (1261, 406, 1318, 462) |
| Pokémon Trainer | `captures/portraits/pokemon-trainer.png` | change-player | (1344, 406, 1400, 462) |
| Diddy Kong | `captures/portraits/diddy-kong.png` | change-player | (1426, 406, 1482, 462) |
| Lucas | `captures/portraits/lucas.png` | change-player | (438, 471, 494, 527) |
| Sonic | `captures/portraits/sonic.png` | change-player | (520, 471, 576, 527) |
| King Dedede | `captures/portraits/king-dedede.png` | change-player | (602, 471, 659, 527) |
| Olimar | `captures/portraits/olimar.png` | change-player | (685, 471, 741, 527) |
| Lucario | `captures/portraits/lucario.png` | change-player | (767, 471, 823, 527) |
| R.O.B. | `captures/portraits/r-o-b.png` | change-player | (850, 471, 906, 527) |
| Toon Link | `captures/portraits/toon-link.png` | change-player | (932, 471, 988, 527) |
| Wolf | `captures/portraits/wolf.png` | change-player | (1014, 471, 1070, 527) |
| Villager | `captures/portraits/villager.png` | change-player | (1097, 471, 1153, 527) |
| Mega Man | `captures/portraits/mega-man.png` | change-player | (1179, 471, 1235, 527) |
| Wii Fit Trainer | `captures/portraits/wii-fit-trainer.png` | change-player | (1261, 471, 1318, 527) |
| Rosalina & Luma | `captures/portraits/rosalina-luma.png` | change-player | (1344, 471, 1400, 527) |
| Little Mac | `captures/portraits/little-mac.png` | change-player | (1426, 471, 1482, 527) |
| Greninja | `captures/portraits/greninja.png` | change-player | (438, 537, 494, 593) |
| Palutena | `captures/portraits/palutena.png` | change-player | (520, 537, 576, 593) |
| PAC-MAN | `captures/portraits/pac-man.png` | change-player | (602, 537, 659, 593) |
| Robin | `captures/portraits/robin.png` | change-player | (685, 537, 741, 593) |
| Shulk | `captures/portraits/shulk.png` | change-player | (767, 537, 823, 593) |
| Bowser Jr. | `captures/portraits/bowser-jr.png` | change-player | (850, 537, 906, 593) |
| Duck Hunt | `captures/portraits/duck-hunt.png` | change-player | (932, 537, 988, 593) |
| Ryu | `captures/portraits/ryu.png` | change-player | (1014, 537, 1070, 593) |
| Ken | `captures/portraits/ken.png` | change-player | (1097, 537, 1153, 593) |
| Cloud | `captures/portraits/cloud.png` | change-player | (1179, 537, 1235, 593) |
| Corrin | `captures/portraits/corrin.png` | change-player | (1261, 537, 1318, 593) |
| Bayonetta | `captures/portraits/bayonetta.png` | change-player | (1344, 537, 1400, 593) |
| Inkling | `captures/portraits/inkling.png` | change-player | (1426, 537, 1482, 593) |
| Ridley | `captures/portraits/ridley.png` | change-player | (438, 602, 494, 658) |
| Simon | `captures/portraits/simon.png` | change-player | (520, 602, 576, 658) |
| Richter | `captures/portraits/richter.png` | change-player | (602, 602, 659, 658) |
| King K. Rool | `captures/portraits/king-k-rool.png` | change-player | (685, 602, 741, 658) |
| Isabelle | `captures/portraits/isabelle.png` | change-player | (767, 602, 823, 658) |
| Incineroar | `captures/portraits/incineroar.png` | change-player | (850, 602, 906, 658) |
| Piranha Plant | `captures/portraits/piranha-plant.png` | change-player | (932, 602, 988, 658) |
| Joker | `captures/portraits/joker.png` | change-player | (1014, 602, 1070, 658) |
| Hero | `captures/portraits/hero.png` | change-player | (1097, 602, 1153, 658) |
| Banjo & Kazooie | `captures/portraits/banjo-kazooie.png` | change-player | (1179, 602, 1235, 658) |
| Terry | `captures/portraits/terry.png` | change-player | (1261, 602, 1318, 658) |
| Byleth | `captures/portraits/byleth.png` | change-player | (1344, 602, 1400, 658) |
| Min Min | `captures/portraits/min-min.png` | change-player | (1426, 602, 1482, 658) |
| Steve | `captures/portraits/steve.png` | change-player | (602, 668, 659, 724) |
| Sephiroth | `captures/portraits/sephiroth.png` | change-player | (685, 668, 741, 724) |
| Pyra / Mythra | `captures/portraits/pyra-mythra.png` | change-player | (767, 668, 823, 724) |
| Kazuya | `captures/portraits/kazuya.png` | change-player | (850, 668, 906, 724) |
| Sora | `captures/portraits/sora.png` | change-cpu | (932, 668, 988, 724) |
| Mii Brawler | `captures/portraits/mii-brawler.png` | change-player | (1014, 668, 1070, 724) |
| Mii Swordfighter | `captures/portraits/mii-swordfighter.png` | change-player | (1097, 668, 1153, 724) |
| Mii Gunner | `captures/portraits/mii-gunner.png` | change-player | (1179, 668, 1235, 724) |
| ALL | `captures/portraits/all.png` | kirby-grid | (962, 520, 1022, 580) |
| ALL | `captures/stages/all.png` | stage-rotation | (136, 106, 223, 194) |
| Battlefield | `captures/stages/battlefield.png` | stage-rotation | (741, 106, 845, 194) |
| Small Battlefield | `captures/stages/small-battlefield.png` | stage-rotation | (1048, 106, 1152, 194) |
| Big Battlefield | `captures/stages/big-battlefield.png` | stage-rotation | (128, 240, 232, 327) |
| Final Destination | `captures/stages/final-destination.png` | stage-rotation | (435, 240, 539, 327) |
| Peach's Castle | `captures/stages/peach-s-castle.png` | stage-rotation | (741, 240, 845, 327) |
| Kongo Jungle | `captures/stages/kongo-jungle.png` | stage-rotation | (1048, 240, 1152, 327) |
| Hyrule Castle | `captures/stages/hyrule-castle.png` | stage-rotation | (128, 373, 232, 460) |
| Super Happy Tree | `captures/stages/super-happy-tree.png` | stage-rotation | (435, 373, 539, 460) |
| Dream Land | `captures/stages/dream-land.png` | stage-rotation | (741, 373, 845, 460) |
| Saffron City | `captures/stages/saffron-city.png` | stage-rotation | (1048, 373, 1152, 460) |
| Mushroom Kingdom | `captures/stages/mushroom-kingdom.png` | stage-rotation | (128, 506, 232, 593) |
| Princess Peach's Castle | `captures/stages/princess-peach-s-castle.png` | stage-rotation | (435, 506, 539, 593) |
| Rainbow Cruise | `captures/stages/rainbow-cruise.png` | stage-rotation | (741, 506, 845, 593) |
| Kongo Falls | `captures/stages/kongo-falls.png` | stage-rotation | (1048, 506, 1152, 593) |
