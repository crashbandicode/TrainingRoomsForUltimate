"""spots-3: fill the `agent` rows of a compliance.py report with this lane's dispositions and evidence (modelled on integration-3's).
Usage: python3 -I dispose_compliance.py <report.md in> <report.md out> <facts.json>
facts.json: {"head": "<sha8>", "gate": "<gate folder>", "release_sha": "...", "diag_sha": "...", "boots": "<labels, claims, releases>",
             "rows": "<row results>", "console": "<console scan line>"}"""
import json
import sys
from pathlib import Path

f = json.loads(Path(sys.argv[3]).read_text())
A = 'artifacts/rc14-spots-3'
D = {
    'opt-level "z"': ('N/A: the release profile is unchanged', '`git diff 79ba5ba9 HEAD -- Cargo.toml` is empty'),
    'Gates run:': ('pass', '`%s/gates/%s/SUMMARY.md`: test, clippy `-D warnings` 0, `--no-default-features` 0, Switch release and diag 0 warnings, size within the ceiling' % (A, f['gate'])),
    'Fixes:': ('pass', 'root cause: spots ranked by damage; now by knockback (the decision); the stale-tip mislabel fixed at its cause (attribution by the game\'s recorded slot). The new windows (damage 0.5..1.6, knockback 0.90..1.05) and guards were approved by the primary (02:40Z, 03:20Z 11-15); `DESIGN.md` Rulings'),
    'Card scan': ('pass', '`%s/gates/card-scan.txt` (`scan-offsets.py` over the card copy of 2026-11-14 `card-rc13/3-tr-reroll/after`, sites 0x200b710, 0x3ffa10, 0x4f86b18): no other plugin refers to them; 0x402f9c is relied on, not hooked' % A),
    'Hook sharing': ('N/A: no hook added (one Called thunk, two Relied windows)', 'card scan above'),
    'Hook after the prologue': ('N/A: no hook added', '-'),
    'Hooks never block': ('pass', '`tests/hook_locks.rs` and `tests/lock_order.rs` pass in the gate; the new reads take no lock (the strip\'s existing TRACKER lock is unchanged)'),
    'Shipped NRO writes only': ('pass', 'the new lines (`[Spots]`, `[SpotsKB]`, `[SpotsWind]`) are `#[cfg(feature = "diag")]`; machine row "no diag strings" passes on release `%s`' % f['release_sha'][:12]),
    'Online: read-only': ('pass', 'reads only (natives `damage_reaction`, `get_param_float`, raw record reads); the strip and the visualizer run only in Training and waiting-room practice (`cpu::fighter_frame`, `hitbox::visualizes`); nothing written to the game'),
    'Online: replay A/B': ('N/A: no online path changed', '-'),
    'Online: two consoles': ('N/A: no online path changed', '-'),
    'Eden: claim': ('pass', f['boots']),
    'Eden: at most two': ('pass', 'one instance per boot; the Yoga held at most two (cpu-target-flight beside boot 2); the up script waited out the other Eden\'s load phase; ' + f['boots']),
    'Eden: profile copy': ('pass', 'lifecycle prep (`eden_test.py new` from `tr2526-base.json`); Prune lines in each RESULTS.md'),
    'Copies:': ('pass', 'lifecycle prep: identical files linked, written files copied'),
    'Disk:': ('pass', 'builds through `remote-gate.ps1` (one lane target per host: `~/build-hosts/lanes/spots-3/target-*`); `df` checked by the gate (refuses < 75 GB) and the lifecycle (< 60 GB)'),
    'Eden: waits gated': ('pass', 'rows gate on `diag_settings_reload`, the hurt census line and the runner\'s image gates; fixed sleeps only after an input'),
    'Eden: test run plays a runner': ('pass', '`runner.py dry-run` READY before each Up (`%s/eden/dry-out*`); `eden_test.py` up/run/down; segments sha256-pinned (`sp-downb` built by `build_spots3_plan.py`)' % A),
    'Eden: rows grouped': ('pass', f['rows']),
    'Eden: failed precondition': ('pass', 'boot 1: kr-gutcheck (a missing walk segment) and sp-rage reran on the same boot (`--retry`); sp-rage moved first in boot 2'),
    'Eden: game-side facts': ('pass', 'the GDB stub was on (it excludes tracing), so the facts came from the diag build\'s own lines (`[SpotsKB]` raw record reads); the static facts from the owned ELF (`damage_reaction_matches_the_owned_game`)'),
    'Eden: profile restored': ('pass', f['boots']),
    'Eden: the run\'s copies': ('pass', 'Prune lines in `%s/eden/runs/*/RESULTS.md`' % A),
    'Eden: console.log': ('pass', f['console']),
    'Every local command': ('pass', 'builds and tests under the gate\'s `systemd-run --user --scope -p MemoryMax=… timeout … nice -n 19 ionice -c 3`; compliance under the same guard'),
    'Evidence doc': ('pass', '`%s/DESIGN.md`, `%s/eden/runs/*/RESULTS.md`, `REPLAY.md`, INBOX posts (spots-3, 11-15)' % (A, A)),
    'GIF caption': ('N/A: no README GIF in this lane (the Windboxes-only clip is row evidence, 640 wide)', '-'),
    'No operator quotes': ('pass', 'docs/menu text plain; DECISIONS.md paraphrases the order'),
    'Handoff file current': ('pass', '`artifacts/phase23/handoffs/spots-3-20261115.md`'),
    'Release: version bump': ('not authorized', 'no release ordered'),
    'Release: rebuild': ('not authorized', 'no release ordered'),
    'Card: MTP': ('not authorized', 'no card write ordered'),
    'Card: the user connected': ('pass', 'the card was never touched (read-only scan of a local copy)'),
    'Nothing pushed': ('pass', 'nothing pushed, tagged or written to the card'),
    'Model/effort gate': ('pass', 'a spawned worker lane under the primary\'s routing; no further workers spawned'),
}
out = []
for line in Path(sys.argv[1]).read_text().splitlines():
    cells = line.split('|')
    if len(cells) > 4 and cells[3].strip() == 'agent':
        key = next((k for k in D if cells[1].strip().startswith(k)), None)
        if key:
            res, ev = D[key]
            cells[3], cells[4] = ' %s ' % res, ' %s ' % ev
            line = '|'.join(cells)
    out.append(line)
Path(sys.argv[2]).write_text('\n'.join(out) + '\n')
left = [l for l in out if '| agent |' in l]
print('agent rows left:', len(left))
for l in left:
    print(l[:160])
