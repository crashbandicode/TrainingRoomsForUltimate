# The Eden harness (`eden/harness/`)

One parameterised copy of the playbook's `tools/eden-solo/gv.py` and `solo.ps1` (playbook `ae87443`), one route, one
claim-ledger module and one Ally variant. They replace nine near-copies of the harness and about fourteen copies of
the route and drivers (list at the end). Host-tested by `tests/eden/test_harness.py` (run by `cargo test` through
`tests/eden_harness.rs`); no Eden is started by any test.

| File | Playbook origin | What it is |
| --- | --- | --- |
| `params.py` | the constants block of `gv.py`, the `$work/$coord/$owner/$display` lines of `solo.ps1` | every value a stream used to edit: `TR_*` variable, config file, default |
| `gv.py` | `tools/eden-solo/gv.py` (same subcommands and names) | prep, launch, shot, tree, press, stick, call, keepalive, stop; added: mute, macro, lines, scan, restore, collect, reload, preflight, params |
| `solo.ps1` | `tools/eden-solo/solo.ps1` | Up / Down: preflight, claim, prep, display, GPU lock (Yoga), launch, mute, game.load; Down always releases |
| `step.ps1` | `tools/eden-solo/step.ps1` | unchanged (the label pattern is `gv.py`'s) |
| `ledger.py` | the coordination-file block of `solo.ps1`; `docs/EDEN.md` "Queue runs" | parse, open claims per host, claim, release, fairness, queue head, the interleave rule |
| `route.py` | `docs/MACROS.md` hybrid, `docs/TILE-CURSOR.md`, `examples/css_check.py`'s marker idea | the screen-gated route (local, ssh or Ally transport) and the gated primitives for row scripts |
| `deploy.py` | none | harness + config + macros + NROs to the Eden host's `<root>` |
| `ally/wa.py`, `ally/stage.ps1`, `ally/client.py` | `eden/ally/wa_s2.py`, `wa_s5.py`, `eden/lent/wui.py`, `stage_ui.ps1`, `ally_ops.py`, `ally_launch.py`, `ui_lent.py` | the Windows variant, same parameter set |

## Parameters

Resolved by `params.py`: the caller, then `TR_<NAME>`, then `TR_CONFIG` (else `harness.config.json` beside it), then the
default. `deploy.py` writes the config on the Eden host, so a host-side run needs no environment. Example:
`eden/harness/harness.example.json`.

| Parameter | Default | Replaces (in the copies) |
| --- | --- | --- |
| `owner` | none (required to claim) | `OWNER` / `$owner` |
| `prefix` | none | the label regex (`s6-\d{2}`, `f6-\d{2}`, `tr\d{3}`, ...); labels are `<prefix>-NN` |
| `root` | none | `ROOT` / `$work` (`trainingrooms-eden-p1`, `-pkg6`, `-f6`, `-st`, ...) |
| `profile` | `y` Yoga, `g` Surface | `PROFILE` (`t1`, `t4`, `s6`, `f6`, `hb`, ...) |
| `source` | Yoga `b`, Surface `g` | `SOURCE` |
| `display` | none | `DISPLAY` / `$display` (`:93` ... `:99`; `:61` is refused) |
| `host` | from the machine's name | `SURFACE = socket.gethostname() == ...`, `HOST = "yoga"` |
| `runs` | `<root>/runs` | `run_dir` |
| `nro`, `pilot`, `tile` | none, none, `/home/intpa/ssbu-tile-cursor/dist/...` | the `-Nro` lists |
| `output`, `extras` | `TrainingRooms`; `ssbu-tile-cursor`, `ssbu-cpu-pilot` | `OUTPUT`, `OTHER_OUTPUTS`, the deluxe and hitbox extras |
| `unit` | `tr<prefix>` | `UNIT`, the `trp1` / `trp6` / `trf6` unit names; the aside folder follows (`plugins-set-aside-<unit>`) |
| `refs`, `macros` | `eden/refs`, `eden/macros` | `REFS`, `MACROS` |
| `install_gate` | `result=installed` | the hard-coded install-line wait (the integrated tree's line is cut before `result=`: `plugin=training_rooms/`) |
| `stale_hours`, `fair_seconds` | 12, 180 | `STALE_CLAIM_HOURS` (6 or 12); the 3-minute rule |
| `ledger`, `ledger_ssh`, `queue`, `stream` | the Yoga's file, the Yoga, the queue file, the prefix | `COORD`, `YOGA_SSH` |
| `ally_root`, `ally_claim`, `ally_mem_gib`, `ally_eden_sha` | the lent rig `C:/Eden/c1162-w5-ally`, none, 11, the rig's pinned `eden.exe` | `O`, `OWNER`, `MEM_GIB`, `EDEN_SHA` of `wa_s2.py` / `wa_s5.py` / `wui.py` |

Per-host facts (`params.HOSTS`): the Yoga forces nothing extra and has the GPU lock; the Surface runs offline
(`lan_online=false`, `airplane_mode=true`), has no GPU lock, has the game under `eden-assets` and reaches the ledger over ssh.
Every profile copy gets `pauseWhenInBackground=false`, `volume=0`, `volume\default=false`.

## Use

Common: the claim rules of `TRAININGROOMS-STREAM-RULES.md` are enforced by `ledger.py`, which `solo.ps1 -Action Up` calls
before anything else: no Eden here, display free, MemAvailable >= 10 GiB, `df` >= 40 GB, then the claim is refused while any
claim on that host has no later RELEASED (foreign owners included; stale after `stale_hours` and listed, not blocking),
within `fair_seconds` of your own RELEASED, and while `imgui-nvn-sol` or `tm-f8-terra*` (`-ally`, `-eden`; matched by name
prefix, each on its own; `ledger.PARALLEL`) is queued for the host and has not had it since the last TrainingRooms release
there (queue file interleave rule). The harness starts no cargo command: `CARGO_INCREMENTAL=0` (the lead's disk order)
applies to whoever runs `cargo test` for `tests/eden_harness.rs`. `Up` fails with exit 3 and the reasons; wait with
`ledger.py wait <cap seconds>` (checks of at most 60 s) and claim in the same call with `ledger.py wait <cap> "<claim text>"`.
A claim line must name its host as "on Yoga" / "on Surface" / "on Ally" and no other host.

**Surface, Eden on the Surface** (display and root are yours):

```text
$env:TR_OWNER='Claude <session>/<stream>-eden'; $env:TR_PREFIX='st'; $env:TR_ROOT='/home/intpa/trainingrooms-eden-st'; $env:TR_DISPLAY=':99'
pwsh -File eden/harness/solo.ps1 -Action Up -Label st-01 -Nro /abs/libtraining_rooms.nro -Tile -Pilot -Settings settings.json
/usr/bin/python3 -I eden/harness/route.py st-01                      # --stage-steps, --tile-gate, --p1-steps, --training-gate ...
/usr/bin/python3 -I eden/harness/gv.py scan st-01                    # exit 1 on a failing Unmapped line
pwsh -File eden/harness/solo.ps1 -Action Down -Label st-01 -Restore  # scan, restore the profile, RELEASED
```

`Up` copies nothing: run it from a checkout on the Eden host or after `deploy.py`. On the Surface `TR_PILOT` and `TR_TILE` are
that host's paths.

**Yoga, driven from the Surface** (the Yoga has no checkout; `TR_HOST=yoga` names where Eden runs):

```text
$env:TR_HOST='yoga'  (plus the four above)
/usr/bin/python3 -I eden/harness/deploy.py --nro /abs/libtraining_rooms.nro     # prints the NRO paths on the Yoga
ssh memento-claw-yoga-ubuntu "pwsh -File <root>/harness/solo.ps1 -Action Up -Label st-01 -Nro <root>/nro/libtraining_rooms.nro -Tile"
/usr/bin/python3 -I eden/harness/route.py st-01                      # transport ssh (automatic): one gv.py call per command
ssh memento-claw-yoga-ubuntu "pwsh -File <root>/harness/solo.ps1 -Action Down -Label st-01 -Restore"
```

A relaunch inside a claim you hold (the file set changed): `Down -KeepClaim`, then `Up -NoClaim`.
Rows: `route.session(label)` gives `press`, `stick`, `shot`, `wait_line`, `wait_match` and `macro` (sha256 receipts in
`<runs>/<label>/macros.jsonl`); `gv.py lines <events|diag|tile|path> [--after N] [--count]` reads a plugin log;
`gv.py collect <label>` copies logs and receipts to `eden/runs/<label>/`.

**Ally** (Windows Eden; the lent rig only, never `C:\Eden\trainingrooms-pkg2` / `-pkg5`):

```text
$env:TR_ALLY_CLAIM='trainingrooms-<stream>-0caa72b8'; $env:TR_PREFIX='st'   # TR_ALLY_ROOT defaults to the lent rig
/usr/bin/python3 -I eden/harness/ally/client.py deploy          # wa.py, wa.config.json, stage.ps1, macros -> <rig>\tr-<prefix>\
/usr/bin/python3 -I eden/harness/ally/client.py stage set1 libtraining_rooms.nro libssbu_tile_cursor.nro ...
/usr/bin/python3 -I eden/harness/ally/client.py setaside        # CLAIM -> this stream, the profile aside (manifest first)
/usr/bin/python3 -I eden/harness/ally/client.py boot set1 ; launch st-01 ; game st-01     # game: mute, load, wait
$env:TR_HOST='ally'; /usr/bin/python3 -I eden/harness/route.py st-01     # AllyTransport
/usr/bin/python3 -I eden/harness/ally/client.py kill st-01 ; restore   # restore verifies against the pre manifest, puts the CLAIM back
```

Claims for the Ally go through the same ledger (`TR_HOST=ally`: "on Ally"). **Firewall:** the harness has no firewall step and never adds, changes or removes a rule. The lent rig's inbound Allow rules are the operator's, placed on purpose, so Training rows do not prompt; never answer or dismiss a prompt. A Block rule for a rig `eden.exe` may be added by hand (operator, 2026-11-02): record name, path and UTC time; the harness has no helper for it (Training rows need none; LDN/online from the Ally needs the lead's OK).
The Ally variant could not be run from here; its host tests cover the log wait and the parameters only.

## What the copies differed in (measured against `eden/tr_eden.py` at `7bb86fd` and the streams' tips)

Varying values (now parameters): work root, profile tag, owner id, display, label regex, unit and aside-folder names,
source profile and game path per host, `STALE_CLAIM_HOURS`, install-line gate, training-scene gate, ssh ControlPath
(`trssh-%C`, `trssh3-`, `trssh5-`, `trssh-u4-`), the Ally root, claim text and Job memory. Displays collided across copies
(`:93` p1 and r2fix, `:95` pkg2 and pkg5, `:98` pkg6 and r1fix): `preflight` now fails when the display is in use.

Behaviour additions, each folded in once:

| Addition | Origin | Here |
| --- | --- | --- |
| mute: `core.volume=0` via `settings.set/save/get` before `game.load`, and `volume=0` in the profile's config | tr_eden.py, stack's `solo.ps1`, `wa_*.py` `mute` | `gv.py mute` (fails unless the read-back says 0) + `prep` |
| `Unmapped` scan | tr_eden.py `scan` (a grep, no verdict) | `gv.py scan`: fails below `0x1000` or inside `--window`, lists the rest and panic/abort/crash |
| profile restore (sha256 against the set-aside) | tr_eden.py `restore` | `gv.py restore`, `Down -Restore`; unreadable fixture folders handled |
| ledger parser | tr_eden.py (6 h), s2/s6/f6 (host-aware, 12 h), pkg6 tip and f6 (seconds stamps), `eden/ally/ledger.py`, `/tmp/integrator3` | `ledger.py` (all of it, Codex owners included, release closes only the hosts it names) |
| screen-gated route | route4.py family, route_s2/s6 (local), f6/hb install gate and training line, `ally_route.py`, `ui_lent.route` | `route.py` |
| rows seeding: `settings.json`, `--keep-data`, `developer.ini`, `--save-states`, `--extra`, `--seed`, `--sd-file`, `--fork-fixture` + `unfix` | tr_eden/s3/s5/w/r2/hb/tr_eden4/dx | `prep --put/--settings/--developer-ini/--diag-txt/--seed/--sd-file/--tree/--keep-data`, `unfix` |
| `reload`, `collect` | s2, s6, f6, dx | same names |
| claim before Up, release after Down also after a failed Up; relaunch inside a claim | tr_eden.py `up`; f6 `claim=False` | `solo.ps1` (`-NoClaim`, `-KeepClaim`) |
| macro sha256 receipts (MACROS "Receipts"; none of the copies kept them) | none | `gv.py macro` |
| Surface host awareness (offline config, no GPU lock, ledger over ssh) | s2, s6, f6, dx | `params.HOSTS` |

Not unified, on purpose: setting a CPU on the character select (TILE-CURSOR "To set a CPU": no copy implemented it;
`route_s6.py`'s `--cpu-steps` was documentation only) and the pair tools (`tools/eden-pair/`: no stream used them).

## Copies this replaces (at their next run; nothing is deleted by this branch)

Harness: `eden/tr_eden.py`, `eden/u4/tr_eden.py`, `eden/u4b/tr_eden.py`, `tr_eden4.py`, `s2_eden.py`, `s3_eden.py`, `s5_eden.py`,
`s6_eden.py`, `tr_eden_w.py`, `tr_eden_r2.py`, `tr_eden_hb.py`, `f6_eden.py`, `dx_eden.py`, `eden/stack/gv.py`, `eden/stack/solo.ps1`, and the two
`tr_eden*.config.json` files. Route and drivers: `route4.py`, `route_local.py`, `route_r2.py`, `route_s2.py`, `route_s3.py`,
`route_s5.py`, `route_s6.py`, `route_w.py`, `route_hb.py`, `route_f6.py`, `route_dx.py`, `u4/route_u4.py`, `drive_s5.py`, `u4/drive_u4.py`,
`hb_drive.py`, `look.ps1` (`gv.py shot`), `calm_then.py`, `seq.py`. Ally: `ally/wa_s2.py`, `ally/wa_s5.py`, `ally/ally_route.py`,
`ally/ledger.py`, `ally/ally_ops.py`, `ally/ally_launch.py`, `ally/ally_conf.py`, `lent/wui.py`, `lent/stage_ui.ps1`, `lent/ui_lent.py`.
Row scripts (`rows*.py`, `st_run.py`, probes) stay and import `route.session`.
