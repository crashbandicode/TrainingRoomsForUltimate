# UI18: text style and container lookup

UI18 used compiled source `5d92df4b953db68113d2b65674cd96d3afce09df`, diagnostic
NRO `e0374823c3ed4f01bc22a73e4574a00d1e29e699dedd71e3cd6036cf9dc19f4f`.
The retained original images and serialized layouts establish two separate UI defects.
The fixes below require integration and runtime verification; this is not a product PASS.

## Waiting text

The builder cloned `set_txt_status`, retaining scale `(0.5, 0.52)`, font
`FOT-RodinNTLGPro-B64`, and the green `set_txt_status` material. Nominal font size
`22 × 34` and neutral text vertex colors did not remove those inherited properties.
The corresponding Training template uses scale `(1, 1)`, `nintendo64`, and a neutral
material. Both archives contain the qualified `nintendo64` font.

The original CPU-tab glyph crop measures 27 × 11 pixels in Training and 15 × 6 in
waiting under the recorded color masks. The waiting crop is green. These are original
runtime pixels, not a preview renderer's missing-glyph result. UI17's earlier glyph
failure remains retracted; it is not evidence for this defect.

The builder now explicitly normalizes owned text scale and resolves the font by name
in each archive. Waiting text binds to a new `tr_text_neutral` material with the same
plain black/white mapping as the qualified Training material. Stock status panes,
materials, animation references, and textures retain their original bytes. Training's
target is expected to remain byte-identical. The reordered-font fixture checks that
the implementation does not assume Training's numeric font index.

## Container lookup

The existing depth-first lookup spends its 4,096-pane budget on expanded stock parts
before reaching Training's notice and input-log containers. Independent expansion of
the actual qualified archives, replacing a part root with its host, gives:

| Layout | Expanded panes | Menu ordinal | Notice ordinal | Input log ordinal |
| --- | ---: | ---: | ---: | ---: |
| Training | 4,493 | 3,907 | 4,218 | 4,223 |
| Waiting | 1,565 | 979 | 1,290 | 1,295 |

Waiting's 1,565 count exactly matches the native diagnostic. Training's native count
is capped at 4,096; the 4,493 total is the artifact expansion, not a native measurement.
All referenced parts resolve. The menu resolves, while the input log is missing and
the full console contains no notice draw. The startup gate requires a reachable native
notice container, so those startup notices were not posted. All 297 original startup
recording frames were audited; the retained contact sheet shows no notice panel.

Known plugin containers now resolve among the qualified root's immediate children;
waiting first resolves the direct `TrModPort` child. Only owned subtrees use the
existing descendant lookup. The child walk retains the existing intrusive-list ABI,
4,096-node bound, and null handling. It allocates nothing and adds no retained pointer.
The main-source composition preserves the startup-once gate and short/long notice
writers. Menu, notice, and input display use the same container boundary.

The original traversal tests execute production `pane.rs` with host-owned intrusive
lists. They retain the failing depth-first control, actual expanded private fixtures,
normal waiting, a nested wrong host, missing containers, and a cyclic sibling list.
Their field shim does not independently validate the native ABI or text setter.

## Retained execution evidence and limits

Private evidence is under `tmp-ui-screens/ui18-yoga` and `ui19-boundaries` in the campaign
directory. UI18's 74-file archive was SHA-verified before restoration. Yoga's original
1,362 profile entries and 14 harness entries were separately verified after restore;
the claim was released at 06:04Z. No card, DBI, or USB was used.

The complete UI18 console has 7,991 lines, zero Unmapped lines, and 138 hook-stat samples
with zero total/delta allocations. Both archives loaded twice from vanilla bases and
retired after at least 5,001 ms. Peak cgroup usage was 7,026,610,176 of 8,589,934,592 bytes.
The accepted two-buffer pool retains 3,141,384 requested capacity bytes: retirement is
logical reuse, not RSS reclamation. Native heap before/after was not observed. Host
memory correlation uses explicitly approximate console-relative time windows, not an
assumed equivalence between plugin and host clocks.

UI18 also measured TraceWriter 8,624/20,480 bytes with `low=true`, requiring 28 KiB under
the unchanged stack policy. Lead owns that correction. Native Mii editor entry worked,
but the installed Q15 constructor observer produced zero observations; that route remains
unresolved. Developer positive-control rows and final integrated runtime gates remain owed.

For the next planned native-editor regression, the lead requires editing an existing
saved Mii without saving. UI18's snapshot hashes match the independently reread retained
`mii_2622.bin`, `mii_3137.bin`, and `mii_4155.bin`: each is 410,432 bytes and has a
409,776-byte native image payload. The next run must verify its own snapshot rather than
assume those files remain present. Add New did not establish the saved-image trigger.

## UI19 validation

Surface's admitted 06:37Z cached host job passed all four production traversal tests and
all 12 builder/archive/pane tests, including the private fixtures (zero failed or ignored).
Both generators exited zero. All 51 frozen input files were unchanged after execution.
The earlier compile failure from an omitted test-bundle recipe is retained as attempt 1;
it executed no tests. The corrected bundle added both exact-base recipes.

Independent TRA1 reconstruction and serialized checks passed:

| Fixture | Stock panes | Stock materials | Stock textures | Owned text leaves checked |
| --- | ---: | ---: | ---: | ---: |
| Training | 178 | 85 | 43 | 261 |
| Waiting | 63 | 37 | 91 | 261 |

All listed stock objects retain their bytes. All prior nonlayout members are identical,
including the combined texture archive. Training's complete target and recipe remain
identical. Waiting changes only owned text scale/font/material bindings, adds one owned
neutral material, and shifts owned item-picture material indices; their material contents
are unchanged. Both targets retain their previous lengths, so the two-buffer requested
capacity remains 3,141,384 bytes. The new waiting recipe SHA-256 is
`014eb1685143d4e66f58681c1d865019ae581f2eeedcaa3cdd30c9079f69ffcb`.

The host build reports the pinned dependency `binrw 0.13.4` future-incompatibility warning
E0365 (private crate re-export); its raw report is retained. This is not a zero-warning
full gate. No dependency change or warning suppression is included. Integration clippy,
no-default-features build, native compile/stack/compliance, and actual runtime pixels
remain pending the lead's coherent candidate. No new Eden admission is implied.
