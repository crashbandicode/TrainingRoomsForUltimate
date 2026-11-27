# BOX 28 worker handoff

- Assigned clean worktree `/home/intpa/tr-fixes-20261113/toggle-wait`, branch
  `fix/toggle-load-wait`, base `cc71537cb7405690c468ecc9e42d776bca8176bd`.
  No source edits, commits, tests, or BOX 28 host operations yet.
- Read `BRIEF.md`, `DESIGN.md`, and lead's `CHECKLIST.md`. Invoked
  `ssbu-mod-playbook`, `bounded-behavior-fix`, `bounded-validation-evidence`;
  the pinned playbook must be completely read again before any code changes,
  and an independent worker checklist must be written.
- **H1 is false as written for saved-alpha accepted cycle requests.**
  `character_roster.rs:943–948` dispatches a `Selection::Cycle` via
  `queue_admitted` and `:777–780` returns Accepted from the saved-alpha
  intercept. In the fallback configuration (ordinary Player owner disabled,
  saved-alpha enabled with a Ready owner and valid roster), its
  `mii_saved_alpha.rs:213–225` stores `state.intent` and WORK without INSTALL.
  `notice_blocked_with` at `character_roster.rs:402–413` does not read that
  intent or WORK; with no unrelated blocked activity it is false while the
  accepted intent is pending, potentially across frames
  (`mii_saved_alpha.rs:345–381`). INSTALL is acquired only at :626–627.
  Crucially, `request_player_kind`/`request_cpu_kind` at
  `character_roster.rs:873–883,918–925` send Direct, and saved-alpha accepts
  only Cycle (`mii_saved_alpha.rs:213–217`): the counterexample is not a
  reachable clip-direct admission. Ask the lead whether the hypothesis should
  be narrowed to accepted Direct requests before proceeding; no workaround or
  forbidden-file edit was made. H2 and H3 are not completely established.
- H3 source breadcrumbs: `runtime/macros/mod.rs:170–184` forwards
  `route_clip_input(...)` to `port::post_clip_input`, `replay_capture.rs:670`
  shows `panel_visible` when EDITOR is Some; confirm `hook_queries`,
  `clip_foreground` and eventual `replay_capture::on_input` dispatch.
- BOX 27 final gate was interrupted: prior shell ID `f06c06` is gone, XPS
  claim `4443e67d` last seen 20:39Z at diag 4/6; local
  `../remote/p5b-gate-04b2a0d4` currently has no SUMMARY.md. Do not launch a
  duplicate or mark Done/Clean. Tell the lead its state/result is unknown.
