# lr-09 results — 2026-11-03

## Outcome

- **Artifact integrity:** executed rows, hook, runner, lifecycle and library
  match the pre-Up pins in `provenance.json`. All 22 receipts bind the exact
  executed manifest. Export tar and every exported file verified after transfer.
- **Observed behavior:** 22 PASS, 0 FAILED, 0 unrun. Three warm Player Falcon /
  CPU Gunner → native Quit → clean retirement → CSS CPU normalization → fresh
  Steve/Lucas Training cycles, followed by another Player Falcon swap. Four
  checkpoint sheets were visually reviewed against the original screenshots.
- **Acceptance:** these behavior clauses are positive. **Clean-console health
  remains failed/unresolved**, so RC-TRIO-END's clean-log clause and whole RC
  acceptance remain open. Pending-build teardown is a separate untested clause.

## Exact identities and turn

AppImage:
`/home/intpa/Applications/Eden-GIF/20261103-063f1b80bb2c-persistent-tracing-6a0c80d2b858/Eden.AppImage`

SHA256: `6a0c80d2b85861fa0d92d5ef6238af7d08a8da94e50e20446a2a54120bee94eb`.
Eden source `063f1b80bb2cc2a729019cef4cdd40b460fc574f`.
Product RC8 `cf4bf44c104ecc96a78d382b4c0276bf33838f18`, diagnostic NRO
`3ca7f0c5c9e5552be4725050949b1ba96266cf4f5875bef660f94ba9b323e9e4`.
Playbook `07dcb7e085c3be7c5eeffebfc54dc739000173e0`.
lr02–lr08 remain old-build evidence, bound to AppImage
`d483c082147eb6a5838ca90b8c0988baec8f8e7bc662234215069a4746215570`.

GO 21:23Z → Up **21:30:26.967172Z** → RELEASED **21:34:08.158698Z**.
Claim `b2159383ffee457fbbf75ad87b40c019`. Runner and Down both exit 0;
Down ran in caller `finally`. Raw `Deadline: terminal game.stop` retained.
Lifecycle `cleanup_failures=[]`, full restore and separate verify passed.
Independent audit: source and restored profile equal the 21,410-entry pristine
snapshot; 252 immutable links, 20,436 private files, zero topology violations,
owned PIDs or open claims. Peak memory 6,706,057,216 B, below the 8 GiB cap.

## First live lifecycle trace integration

Generated `trace.json`, launch configuration and `instances.json` source alias
worked. Actual IDs resolved armed: install `main+0x653240` = 1, finalize
`main+0x6524e0` = 2. Main base `0x80874000`; runtime addresses match offsets.
Module inventory is retained in `receipts/01-main.json`.

`python3 -B -I eden/lr09-shared/analyze_trace.py` regenerates
`trace-analysis.json`; its three offline tests passed. The five pre-Up
trace-pattern/resolution tests also passed.

- 96 consecutive records: install 18 entry + 18 return; finalize 30 + 30.
- 48 balanced point/process/thread stack pairs. Counters agree with the file.
- Zero dropped, file-lost, abandoned-return or overflow counts; empty boot and
  output errors. Final hook explicitly disarmed both points before collecting
  counters. Final copied trace equals the retained run trace.
- Fresh-row CPU install entries have owner 1 / incoming kind 74 / reload flag 1
  in all three cycles. Fresh-row CPU finalize entries have owner 1 / kind 74.
  Existing diagnostic gates establish the separate ready/retirement outcomes.
- **Capture limitation:** 25 failed typed reads occur on return records, none
  on entries. `on_return` evaluates the same expressions using return-time
  registers; entry argument pointers are no longer valid. Successful return
  reads also cannot be interpreted as preserved entry arguments. These rows
  gate only on entry captures. The stream is retained without filtering.
- Failure-path final counters are not qualified: this run reached its final
  hook. A future early failure still needs counter capture before Down.

These observations do not prove `bank_build=false`, construction completion
from tracing alone, saved-Mii pointed identity, or CPU detached-bank retirement.
The two finalize entries per CPU Quit are retained; no exactly-once claim.

## Console health finding

Full console SHA256
`188561571b5e81237f69099995ef9c7fc3f5381872f4e71181fa5177290996b9`:
829,407 bytes / 5,865 lines, zero Unmapped matches, **one panic keyword hit**.
The raw tail also contains `fatal error: fault` and SIGBUS:

```text
2026/11/03 21:33:30.546825 [secure] disconnected ...
unexpected fault address 0x70e080
fatal error: fault
[signal SIGBUS: bus error code=0x2 addr=0x70e080 pc=0x70e080]
goroutine 88 ...
panic during panic
SIGBUS: bus error
```

Final row ended 21:32:59.578966Z. `game.stop` request was 21:33:00.463Z;
forced-stop completion was 21:33:31.122620Z. The fault is in the shutdown
tail after the timed disconnect. Go formatting suggests a host Go component;
the emitting executable and root cause are **not established** by this combined
console. Do not equate this with the otherwise expected stop deadline or blame
the guest/plugin/tracing without further evidence. The original console and
control/lifecycle journals are retained. Existing eden-tracing worker notified
through mailbox `lead-lr09-trace-and-shutdown-findings`; delivery was queued.

The full scan command is the retained raw adapter `lr04-scan-evidence.py` over
`final-state/run-yoga/console.log`. Receipt/archive/restoration numeric adapters
remain externally retained and not fully host-tested; that older provenance
gap remains open. The new trace analysis has its own committed tests.

## Harvest and remaining work

Canonical harvest added zero segments and two top-level gates:
`lr06-catalog-cycle2`, `lr06-catalog-cycle3`. Hook-internal trace/native checks
are project evidence, not newly harvested library entries. Row01's capped main
gate failures and successful recovery remain in raw outputs; final row receipts
are unchanged.

Next: resolve the shutdown health attribution, then continue the outstanding
busy/pending-build teardown and other Phase1 clauses under the next fresh label.
Remaining-row owners are in the lead inbox. Phase2/3 composition remains blocked
on Phase1 acceptance. No new agent, product rebuild, push, tag, release, card,
DBI or USB operation occurred. Current scope ends with lr09 closure.
