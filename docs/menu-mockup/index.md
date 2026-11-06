# Menu mockup screens (proposal v10, in-game look, 1280x720)

Rendered by `tools/render_menu_mockup.py` from MENU-DRAFT.md §2. Pages show the group that best shows the new controls; the help bar shows the focused item's text. The portrait grid uses the real fighter icons, cropped from captures of the fork's own grid pickers. The item grid uses the fork's own 38 item icons, copied unchanged. The stage grid shows captures of the stock thumbnails; the clean room does not bake them but reads each stage's own game UI texture at run time, so a stage mod's picture shows (SPEC-core-07 §4.5). Sources are in the tables below. The Kirby Hats, Solo Popo and character-item chips are a neutral placeholder: their final icons are open and will be designed once the functionality is settled. The DI and SDI Direction chips show the game's own DI streak and SDI spark as renders made from the game's effect file (one still per effect, turned to each direction as the game turns that effect to the launch angle / SDI vector; SPEC-core-07 §4.6); the chip sheet shows crops of real captures beside them as the visual check. The Airdodge Direction and Shield Tilt arrows are the clean room's own vector art. The screens carry no origin tags (upstream / ours, proposal v6): origin is internal, recorded in `PROVENANCE.md` and the provenance table in `docs/MENU-PROPOSAL.md`.

Version 10 (2026-11-01): the SDI chips draw the spark's burst x2.0 longer along the SDI vector (screens 01, 16, 17; the stretches are compared on 21; method and measurements below, `DECISIONS.md` N10); the decided items are drawn (`DECISIONS.md` N1-N7): 02 Respawn invincibility added and Item on load gone, 03 the advanced row Settings override playback, 05 editable menu shortcuts, 11 and 18 the item grid docked (5x8, one page) and handheld (4x4, three pages), 19 the delay popup with checklist and range, 20 the hidden Developer page. Preload Samples left the Loading group (04, 14 and 15 now count 6 rows there).

- `01-cpu.png` / `.svg` — page CPU (Defence group: direction grids; the DI and SDI rows preview their selected chips)
- `02-saves.png` / `.svg` — page Saves (Save states group; the 2026-11-01 screen draws Respawn invincibility focused, Item on load gone). The in-game row is labeled Respawn on platform.
- `03-recording.png` / `.svg` — page Recording (Playback group; the advanced row Settings override playback focused)
- `04-fighters-stages.png` / `.svg` — page Fighters & Stages (Player 1 and CPU: portrait pickers, cycle preview, Solo Popo placeholder)
- `05-display-controls.png` / `.svg` — page Display & Controls (Controls group; Menu shortcut 1 / 2 are editor rows, Permanent shortcut stays info)
- `06-situation-overrides.png` / `.svg` — sub-screen Situation overrides (CPU > Reactions)
- `07-button-chords.png` / `.svg` — sub-screen Button chords (Display & Controls > Controls)
- `08-swap-groups.png` / `.svg` — sub-screen Swap groups (Fighters & Stages > Loading)
- `09-direction-grid.png` / `.svg` — sub-screen DI Direction: the 3x3 direction grid of DI streak chips with absolute Left/Right and weights
- `10-character-grid.png` / `.svg` — sub-screen Character Cycles: the fork's 13x7 portrait grid with P1 (red) and CPU (blue) order badges
- `11-item-grid.png` / `.svg` — sub-screen Give item, docked: one page of 5x8 tiles (38 used) with short names and the fork's own item icons, page 1 of 1
- `12-stage-grid.png` / `.svg` — sub-screen Lobby Stages, docked: 5x8 tiles per page (the docked item grid's geometry), All first, order badges and form tags, short names, page 1 of 3 (40, 40, 38 of All + 117); tiles 17-40 are text placeholders (the mock has captures for the first 16 only); at run time each thumbnail is the stage's own game UI texture (a stage mod's picture shows), here stock captures
- `13-kirby-hats-grid.png` / `.svg` — sub-screen Kirby Hats: the same portrait grid as a two-role hat order
- `14-fighters-extras.png` / `.svg` — page Fighters & Stages (Extras group: Kirby Hats and Give item placeholders, Mii lists)
- `15-fighters-stage-rows.png` / `.svg` — page Fighters & Stages (Stages group: stage order preview)
- `16-chip-sheet.png` / `.svg` — review sheet: the placeholder chip at full and row size for its three uses, the rendered DI and SDI chips with crops of the captures beside them as the check, and the Airdodge / Shield Tilt arrows
- `17-sdi-direction-grid.png` / `.svg` — sub-screen SDI Direction: the 3x3 direction grid of SDI spark chips with absolute Left/Right and weights (same layout as 09)
- `18-item-grid-handheld.png` / `.svg` — sub-screen Give item, handheld: 4x4 tiles with full names, page 1 of 3, with a small handheld tag
- `19-delay-popup.png` / `.svg` — popup Aerial Delay with both controls, two states: (a) checklist active (0, 5, 10 ticked), range greyed; (b) range 0 - 10 active, checklist greyed; Y clears the active one
- `20-developer.png` / `.svg` — sixth page Developer, hidden unless developer.ini has developer_page=on: Preload Samples (moved from Loading), Reload developer config, Debug hooks
- `21-sdi-stretch-compare.png` / `.svg` — review sheet: SDI Up and In chips at 60 and 30 px for the v9 render and four stretches along the SDI vector (x1.17, x1.4, x1.7, x2.0), beside the captured Up and In crops at the same size
- `22-stage-grid-handheld.png` / `.svg` — sub-screen Lobby Stages, handheld: 4x4 tiles per page as the fork, full names, All, order badges and form tags, page 1 of 8, with a small handheld tag

## Image sources (review mockup only, not shipped artwork)

The crops in `captures/` are game-asset captures (stock fighter icons, stock stage thumbnails) as the fork's own grid pickers show them in Training; `captures/items/` holds byte copies of the fork's item icons. They exist for this review mockup only; the plugin never ships these files. Written by `tools/crop_menu_captures.py` (pins each source by sha256); the manifest is `captures/crops.json`. Eden captures are 1920x1080 window grabs with the game frame at (361,181) scaled 1198x674; the two `docs/feature-walkthroughs/` images are the fork's own 1280x720 frames (byte-identical in the fork worktree). Upstream's menu icons are not used (PROVENANCE §C).

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

Item icons (proposal v5): byte copies of the fork's own item-grid pictures, not crops. Source directory `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/stages/m5k/stage/scripts/item-icons/assets` (the frozen M5k stage; byte-identical to the fork worktree's `scripts/item-icons/assets/`), manifest `sources.json` sha256 `8fbed7ab430130d87bf6761994f6b06e911e038148d2d9e9378033ce6f870887`. Each icon's sha256 is checked against that manifest before the copy. The manifest records how the fork made each one: native `item_0` UI icons for Mr. Saturn, Bob-omb and Banana Peel, renders of the item's own game model and textures for the rest.

| # | item | file | sha256 |
| --- | --- | --- | --- |
| 0 | Banjo & Kazooie: Grenade Egg | `captures/items/icon-00.png` | `3d32ba4b3b6aaaa2f796dfbf1eedcf5aea9f342d89e50ea1323831bcc17a1065` |
| 1 | Bowser Jr.: Mechakoopa | `captures/items/icon-01.png` | `90f5a8f7482ae7cdb5c583b7d349f52de90d5a7eab7507bbe5ed993aea73b735` |
| 2 | Daisy: Smile Turnip | `captures/items/icon-02.png` | `ee643fab9b28a2aa17b198e95d5fcefcd7aad694a411f39026438c86a1d918ce` |
| 3 | Daisy: Winky Turnip | `captures/items/icon-03.png` | `ae1128531822d7f46b0117453e126b6d7af701b35e2078158f454f6459d083d0` |
| 4 | Daisy: Dot-Eyes Turnip | `captures/items/icon-04.png` | `ef7967d410d8b5af487973cd48e875a1595906a1a4a7b9fe0d54c9f36f4641d6` |
| 5 | Daisy: Stitch-Face Turnip | `captures/items/icon-05.png` | `4e459691982911c1badaf1a440313b90bf0bca91b0293d9a313e1524e50a36c6` |
| 6 | Daisy: Mr. Saturn | `captures/items/icon-06.png` | `918c87d08f790d805a10ca56b4be71902c90cc174c4fee5f4fdc52146c73bcca` |
| 7 | Daisy: Bob-omb | `captures/items/icon-07.png` | `d4d894be9fa6bc4016c14eec92814fc2c759926948c6a856009fb26e4b973eea` |
| 8 | Diddy Kong: Banana Peel | `captures/items/icon-08.png` | `c1c7d74fc6da1152dede1462ddd1ec731e3edbf093122a00e6e002cc8b6509ca` |
| 9 | Diddy Kong: Peanut | `captures/items/icon-09.png` | `6ce930342a8d715305b177d5fa176b8ae3eafd2c1286b6a2cfae8dfdb7aedb26` |
| 10 | King K. Rool: Crown | `captures/items/icon-10.png` | `f39120f31655bbcd33f3b78cce7789f751b969c0705b463f4e0ad7afd48140eb` |
| 11 | Link: Remote Bomb | `captures/items/icon-11.png` | `826961ac749989535578113b0a4c3314f903d4e7d02b97b185ba9473dcbdcfc9` |
| 12 | Link: Arrow | `captures/items/icon-12.png` | `69e8d6ed411964fa129caedacf494fdafd3b4229238d66562cfa6a15b714c441` |
| 13 | Mega Man: Metal Blade | `captures/items/icon-13.png` | `724282a79bde109d50d30f3376b6d51f3176db9eab972a09bafa567155de07f4` |
| 14 | Pac-Man: Cherry | `captures/items/icon-14.png` | `44447262eb791551f03c6d924a944c2bf43729606a43f3e963bfeeb141dc0437` |
| 15 | Pac-Man: Strawberry | `captures/items/icon-15.png` | `6c00d18a79c59544b2af32b9d6f444a879fa3157a50d192679c98bee34163720` |
| 16 | Pac-Man: Orange | `captures/items/icon-16.png` | `f24254d6fb357a41db5765d0448a6ec103efd0f693f21dfac02b0e52bdc0a2ec` |
| 17 | Pac-Man: Apple | `captures/items/icon-17.png` | `8a98c9b006dd82b2d65e5ee914919ac2a91b20f7c07072d92178ebc5d0882ca5` |
| 18 | Pac-Man: Melon | `captures/items/icon-18.png` | `d763b83fa88d7f9526f40b2c307324a5055199927cd321f759e8e18313d61e76` |
| 19 | Pac-Man: Galaxian | `captures/items/icon-19.png` | `96028383dd49c9aaedca2beda752d5f7424bc63c4f434d52524a4a6d5f2c2392` |
| 20 | Pac-Man: Bell | `captures/items/icon-20.png` | `c6477c1b71920ab76c986dc131bbc6121861b3ba95f5ae459f5a77bd20b74e96` |
| 21 | Pac-Man: Key | `captures/items/icon-21.png` | `2822de83b6eda7b1ff752b85790f94bf9669eea1456b7cf3d1d9854f74c86ab5` |
| 22 | Peach: Smile Turnip | `captures/items/icon-22.png` | `4abbb9a61dcced2c65b3e104588ce19aef3d7c87425aea9c860cfa4cfa495fee` |
| 23 | Peach: Winky Turnip | `captures/items/icon-23.png` | `0cd6f8c78294e6b2d95dfef3d71e3902ccbc39cf6d1e6e86409fcaf5adaf8aa6` |
| 24 | Peach: Dot-Eyes Turnip | `captures/items/icon-24.png` | `aeaa8cac778ea511445db3961373d89ef787e5598077fadb33d083b78b989a02` |
| 25 | Peach: Stitch-Face Turnip | `captures/items/icon-25.png` | `d94484daaeade3fa89ce4d220796166e1ea26525cc6f0e315e581361b0c5fd97` |
| 26 | Peach: Mr. Saturn | `captures/items/icon-26.png` | `918c87d08f790d805a10ca56b4be71902c90cc174c4fee5f4fdc52146c73bcca` |
| 27 | Peach: Bob-omb | `captures/items/icon-27.png` | `d4d894be9fa6bc4016c14eec92814fc2c759926948c6a856009fb26e4b973eea` |
| 28 | Richter: Holy Water | `captures/items/icon-28.png` | `151759e5e2d31d5e7249338326617693789bbf50ee78612c82977024fcfd2456` |
| 29 | R.O.B.: Gyro | `captures/items/icon-29.png` | `aa50909bf4224a22fce1f32fd4f66d1e1d8606955472fa3a02d5c983d6960d6b` |
| 30 | Robin: Tome | `captures/items/icon-30.png` | `5fd3b9493ed7b94365d41d0b568a866b98f3c91996327a7bbdc945bef9efaa28` |
| 31 | Robin: Levin Sword | `captures/items/icon-31.png` | `04c227ea2313c9d73a4ebfbc3bb3bab8449ed25b12258fc9ad97106ce52a7fc1` |
| 32 | Sheik: Burst Grenade | `captures/items/icon-32.png` | `c270c03a96f515285f9301024a455994b20c0d97e9660ae8ebf8e01e50ef63dd` |
| 33 | Simon: Holy Water | `captures/items/icon-33.png` | `151759e5e2d31d5e7249338326617693789bbf50ee78612c82977024fcfd2456` |
| 34 | Snake: Hand Grenade | `captures/items/icon-34.png` | `a589d64af7f963ce7fe7ad3ce185db10c7f0fbbb4c2114832f7958ec23155e56` |
| 35 | Toon Link: Bomb | `captures/items/icon-35.png` | `bcf3878cebb06ba72fa31b9357d9ecf67d35eb40ebb1ffba3e5cf0984b560d7b` |
| 36 | Villager: Wood Chip | `captures/items/icon-36.png` | `0af3312adee081227d9300119e649d886371a075e5b6f648768b7e8a6bfbe2db` |
| 37 | Young Link: Bomb | `captures/items/icon-37.png` | `9bf26f635d1ff93720cbe7d3b06c808c909d62363780ab51113ec9fe72bf1bef` |

DI and SDI chip renders (proposal v9): the DI and SDI chips on 01, 09, 16 and 17 draw game-derived renders made by the c1162 W9 extraction (`c1162/di-sdi-extract/RESULT.md` §4): each effect's own emitter textures, meshes and colours read from the stock 13.0.5 `effect/system/common/ef_common.eff` (DI `P_CmnVector`, SDI `P_CmnHitStopSpark`), composited as one still at particle life t = 0.3 with the effect origin on the chip centre and its reach at 0.46 x the chip size. Only the two Up layers (432 px RGBA light, alpha = max channel) are byte copies here; the renderer turns each to every direction, as the game turns its one effect to the launch angle / SDI vector. Field meanings behind the render are W9 inference, and one SDI emitter (`spark1`) is placed by calibration against the capture (RESULT.md §4.8, §7). The clean room renders the same from the player's own file at run time; nothing from this folder ships. No `.eff`, texture, mesh or parameter file is in the repository. The captures below stay as the visual check (chip sheet 16). W9 manifest `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/c1162/di-sdi-extract/out/chips/manifest.json` sha256 `e6ac05dded1b811ab43987d3a57c5d2c15d68afd55ad528ee890b3de59362661`.

| render | file | source | sha256 |
| --- | --- | --- | --- |
| DI Up (432x432) | `captures/renders/di-up-4x-layer.png` | `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/c1162/di-sdi-extract/out/chips/di-up-4x-layer.png` | `b87f8d8122ed5c752f623a95f4832f34fea1f7242cf82851105524910c494ccb` |
| SDI Up (432x432) | `captures/renders/sdi-up-4x-layer.png` | `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/c1162/di-sdi-extract/out/chips/sdi-up-4x-layer.png` | `9d665a9051a1f54030ed53529f2eba1c18bf1a3196f5fdffe75b1b54fa061254` |

SDI burst, longer (proposal v10): the SDI chips (01, 16, 17 and the decision pictures 17 and 18) draw `captures/renders/sdi-up-4x-layer-stretch.png` (the burst stretched x2.0 along the SDI vector), not W9's own `sdi-up-4x-layer.png` (kept above, unused). Measured on the SDI Up captures (`c1162/captures-ally-sdi/sdi-up`, frames +10 to +15; the largest connected pixels with min(R,G,B) >= 240, bounding box): height / width 1.76, 1.47, 1.78, 1.84, 1.65, 1.57, median 1.71 (frames +16 and +17 are merged with the fighters and left out). W9's render, the solid burst (alpha >= 230, crown and legs, the thin line excluded): 58 x 86 px, height / width 1.48. The first try (x1.17, height / width 1.72, the captured median) was not visible at chip size, because the capture's burst is bolder and reaches much farther from the head in chip terms: measured against the captured crops at the chip scale of sheet 16 (reach 0.40 of the chip), the captured Up burst tip is 0.36 of the chip from the head and the captured In burst tip 0.31 with detached streaks going on past 0.40, while W9's burst tip is 0.19. Sheet 21 shows the Up and In chips at 60 and 30 px for x1.0 (v9), x1.17, x1.4, x1.7 and x2.0 beside the captured crops; x2.0 (tip 0.39) is the closest in reach and in weight, x1.7 (tip 0.32) falls short of the Up capture and of the In streaks, so x2.0 is drawn. Remaining difference: the render's burst keeps its width (about 13 % of the chip against about 29 % in the capture), so it is a taller, thinner flame than the captured arch. The thin line (`spark1`, W9's calibrated quad) is unchanged. Method, the root route as far as the file goes: W9's own renderer (`di-sdi-extract/tools/render_chips.py`, imported read-only) is run by `tools/render_sdi_stretch.py`, which changes only the burst emitter `impactring1`'s scale-Y. x1.17 reads its own scale-key table at particle life 0.5 instead of 0.3 (y key 1.356 -> 1.584, alpha still 1.0). That table ends at y = 1.93 (x1.42) at life 1.0, where the burst has faded to alpha 0.14, so x1.4, x1.7 and x2.0 are not in the file: they multiply the life-0.3 scale-Y by the factor, CALIBRATED against the captures, like W9's `spark1` quad. The scale-X, the origin, the colours, the gain, the framing and the other emitter are unchanged. The tool self-checks that factor 1 reproduces W9's file pixel for pixel. It needs numpy, so it runs under the ssbu-recomp venv Python (`tools/crop_menu_captures.py` calls it); the inputs are pinned by sha256 below. No `.eff`, texture, mesh or parameter file is in the repository.

| field | value |
| --- | --- |
| file drawn | `captures/renders/sdi-up-4x-layer-stretch.png` (432x432, RGBA light layer), factor 2.0 (scale); same pixels as `captures/renders/variants/sdi-up-4x-layer-x2.0.png` |
| sha256 | `11d9e2d29004c918fe7dc266216140d7087b3993a0dc1e19fe48f6967dbb4d61` |
| tool | `tools/render_sdi_stretch.py (imports W9 tools/render_chips.py)`; W9 `tools/render_chips.py` sha256 `b123bc3baa7ba3c47d963327f756d6fc086433df2f9003303544558e73e7b0bb` |
| inputs | stock 13.0.5 `ef_common.eff` sha256 `8ff5bad0681cc922dfc4988acfb0212def0514d717bb62b4df17b916d04e0023`; W9 `sdi-up-4x-layer.png` sha256 `9d665a9051a1f54030ed53529f2eba1c18bf1a3196f5fdffe75b1b54fa061254` |

Compared stretches (sheet 21, review material; the file drawn is the x2.0 one):

| factor | file | source | sha256 |
| --- | --- | --- | --- |
| x1.17 | `captures/renders/variants/sdi-up-4x-layer-x1.17.png` | emitter key table at life 0.5 (y key 1.356 -> 1.584) | `03afeaacc00f7071b161c37cd855ceb899955f108385c8b3b058c042148d8cdb` |
| x1.4 | `captures/renders/variants/sdi-up-4x-layer-x1.4.png` | life-0.3 S.y x 1.4 (calibrated) | `09a8a003dfbabbba3ac5a297c7c1f51f7d379c8ef3c400bd693e0874ebde01a2` |
| x1.7 | `captures/renders/variants/sdi-up-4x-layer-x1.7.png` | life-0.3 S.y x 1.7 (calibrated) | `2d63be098dc73c7cefe96c016946c39814129f8bdddb779231d610868dd9e5cb` |
| x2.0 | `captures/renders/variants/sdi-up-4x-layer-x2.0.png` | life-0.3 S.y x 2.0 (calibrated) | `11d9e2d29004c918fe7dc266216140d7087b3993a0dc1e19fe48f6967dbb4d61` |

DI streak (proposal v8; the visual check since v9): the game's own DI visual (system effect `sys_vector`), cut from one frame of a real capture. v8 drew the DI chips from this crop; since v9 the chip sheet (16) shows it once, as captured, beside the rendered DI chips. The game draws one effect and turns it about Z to the DI-corrected launch angle (SPEC-core-07 §4.6, from the static trace `c1162/di-sdi-trace/RESULT.md` §3.1). The clean room reads the effect from the player's game files at run time and never ships a picture of it. Capture: `c1162/captures-ally` run A (Ally Eden, fork M5l, Training on Final Destination, Steve forward smash on CPU Sora, DI Direction = Up, Hitbox Visualization off), Eden's GIF recorder, 1280x720 game frame, 256-colour palette; frame +13 after first contact (the streak shows from about +5 to +15). The strip: rotate the frame about the centre by the streak's measured axis so the streak points along +x, cut along and across it, feather the strip's edges in alpha (smoothstep; pixels unchanged). The frame's background and the CPU's keyblade show through.

| field | value |
| --- | --- |
| crop | `captures/di/di-streak.png` (137x44) |
| source | `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/c1162/captures-ally/di-sdi/runA/frames/runA_f+13.png` |
| sha256 | `4c7efe013c29de1a7ff6c49c5db769523c886355695366ee73fa61cb40515300` |
| centre, axis | (738, 332), 44.8° counter-clockwise from +x |
| along, across | (-45, 92) px along the streak, ±22 px across; feather (20, 10) |
| crop box in the source | corners (690.6, 348.1), (787.8, 251.6), (818.8, 282.8), (721.6, 379.3); bounding box (690, 251, 819, 380) |
| rotation centre in the crop | (45, 22) px; on a chip the strip's tail end sits on the chip centre and the streak runs toward the direction |

SDI spark (proposal v9, the visual check): the game's own SDI visual (system effect `sys_hitstop_spark`, a white spiky burst with a ring outline drawn on the CPU's head and pointing along the SDI vector), cut from one frame of a real capture. The chip sheet (16) shows it as captured, with its ring centre (the head) on the chip centre, beside the rendered SDI chips. The clean room reads the effect from the player's game files at run time and never ships a picture of it. Capture: `c1162/captures-ally-sdi` (W8; Ally Eden, fork M5l, Training on stock Final Destination, Steve forward smash on CPU Fox at 100 %, SDI Direction Up, Strength High, DI none, Hitbox Visualization off), the same recorder and frame as the DI capture; offset 0 is the first frame of the hit flash; the spark shows from about +6 to +17 and is absent in the no-SDI control (`sdi-none/`). The strip is cut as the DI streak; its alpha also carries a whiteness key (smoothstep of min(R,G,B) between the two key values), because the spark is pure white and the stage, the smoke and both fighters sit behind it; pixels are unchanged. A second crop from the SDI In run (the CPU stood right of the player, so In points left) is shown once on the chip sheet, as captured, beside the rendered In chip: a visual check of the rotation.

| field | SDI Up (16, check) | SDI In (16, check) |
| --- | --- | --- |
| crop | `captures/sdi/sdi-spark.png` (206x124) | `captures/sdi/sdi-spark-in-check.png` (166x132) |
| source | `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/c1162/captures-ally-sdi/sdi-up/frames/sdiup_f+12.png` | `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/c1162/captures-ally-sdi/sdi-in/frames/sdiin_f+12.png` |
| sha256 | `34084656526eec369da73ca3eaf49a355a5a4758561663450cbdd23f2e343c37` | `c43059336aa8823978b13602ead400d39347b9fd9179ac4ae719a3cf3c22d124` |
| centre, axis | (595, 262), 90.0° counter-clockwise from +x | (463, 352), 180.0° counter-clockwise from +x |
| along, across | (-64, 142) px along, ±62 px across; feather (10, 10) | (-80, 86) px along, ±66 px across; feather (10, 10) |
| whiteness key | (200, 245) on min(R,G,B) | (200, 245) on min(R,G,B) |
| crop box in the source | corners (533.0, 326.0), (533.0, 120.0), (657.0, 120.0), (657.0, 326.0); bounding box (533, 120, 657, 326) | corners (543.0, 418.0), (377.0, 418.0), (377.0, 286.0), (543.0, 286.0); bounding box (377, 286, 543, 418) |
| rotation centre in the crop | (64, 62) px (the ring centre, on the chip centre) | (80, 66) px (the ring centre, on the chip centre) |
