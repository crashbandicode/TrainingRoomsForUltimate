# UI20 failures and UI21 repair candidate

UI20 ran frozen r15 (`ab13a0f`, NRO `0bbcfde5`) on Yoga. It confirmed three UI failures. The host was stopped, its full 1,362-entry profile and 14-entry harness snapshot restored and independently verified, and the claim released at 07:45 UTC on November 3. No fixed-runtime acceptance is claimed below.

## Displayed developer state

The menu stores both Developer controls as `[On, Off]`. Both runtime publishers instead read index 1. In UI20, displayed Off queued six completed configuration reads; displayed On followed by a 1,200 ms held L+R+A queued none. The default snapshot also selects Off.

Both publishers now use the same typed `developer_ini::Control` accessor, reading index 0 and applying the existing boot page gate. The queue, cancellation epochs, callback ownership, file reader and debug hook remain unchanged. Original regressions exercise factory Off, explicit Off/On, an empty selection, hidden-page preservation, queued/in-flight cancellation, and hiding an enabled page. Existing held-chord, overflow, reader and native-guard tests remain part of the targeted gate.

## Notice contrast

The captured short and long notices used black body text on the owned dark panel. `notice_draw` logged `[0,0,0,255]`; the producer-to-linear-color-to-TextBox path preserved that value. SPEC-input-05 explicitly requires default black and the existing recording phase colors, so changing the API default would violate its contract.

The lead approved an opaque light notice background and dark header at 07:48 UTC. Only those two pane colors change. Body colors, menu palette, short/long geometry and timing remain unchanged. The first proposed background `[236,238,242]` failed the new 4.5 contrast criterion for magenta (4.1804); that uncompiled candidate is retained. Opaque white gives contrast 21.0 for default black, 14.002 for the header, 11.176 for blue, 4.856 for magenta and 6.010 for red. Native short/long/custom-color pixel proof remains required.

## Waiting text material

UI20's waiting menu opened, changed page, closed and reopened, but all owned text was absent. Stock text and the owned rectangles remained visible. The exact waiting archive was `02dc5796`; the 20-second original GIF is `abe65ca4`.

The UI19 builder created `tr_text_neutral` from `Material::default()`. Comparing serialized material records shows that this erased the stock v8 word at offset `0x20`: stock text materials, including the working Training template and waiting status template, contain `0x08040200`; the new material contained zero. The library deliberately preserves this undecoded word. The repair clones the complete waiting stock text material and changes only its owned name and black/white tint. It preserves all other render state and all original materials. No meaning is invented for the opaque word. A private-stock regression compares the complete cloned material, including that word; fresh native rendering must confirm the repair.

## Evidence and limits

Private UI20 evidence is in `../tmp-ui-screens/ui20-yoga-pending`. All 47 remote files (90,178,211 bytes) were locally hash-verified against manifest `e9656bce`. Final console `129dcbd4` contains 7,846 lines, 165 zero-allocation/zero-delta hook samples and zero total Unmapped errors. Six existing-Mii constructor observations qualify the exercised context only, not a coherent native save provider. Each Training/waiting archive injection allocated zero times and retired after 5,001 ms; the two buffers remain pooled.

The source candidate is frozen against reviewed main `96da3a4` in `../tmp-ui-screens/ui21-root`. Text patch `67e5a0be` contains seven files; its pane adaptation preserves main's existing long-notice layout. An initial test-only import error (`model::ui::panes` instead of `model::panes`) failed compilation before tests; the original source, log and failed target were retained. The corrected gate passed 11 developer-related tests and one notice contrast test in fresh executable processes. Its compile emitted no warnings.

The separate builder gate passed four production traversal tests and 12 builder/model/private-stock tests, including the complete material-clone regression: 28 executed tests across the two successful gates, zero failed or ignored. Builder `14bb58de` generated both recipes. Its existing `binrw 0.13.4` future-incompatibility warning remains in the raw log; this is not a zero-warning full integration gate.

Independent byte verification found only notice base/header vertex/text colors changed in both layouts and material 37's preserved word changed in waiting. Every other pane byte and every non-layout archive member matched r15. All stock panes (178 Training/63 waiting), materials (85/37) and textures (43/91) were preserved. Both recipes rebuild their targets exactly. Target lengths remain 742,292/1,570,692 bytes, so the two-buffer capacity is unchanged. The known untouched-SARC serialization difference remains disclosed in the builder reports.

| Output | Recipe SHA-256 | Private target SHA-256 |
| --- | --- | --- |
| Training | `1fc5444a404c8158fb707ed9181e8f3083b55a767a8daedee1f15846bfa7a8c3` | `85cf811f7ce3c9855a69b945338550c0b7310f106b52217de45ba527e83fd106` |
| Waiting | `32901f6880f0682fe00201b0f27a955c04568d2a841bd1649e7e3e160686b5b2` | `1f06f9807084eeeec8c019a9e30bdfcee07faca50f0d18c57112547e58336e6b` |

Both host claims were released, the last at 08:07 UTC. Full integration gates, a composed native build, modified-path stack qualification and runtime acceptance remain required. No failing GIF is published as a working demonstration.
