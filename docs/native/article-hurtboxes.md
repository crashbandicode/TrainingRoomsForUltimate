# Indexed article seam — 2026-11-15

| Contract | Value |
| --- | --- |
| Game | 13.0.5; main build ID `21450c647b8c5940ed5b68218ea128fd3c5d597d` |
| Seam | `article_get_from_no`, Called only, main+`0x2092e10`; no patch |
| ABI | `extern "C" fn(usize, i32, i32) -> usize`: live accessor, article kind, ordinal; nullable `Article*` return |
| Ordinal | Dense zero-based matching-kind entry in the current ring; enumerate to first null. Active-count equivalence is unproved. |
| Guard | Exact 64 bytes, SHA256 `f586a4f0204ceacc5e6ac31d8ce7f08dffa773fda367e5a1fd71a698ceec0fd4`; canonical bytes in `data/tables/sites_hitbox.SITES.tsv` |
| Owned ELF | SHA256 `0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2`; guard file offset `0x2093e10` |

The existing boot version/build-ID verdict remains authoritative. The wrapper uses `state_natives::at` and therefore calls only a granted offset. The Hitbox row enters that feature's existing capability closure. No new allocation or lock is introduced.

The dated native evidence record `artifacts/rc15-article-hurt/NATIVE-ABI.md` retains symbol, dispatch, pointer-return and ordinal derivation. Its actual19-NRO card copy dated 2026-11-15 produced no detected references at the thunk, concrete getter or virtual slot. The scan recognizes constants and MOVZ/MOVK construction; it does not establish absence of other address construction or later plugins. Install-time guards remain required.

`core06_site_guards` pins the row and its capability dependency; `core06_rows_match_the_owned_elf` compares all 64 guard bytes and the exported symbol against the owned ELF. These extensions await the coordinated host gates. This seam record claims no runtime rendering or lifecycle coverage.
