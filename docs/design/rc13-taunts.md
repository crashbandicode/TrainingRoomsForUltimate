# rc.13 Taunts: Player 1's taunts can be turned off in practice (design, 2026-11-14)

- **Scope:** the operator's rc.13 order (the primary's 03:44Z 11-14 post): a setting that disables taunts during
  Training or the lobby, with Kirby's D-pad Up kept while he has a copy hat.
- **Owner and branch:** gif-nav-author-2, branch `gna2-taunts` from worker A's `a21fd166`
  (`/home/intpa/tr-fixes-20261113/wa-orbs`). Nothing goes to `main`.
- **Sources:** the plugin's source at `a21fd166`; the game's own constant table (by name, `model::cpu::ids`); the
  `skyline_smash` constant list for the names. No new game site.
- This is a design working file. Its requirement table quotes the operator's request clause by clause, as the
  completion checklist asks of a design report. The README and the user docs carry no quotes.

## 1. What exists today

- **The game starts a taunt through a transition term.** A grounded status script opens `FIGHTER_STATUS_KIND_APPEAL`
  only when its D-pad command flag (`FIGHTER_PAD_CMD_CAT2_FLAG_APPEAL_HI`, `_S_L`/`_S_R`, `_LW`) is set and
  `WorkModule::is_enable_transition_term` answers yes for `FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_APPEAL_U`, `_S` or
  `_LW`.
- **TrainingRooms already hooks that question.** `cpu_transition_term_hook` (`main+0x208dd60`, the `lua_bind`
  `WorkModule::is_enable_transition_term`, seam row in `sites_pkg2`, Feature `CpuLedge`) answers every body's query. Today
  it changes only the CPU's ledge climb and returns the original for everything else.
- **TrainingRooms' own D-pad buttons never reach that question.** The chords, the menu shortcuts and B + D-pad Up are
  read from Player 1's controller poll (`model::chords`, the menu input hook). A denied transition term changes nothing
  there.
- **Where it is practice** is `role_scene()` in `runtime::features::cpu`: native Training, or the waiting-room practice
  in the live waiting room (`model::scene`, decision D4); an online match is neither.
- **Player 1** is entry 0. The frame wrapper (`fighter_frame`) sees each entry-0 body every frame, Nana included.
- **Kirby's copy hat:** `FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY` set (as `roster::kirby_hats::equipped` and the body
  bank read it).

## 2. Design

- **The row:** `taunts`, a toggle `[On, Off]`, appended to `model::factory::DEFS` (stored indices unchanged), default
  On (today's behaviour, and every other "native behaviour" toggle defaults On: Lobby Items, HUD, L+R+A Reset). Shown
  as *Taunts* on Display & Controls → Controls, after L+R+A Reset (the D-pad buttons' page). A missing row reads On.
- **The rule (pure, `model::taunts`):** `denied_mask(on, practice, kirby_with_hat)` is 0 when the row is On or the body
  is not in practice, Side and Down for Kirby with a copy hat, else all three. `denies(terms, mask, term)` answers the
  hook: `term` is a resolved appeal term whose bit is in the mask.
- **Published once per frame, read lock-free by the hook:** `fighter_frame`'s entry-0 branch reads the row
  (`CpuSettings::taunts`, the CPU settings the menu already publishes; only in practice), the scene (`role_scene()`), the body's kind and its copy flag, and stores the body's
  accessor with its denied mask in a two-slot table (Popo/Nana). Outside practice, or with the row On, it stores mask
  0. `scene_departed` clears the table.
- **The hook:** `cpu_transition_term_hook` keeps its ledge branch. When the original answer is yes and `term` is one of
  the three resolved appeal terms, it asks the table for this accessor and answers no when the mask denies that appeal.
  No lock, no native call and no allocation on that path.
- **Constants:** `TermContAppealU/S/Lw` and `KirbyFlagCopy` join `model::cpu::ids` (resolved by name like every other
  term). An unresolved term is never denied.
- **Logging:** the diag build logs `cpu_event taunts entry=0 slot=<n> kind=<k> mask=<m> practice=<b>` when a body's mask
  changes, for Eden; Player 1's status lines (`[FrameStrip] p1 status=`) show whether `FIGHTER_STATUS_KIND_APPEAL` started.
- **Not changed:** the CPU's taunts (the order is Player 1's), online play, the D-pad's command flags, input itself.

## 3. Requirement trace

| Requirement clause | Design | Test |
| --- | --- | --- |
| Taunts can be turned off | the `taunts` toggle row, appended, default On | `taunts_row_defaults_on_and_reads_off` |
| Off disables every D-pad taunt | `denied_mask` denies Up, Side and Down while Off in practice | `off_denies_every_appeal_in_practice` |
| Kirby with a hat keeps the up taunt | Up stays allowed for Kirby with `FLAG_COPY` set | `kirby_with_a_hat_keeps_up_only` |
| Kirby without a hat has no exception | Kirby without the flag is denied like anyone | `kirby_without_a_hat_is_denied` |
| It applies in Training and in the waiting room only | only `role_scene()` Training or Lobby; online and every other scene answer the original | `outside_practice_nothing_is_denied` |
| (On is today's behaviour) | On allows all three everywhere | `on_allows_every_appeal` |
| (only the game's taunt is touched) | only the three appeal terms are ever denied | `only_appeal_terms_are_denied` |

## 4. Size

A row, a 2-slot table of atomics, one compare chain in an existing hook and one branch in the entry-0 frame. Measured
with `tools/build/nro_size.py` before and after (the post reports the delta).
