# BOX 28: the fighter toggle waits on the roster (a DEFECT fix on the build's critical path)

A CODE task on the TrainingRooms plugin. The design is settled and is mine: `DESIGN.md` beside this file. Read it
first, with `CHECKLIST.md`. This brief settles the interfaces and the tests. You wrote part C (BOX 27); this fixes it.

## Rules

- Shell: local commands only via `/usr/bin/pwsh -NoLogo -NoProfile -NonInteractive -Command ...`; no local Bash bodies;
  remote Windows hosts use `C:\Program Files\PowerShell\7\pwsh.exe` with the same flags; remote Linux helpers may use
  their native shell. Python: `/usr/bin/python3 -I -B`. NO cargo on this machine: build and test only through
  `/home/intpa/smash-training-lobby/primary-tools/remote-gate.ps1` (lane `toggle-wait`, owner `docs-tutorials`, called
  in-process with `&`, `-WaitMinutes 20`; never `-AllowSurface`, never `-Action Done` or `Clean`). The gate tool's own
  temp files and the harness's overflow files are fine; YOUR scratch goes in
  `/home/intpa/smash-training-lobby/artifacts/phase23/docs-tutorials/toggle/`.
- Playbook (`ssbu-mod-playbook` skill): the clone `/home/intpa/ssbu-mod-playbook` is at `c8d9959`; do NOT pull it (a
  process may be using it). Before changing code, read completely: `README.md`, every file in `docs/`,
  `tools/PROVENANCE.md`. Build your own checklist from `docs/CHECKLIST.md` (mine is `CHECKLIST.md` here; say where
  yours differs). Before your report, run `python3 -I /home/intpa/ssbu-mod-playbook/tools/check/compliance.py
  <worktree> --base cc71537cb7405690c468ecc9e42d776bca8176bd --trace <this folder>/DESIGN.md --nro <the gate's release NRO>`
  and save it as `COMPLIANCE-raw.md` here. If you cannot read the playbook, stop and say so.
- Worktree `/home/intpa/tr-fixes-20261113/toggle-wait`, branch `fix/toggle-load-wait`, at `rc-next`
  `cc71537cb7405690c468ecc9e42d776bca8176bd`, clean. Iterate on a throwaway `wip/toggle-wait`. Commit as
  `-c user.name=crashbandicode -c user.email=84094612+crashbandicode@users.noreply.github.com`, message from a UTF-8
  file with `git commit -F`, ending with "Generated with [Devin](https://devin.ai)" and
  "Co-Authored-By: Devin <158243242+devin-ai-integration[bot]@users.noreply.github.com>". Never push, never tag.
- **No pass count, frame count, timeout, tolerance, guard or skip of any kind may be added.** If you believe one is
  needed, stop and report it with its reason; it goes to the operator.
- No new `static` (the ticket word `CLIP_REPLY` is reused). No `allow`, no `#[ignore]`, no weakened or deleted
  assertion except the ones this change makes false, each listed in the report with why.
- CARRIED files (edit, then record the edit, then the carry tool; a base copy of the file as the manifest records it
  goes in `toggle/`; after `carry` the file must be byte-identical to what you wrote):
  - `src/model/macro_ui/clip_panel.rs`, `src/runtime/macros/replay_capture.rs`: pkg4:
    `python3 -I tools/carry_edits.py <dest> <base> --reason "…"`, then `python3 -I tools/carry_pkg4.py carry`,
    `ledger`, `verify`.
  - `src/runtime/roster/character_roster.rs`: pkg6: read `tools/carry_edits.py --help` and `tools/carry_pkg6.py`
    first and use the pkg6 manifest (`--manifest docs/provenance/pkg6-carry-manifest.json` if that is how the tool
    selects it), then `carry_pkg6.py carry`, `ledger`, `verify`. If the pkg6 tools refuse or need a stage you cannot
    reach, stop and report the exact lines.
  The gate's carry check runs before booking and must pass.
- Files you may edit: the seven of `DESIGN.md`'s table, `docs/RESTRICTIONS.md` only if a row's text must change, the
  pkg4/pkg6 manifests and ledgers through the tools, and test fixtures that build the changed types by hand (list
  them). Anything else: stop and report.

## Three facts to verify by reading BEFORE you code (each decides the design; if one is false, stop and report)

- **H1.** From the game frame that admits a request on, `character_roster::notice_blocked()` (`:402`) is true until
  the roster is done with that request: on the ordinary path (`state.pending`, then `state.active`), and for the Mii
  owners' intercepts (`mii_ordinary_player`, `mii_ordinary_cpu`, `mii_saved_alpha`).
- **H2.** When a load completes, the role's `observed` becomes a new `(entry, kind)`; there is no frame on which the
  roster is not blocked while `observed` is still exactly the pair it was at admission, unless the request was dropped.
- **H3.** With the owner `REPLAY_LOADING` and the editor still in `EDITOR`, B still reaches
  `replay_capture::on_input` (I read `on_p1_input` → `route_clip_input` → `clip_foreground || replay_loading` →
  Forward; `clip_foreground` comes from `panel_visible()`; confirm).

## Interfaces (exact)

`src/runtime/roster/roster_requests.rs` (the word packs the ticket and a 3-bit state; choose the layout, keep tags
0..=2 for the older paths, and keep the combo's reply ring untouched):

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClipOutcome { Asked, Admitted, Refused, Applied, Dropped, Withdrawn, Committed }
/// Worker. A new ticket; any older ticket's answers are dead from here on.
pub(crate) fn request_clip_kind(player: bool, kind: u32);
/// Worker. Ask to take the current ticket's request back.
pub(crate) fn withdraw_clip();
/// Worker. The current ticket's state; `None` when there is no ticket in flight. Reading does not consume it.
pub(crate) fn clip_outcome() -> Option<ClipOutcome>;
/// Worker. Forget the ticket after a final state (Refused, Applied, Dropped, Withdrawn) was shown.
pub(crate) fn clear_clip();
```

`take_clip_reply` goes. `service()` (game frame): admission as today for the current ticket only; a withdraw seen
before its request is sent means the request is never sent; a withdraw after admission calls
`character_roster::withdraw_direct`; and once per frame, for an `Admitted` or `Committed` ticket, the outcome from
`character_roster::clip_progress` by `DESIGN.md` §2. The pure decision is one function so the fixture can test it:

```rust
/// `busy`: the roster is working. `before`: the role's (entry, kind) at admission. `now`: the same, now.
pub(crate) fn settle(asked_kind: u32, busy: bool, before: Option<(usize, u32)>, now: Option<(usize, u32)>) -> Option<ClipOutcome>;
```

`src/runtime/roster/character_roster.rs` (two functions, no other change):

```rust
/// What the clip ticket reads each game frame: the roster is working on something (`notice_blocked`), and the role's
/// observed (entry, kind).
pub(crate) fn clip_progress(player: bool) -> (bool, Option<(usize, u32)>);
/// True when the role had a user's Direct request for `kind` waiting and no load begun, and it was removed.
pub(crate) fn withdraw_direct(player: bool, kind: u32) -> bool;
```

`src/model/macro_ui/clip_panel.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FighterOutcome { Loading, Committed, Applied, Refused, Withdrawn }
impl ClipEditor {
    /// A switch is wanted, asked or loading.
    pub fn fighter_load(&self) -> bool;
    /// Every worker pass while a switch is asked: what the roster says. Applied ends the load only when the seen kind
    /// is the asked one; Refused ends it with the refusal; Withdrawn ends it silently. True when the strip changed.
    pub fn note_fighter_outcome(&mut self, outcome: FighterOutcome) -> bool;
    /// B. A wanted switch is dropped at once (returns false: nothing to tell the roster). An asked one is marked as
    /// being withdrawn and stays "loading" (returns true: the worker posts the withdraw).
    pub fn cancel_fighter(&mut self) -> bool;
}
pub const FIGHTER_REFUSED: &str = "The roster refused the fighter switch"; // restriction: clip-fighter-refused
```

`tick_fighter_load` and `fighter_load_passes` are deleted. `note_fighters(kinds, settled)` no longer ends a load by
itself. The loading line is "Loading {name}… B cancels" until a withdraw was asked or the roster committed, then
"Loading {name}…". `fighter_word` stays `loading` throughout.

`src/runtime/macros/replay_capture.rs`: `pub fn fighter_load() -> bool`; `service_wanted_fighter` maps
`roster_requests::clip_outcome()` to `note_fighter_outcome`, shows `FIGHTER_REFUSED` through `notify` on a refusal,
calls `clear_clip()` after a final state, and has no timeout branch; the B block in `on_input` calls
`cancel_fighter()` and, when it returns true, `roster_requests::withdraw_clip()`.

`src/runtime/macros/mod.rs` `sync_owner`:
`let held = (replay_capture::panel_visible() && !replay_capture::fighter_load()) || replay_capture::browser_visible();`
`let loading = replay_capture::loading() || replay_capture::fighter_load();`

## Tests (exact names; red first)

Model cases in `clip_panel.rs` (plain fns, registered in `case_table()`), fixture cases through
`tests/support/clip_admission.rs` (it compiles the production `service_wanted_fighter` and `roster_requests.rs`
against a fake roster; extend the fake with `notice_blocked`, `observed`, `clip_progress`, `withdraw_direct`):

1. `an_outcome_after_600_passes_still_ends_the_load`: 2,000 worker passes with the roster busy keep "loading" and the
   "Loading …" line; then the roster applies and the strip names the new fighter. Model and fixture.
2. `b_before_commit_withdraws_the_request`: B while the request waits in the roster → the fake roster's pending is
   gone, the fighter never changes, the editor is back on "current" with no line. Also B before the hold let go.
3. `b_after_commit_follows_the_roster`: B after the fake roster began the load → the editor stays "loading", the line
   loses "B cancels", and it ends on the new fighter.
4. `a_refused_switch_shows_the_refusal`: refusal at admission.
5. `a_dropped_switch_shows_the_refusal`: admitted, then the fake roster gives it up with the fighter unchanged.
6. `the_strip_stays_loading_until_the_roster_answers`: no other line of the editor's own appears while a load is
   under way; the string "did not load" is gone from `src/`.
7. `the_cpu_toggle_and_the_way_back_wait_on_the_roster`: ZL+R to the take's fighter and back, and Player 1 back, each
   through cases 1, 2 and 4's shapes.
8. `the_editor_does_not_hold_the_rosters_gate_during_a_fighter_load`: in `input_delay.rs`, the pinned `sync_owner`
   text (the old pin at `:348` is replaced by the new line), and `Owners` cases: with a fighter load the published
   set is `REPLAY_LOADING` only, which does not hold gameplay and still makes Player 1's poll neutral.

## Steps

A. Read (playbook, design, H1 to H3). Write your checklist.
B. RED commit on `fix/toggle-load-wait`: the tests, with the new names as signatures that keep today's behaviour
   (the counter still there), so it builds and the new cases fail by assertion. Prove it:
   `-Steps test -TestFilter 'lib=clip_panel','lib=input_delay','all=roster_request_handoff'`
   (check the filter syntax in `remote-gate.ps1`'s help first) `-Out …/docs-tutorials/remote/toggle-red-<sha8>`.
C. GREEN on `wip/toggle-wait`, gated (`-Steps test,clippy,ndf,release,diag,size`); then ONE commit (or a merge
   commit) on `fix/toggle-load-wait` with that tree, and the six-step gate on that exact sha
   (`-Out …/remote/toggle-gate-<sha8>`). If the size step is over the ceiling, add NO budget entry: report the count
   at `cc71537c` (209,091) and at green.
D. Compliance raw run. Report.

## Report (≤ 30 lines, UTC from the clock)

One plain sentence first. Then: H1 to H3 with the lines that show each; red sha, failing cases, Out path; green sha
and the gate's SUMMARY rows verbatim; the test count from the log's result lines, summed (the summary row can miss a
line); lines at base and green; release and diag raw `.bss` and `.data` and both NRO sizes against
`/home/intpa/smash-training-lobby/artifacts/phase23/next-build/` gate of `cc71537c` if its ELFs are there (`readelf -S`);
every file changed; every existing assertion changed, with why; the carry tools' exit codes for pkg4 and pkg6; what
you could not do as written; what is not established.
