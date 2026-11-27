# Indexed article seam — 2026-11-15

| Contract | Value |
| --- | --- |
| Game | 13.0.5, main build ID `21450c647b8c5940ed5b68218ea128fd3c5d597d` |
| Called site | `article_get_from_no` at main+`0x2092e10`. The plugin leaves its instructions intact. |
| ABI | `extern "C" fn(usize, i32, i32) -> usize` |
| Arguments | Live module accessor, article kind, zero-based ordinal |
| Result | Nullable `Article*` |
| Enumeration | Visit consecutive matching-kind entries in the current ring until the first null. |
| Active count | Uses a different predicate from indexed lookup. |
| Guard | 64 bytes from [the Hitbox site table](../../data/tables/sites_hitbox.SITES.tsv) |
| Guard SHA256 | `f586a4f0204ceacc5e6ac31d8ce7f08dffa773fda367e5a1fd71a698ceec0fd4` |
| Owned ELF SHA256 | `0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2` |
| Guard file offset | `0x2093e10` |

| Runtime check | Effect |
| --- | --- |
| Boot | The game version and build ID must pass. |
| Native call | `state_natives::at` checks the offset grant before calling. An unavailable getter returns zero. |
| Callers | Hitbox and Break the Targets projectile tracking both require this site. |
| Wrapper cost | The wrapper adds no allocation or lock. |

| Evidence from 2026-11-15 | Scope |
| --- | --- |
| `artifacts/rc15-article-hurt/NATIVE-ABI.md` | Symbol, dispatch, pointer return and ordinal derivation |
| Card scan | Across 19 NROs, no detected references to the thunk, concrete getter or virtual slot. |
| Scan coverage | Constants and MOVZ/MOVK construction only. |
| Scan limits | Other constructions and later plugins remain outside that scan. Install-time guards remain required. |
| `core06_site_guards` | Pins the site row and Hitbox dependency. |
| `core06_rows_match_the_owned_elf` | With the owned ELF present, compares all 64 guard bytes and checks the exported symbol. |
| XPS gate | Both tests passed for `6def8553` on 2026-11-15. |
| Runtime coverage | Rendering and object lifecycle were outside that gate. |
