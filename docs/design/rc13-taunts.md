# Player 1 taunts in practice

The rc.13 setting shipped on **Display & Controls → Controls → [Taunts](../menu/display-and-controls.md#taunts)**.

## Design record, 2026-11-14

| Field | Record |
| --- | --- |
| Scope | Player 1’s D-pad taunts in Training and waiting-room practice. |
| Decision | 03:44Z, 2026-11-14. Kirby keeps Up while wearing a copy hat. |
| Owner | gif-nav-author-2, branch `gna2-taunts`. |
| Base | Worker A’s `a21fd166`, `/home/intpa/tr-fixes-20261113/wa-orbs`. |
| Integration | The design branch targeted worker A’s tree. |
| Sources | Plugin at `a21fd166`, game constants by name in `model::cpu::ids`, and `skyline_smash` constant names. |

## Existing input path

| Component | Role |
| --- | --- |
| Grounded status script | Requests `FIGHTER_STATUS_KIND_APPEAL` when the command flag and transition term both allow it. |
| D-pad flags | `FIGHTER_PAD_CMD_CAT2_FLAG_APPEAL_HI`, `_S_L`, `_S_R`, `_LW`. |
| Transition terms | `FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_APPEAL_U`, `_S`, `_LW`. |
| `cpu_transition_term_hook` | Reuses `WorkModule::is_enable_transition_term` at `main+0x208dd60` (`sites_pkg2`, feature `CpuLedge`). |
| Earlier hook | At the design base, it already handled the CPU’s ledge climb. |
| Chords and shortcuts | Consume Player 1’s controller poll independently, including B + D-pad Up. |
| `role_scene()` | Recognizes native Training and live waiting-room practice (`model::scene`, D4). |
| Player 1 | Entry 0, including Nana. The frame wrapper visits each body. |
| Kirby | Reads `FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY`, as do `roster::kirby_hats::equipped` and the body bank. |

## Shipped behavior

| Setting and scene | Player 1’s native taunts |
| --- | --- |
| On, or outside practice | All permitted. This setting never overrides a native refusal. |
| Off, in practice, Kirby with a hat | Up can drop the hat. Side and Down are denied. |
| Off, in practice, any other body | Up, Side and Down are denied. |

- The `taunts` row followed L+R+A Reset. Appending it to `model::factory::DEFS` preserved stored indices. On comes first and supplies the missing-row default, as with Lobby Items, HUD and L+R+A Reset.
- The entry-0 `fighter_frame` branch reads the published `CpuSettings::taunts` in practice, body kind and copy flag. `denied_mask(on, practice, kirby_with_hat)` produces the three-bit mask.
- An atomic table holds an accessor and mask for each of two bodies (Popo and Nana). Outside practice or with Taunts On, the mask is zero. `scene_departed` clears both slots.
- After a positive native answer, `denies(terms, mask, term)` checks this accessor and the three appeal terms. CPU taunts, online matches, native D-pad flags and controller input retain their normal paths.
- `TermContAppealU/S/Lw` and `KirbyFlagCopy` resolve by name. Unresolved terms pass through. The taunt check requires no lock, native call or allocation.
- When the mask changes, diagnostics emit `cpu_event taunts entry=0 slot=<n> kind=<k> mask=<m> practice=<b>`. The `[FrameStrip] p1 status=` line identifies an appeal.
- The size comparison used `tools/build/nro_size.py` before and after adding the row, two-slot table, hook comparisons and entry-0 branch. The original post reported the delta.

## Requirement trace

| Requirement clause | Design | Test |
| --- | --- | --- |
| Taunts can be turned off | the `taunts` toggle row, appended, default On | `taunts_row_defaults_on_and_reads_off` |
| Off disables every D-pad taunt | `denied_mask` denies Up, Side and Down while Off in practice | `off_denies_every_appeal_in_practice` |
| Kirby with a hat keeps the up taunt | Up stays allowed for Kirby with `FLAG_COPY` set | `kirby_with_a_hat_keeps_up_only` |
| Kirby without a hat has no exception | Kirby without the flag is denied like anyone | `kirby_without_a_hat_is_denied` |
| It applies in Training and in the waiting room only | only `role_scene()` Training or Lobby; online and every other scene answer the original | `outside_practice_nothing_is_denied` |
| On preserves native behavior | On allows all three everywhere | `on_allows_every_appeal` |
| (only the game's taunt is touched) | only the three appeal terms are ever denied | `only_appeal_terms_are_denied` |
