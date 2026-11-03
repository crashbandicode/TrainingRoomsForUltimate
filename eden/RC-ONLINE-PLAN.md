# RC online and waiting-room acceptance

2026-11-03. Pending execution on the final integrated candidate. This plan does
not turn a solo waiting-room observation into two-console or LDN acceptance.

## Evidence contract

The reviewed detector contract is the owned ssbu-desync-detector
docs/WHAT-IS-CHECKED.md on Yoga. Replay A/B compares every recorded check value
and position over the full replay. A position difference fails even if the RNG
check has not diverged yet. Online-only paths additionally require two consoles:
both session summaries must end with desync_seen=0 at matching frame numbers.
Replay equality alone does not qualify matchmaking, waiting-room departure or
session teardown.

Every run binds source, exact NRO/ELF, settings, native replay bytes, detector
NRO, Eden/runtime binaries, profile snapshot, inputs, full logs and screenshots.
Require zero TOTAL console Unmapped, zero hook allocation deltas, full process
stop and exact profile/config/plugin restoration before releasing each claim.
Keep failed attempts under their original labels. Final source must have passed
the integrated gates before its runs count as RC evidence.

## Qualified route and resources

Use the existing TrainingRooms harness, controller API and playbook
eden-pair route semantics; do not invoke the historical elite-rematch scripts
with their hard-coded foreign roots/owners. One Eden per host. Surface and Yoga
claims must both be admitted for a paired run; no compile beside either Eden.

The qualified private Yoga rig is /home/intpa/trainingrooms-eden-lead-astra4,
profile y, display :98. UI owns the current UI21 development lanes under separate claims and full
restoration between them. Re-read the canonical ledger and queue at admission;
this planning document is not evidence that either host is free. Source profile b is
read-only. The launch already supplies the existing LAN runtime at
/home/intpa/elite-rematch-work/eden-lan-runtime, SHA256
ee3c2e953a0b87541c29eb8918ce0354489035296ac5eaf060b3444d06a8bbcb.

A read-only preflight confirmed source/private configuration has LAN Online on,
airplane mode off and volume zero. Set TLS true only in the next owned private
profile after standard prep and before launch; record and restore the original
Qt bytes. Coordinate the same setting on the eventual second private peer.
Do not change public accounts, service state, firewall rules or source profiles.
No Ally hosting is included in this plan.

## Run batches

1. UI waiting-room batch, one Yoga process: Training Q15 constructor/UI context,
   then Main > Online > Smash > Quickplay > Solo > fighter selection > waiting
   room. Gate each segment by actual screenshot/log. Verify the waiting layout,
   menu, input ownership, notification behavior and CPU entry 4; compare Q15
   constructor and UI context from the SAME boot. Solo TLS has no test peer.
   This qualifies only the observed waiting-room/UI paths.
2. Replay baseline: detector and required aid only, exact retained native replay.
   Record complete detector session and frame artifact; then restore.
3. Replay candidate: same replay/configuration with the exact candidate added.
   Use the detector compare example against the two complete frame artifacts.
   Require equality of checks AND positions and complete matching session length.
   Preserve both raw artifacts and the exact comparison command/output.
4. Online pair: one private Surface console and one private Yoga console, detector
   on both, candidate on only one. Enter waiting practice, use the changed
   practice/UI path, let matchmaking transition to a match, complete the match
   and its following results/rematch/exit path. Both peers must record matching
   session frames with desync_seen=0; no forced diagnostic branch stands in for
   actual mode transitions.
5. LDN: separately exercise the approved local-wireless waiting-practice and
   lease/scene-departure rows. The online-mode result does not cover the distinct
   local mode tuple. Record native mode, actual UI readiness and entry ownership,
   then confirm cleanup when leaving the waiting scene.

## Source review still required

The actual widened practice predicate is runtime/scene.rs. Lobby activation is
published through features/cpu::set_lobby from lobby/training/lobby::update;
the waiting-scene predicate requires initialized melee 0x0d and the exact local
or online family. Departure clears UI readiness/session state. These boundaries
must be exercised in the paired transition; reading them is not a per-hook
online-safety verdict. Review every changed callback that can mutate simulated
state or call native RNG, including inherited callers, against its actual
registration and scene gate. Do not substitute an unchanged-code assertion or a
zero-Unmapped scan for detector equality.
## Detector staging identity checked 2026-11-03 08:01Z

Read-only inspection of the immutable Yoga source profile b found the active
plugins/libssbu_desync_detector.nro:483328bytes,SHA256
7603ad747733b5e55a52b8e69fc63e3a698bb4258c7e728ca6c845ca41d2113f.
The separately set-aside copy has SHA256
b01823ddefb329cc21c02f8eec0557d3a97ead3cf930bc6a4e0d390222433ecb;
it is a different binary and must not be silently substituted. No source or
live profile was modified. Rehash the selected aid at actual staging and use
the identical aid for baseline, candidate and both peers. This is an available
artifact identity, not a new qualification or proof of its version/source.
The detector README/WHAT-IS-CHECKED/HOOKS documents were reread: playback
frames contain computed and recorded position checks as well as RNG checks;
all must be compared across the complete retained replay. Full online session
summaries and separate LDN remain required. No replay or online run is claimed
by this inspection.
