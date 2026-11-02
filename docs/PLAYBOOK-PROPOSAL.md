# Proposal for the playbook: parameterise `eden-solo` so a project can drop its copy

For the operator to accept or refuse; `~/ssbu-mod-playbook` was only read. Basis: playbook `ae87443`; the working
implementation is `eden/harness/` in this repo (`docs/HARNESS.md`), host-tested. TrainingRooms kept nine copies of
`gv.py`/`solo.ps1` because the playbook's are gsp-visualizer's: owner, label pattern, root, profile, display and plugin
folders are literals, and nothing handles settings files, mute, restore, the Unmapped verdict, or a ledger with more than
one host. Each copy fixed the same things differently.

## 1. `tools/eden-solo/params.py` (new), read by `gv.py`, `solo.ps1` and the examples

Order: caller, `TR_<NAME>`, `TR_CONFIG` or `harness.config.json` beside the file, default. The playbook's own values become
the defaults of its own config (`gsp-visualizer`'s file), so gsp-visualizer changes nothing.

| Parameter | Replaces in `gv.py` / `solo.ps1` | Notes |
| --- | --- | --- |
| `owner` | `OWNER`, `$owner` | ledger key: first two words |
| `prefix` | `label_ok`'s `e\d{2,}`, `ValidatePattern('^e\d{2,}$')` | labels `<prefix>-NN` |
| `root`, `runs` | `ROOT`, `run_dir`, `$work` | run folder defaults to `<root>/runs` |
| `profile`, `source`, `host` | `SOURCES`, `PROFILES`, `SOURCE`, `PROFILE` | per-host source (Yoga `b`, Surface `g`) and game path from one `HOSTS` table |
| `display` | `$display = ':68'` | `:61` refused |
| `nro`, `pilot`, `tile` | `-Nro` default | tile cursor and CPU pilot as flags (`--tile`, `--pilot`) |
| `output`, `extras` | `OUTPUT`, `OTHER_OUTPUTS` | the plugin's data folder and the folders archived to `.before-<label>` |
| `unit` | `gv-<label>-display`, `gv-<label>-gpu-lock` | the aside folder (`plugins-set-aside-<unit>`) follows |
| `ledger`, `ledger_ssh`, `stale_hours`, `fair_seconds` | `$coord`, the `88c0f449` literal | see 3 |

`HOSTS` carries what differs per host: source profile, game file, forced config keys (the Surface runs offline), and
whether there is a GPU lock (Yoga only). `FORCED` gains `volume=0`, `volume\default=false`.

## 2. `gv.py` subcommands to add (each exists in `eden/harness/gv.py`, tested)

`mute` (`settings.set/save/get core.volume`, fails unless the read-back is 0; `solo.ps1` calls it before `game.load`),
`macro` (play, wait, `input.reset`, sha256 receipt in `runs/<label>/macros.jsonl`: closes MACROS "Receipts"), `lines`
(plugin log lines or count, for gated waits), `scan` (EDEN "Read the results" as a verdict: Unmapped below `0x1000` or inside a
window fails), `restore` (set-aside NROs back by sha256, data folders removed), `collect`, `reload`, `preflight`
(no Eden, display free, MemAvailable, `df`), `params`. `prep` gains `--put PATH[:NAME]` (settings.json, developer.ini, any
real file), `--seed DIR`, `--sd-file`, `--tree`, `--keep-data`. `solo.ps1` reads its values from `gv.py params`, runs
`preflight` first, claims through `ledger.py`, always releases (also after a failed Up), `-NoClaim`/`-KeepClaim` for a
relaunch inside a claim, `-Restore` on Down.

## 3. `tools/eden-solo/ledger.py` (new) and one paragraph in EDEN.md

The `(CLAIMED|RELEASED): (\S+ \S+)` parse misses real lines: stamps with seconds (`12:34:56Z`), `CLAIMED again:`,
`CLAIMED (build only ...):`, owners like `Codex <uuid>/<stream>` and `Claude tangent <name>`, and it keys by owner only, so a
RELEASED naming one host closes the owner's claims on every host. `ledger.py` handles those, is host-aware (a claim is ON
the host its "on <Host>" phrase names), refuses while any foreign claim on the host is open (listing, but not honouring,
claims older than `stale_hours`), enforces a wait after the owner's own RELEASED, writes stamps without seconds (older
parsers read only that form), reads the ledger over ssh from the machines that do not hold it, and exposes the queue file's
head per host. Its tests parse real ledger lines. EDEN.md would say: a claim line names its host as "on <Host>" and no
other; waits are checks of at most 60 s.

## 4. `tools/eden-solo/route.py` (new) and a line in MACROS.md

The hybrid route written once: install-line gate, A until the menu's reference screenshot matches, the macro segment,
`settled screen=sss at=N` from the tile cursor's log read only after the stage select appeared, `lstick,<dir>` picks, each
screen waited on by a reference-screenshot distance with a cap, exit 2 on a miss, never pressing on onto an unseen
screen. Parameters: stage steps, tile gate, P1 steps, install gate, training gate (a screenshot, or a plugin log line), the
refs folder, and a transport (local, ssh to the host that runs Eden, or the Ally). Reference screenshots stay out of git.

## 5. Windows rig (only if the operator wants Ally in the playbook)

`wa.py` + `stage.ps1` + `client.py` take the rig root (`TR_ALLY_ROOT`, default the lent rig), the rig's CLAIM text, the
Job memory limit and the pinned `eden.exe` hash as parameters instead of three copies per stream.

## Cost and what stays out

The harness is about 2,300 lines (`eden/harness/`), against the 5,100 lines of harness and route copies the pbaudit counted (`docs/pbaudit/TOOL-INVENTORY.md`) plus the Ally scripts. In the playbook it is added once; TrainingRooms then keeps only `harness.config.json`, its macros and its row scripts. Not proposed: the pair
tools (`tools/eden-pair/`), which no stream used, and CPU-coin setting on the character select, which nobody implemented.
