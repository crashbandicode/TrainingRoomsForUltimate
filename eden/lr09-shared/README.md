# lr-09 — first new-build lifecycle trace continuation

**Closed: 22/22 execution PASS; console health FAIL/unresolved.** 2026-11-03.
See [RESULTS.md](RESULTS.md). All three warm quit/reentry cycles and final
continuation were observed. Shared Down and independent restoration passed;
Yoga released at 21:34:08.158698Z. A Go SIGBUS/panic in the shutdown tail
prevents clean-log acceptance. RC remains NOT READY.

PRIMARY21:23Z GO consumed. Shared dry-run22rows/57gates/20recordingsREADY;
five focused trace-pattern/resolution tests passed. Freshstate/profile,
runner480s/guest900s, canonicalDown in callerfinally. Strict admission applies.

## Pinned build and product

AppImage exact resolved path:
`/home/intpa/Applications/Eden-GIF/20261103-063f1b80bb2c-persistent-tracing-6a0c80d2b858/Eden.AppImage`

SHA256`6a0c80d2b85861fa0d92d5ef6238af7d08a8da94e50e20446a2a54120bee94eb`;
freshYoga readback collected. Tracing actualrelease21:22:26Z observed.
ProductRC8cf4bf44 / diagnostic3ca7f0c5 unchanged. lr02–lr08 evidence remains
labelled oldAppImaged483c082; this run is a new-build cohort.
Playbook07dcb7e085c3be7c5eeffebfc54dc739000173e0; runtime tool pins and
pre-Upmanifest inraw`rc8/lr09-shared/input-provenance.json`.

## Trace contract

First live use of shared lifecycle per-instance`trace`: two preboot points,
native install`main+0x653240` and finalize`main+0x6524e0`,256hits perpoint,
1000records/s caps, entry/return recording. Source ABI:
`native_fighter_install.rs:122–135` passes entryx0/descriptorx1/flagsx2;
owner isentry+0xe0, incomingkinddescriptor+0x18, firstreloadflagbyte=1.
`mii_ordinary_cpu.rs:1065–1077` dispatches nonbank requests to native reload;
`native_body_bank.rs:4071–4085` identifies finalize(entry).

Typed captures use address expressions`x0+0xe0`, `x1+0x18`, `x2` rather
than an extra pointer dereference. Hooks resolve actual IDs fromtrace.list,
refuse unresolved/nonarmed points, and match fresh-rowtrace_jsonl records
with successful typed reads forCPUowner1/Gunner74/reloadflag1. Existing
diagnosticready/released/HUD and cleanretirement outcomes remain separate.
Native install-entry observation is **not** a bank_build=false assertion or
construction-completion proof. Finalize-entry trace supplements clean outcome
gates. No guessing fixedpointIDs, no ELFalias needed, no product rebuild.

lr08's emittedCPUstaged/ready pair was lost to the gate afterlogrotation;
bothfiles/failedreceipt remain in77c999a. New trace-backed checks observe the
native operation through a separate stream; they do not rewrite lr08PASS.
Any trace wiring/pointresolution/capture/loss gap must be reported. Tracefile,
generatedtrace.json, modules/actualIDs/counters and controlreceipts are closure
evidence; hooks are project code, not sharedengine copies.

Threewarmquit/reentrycycles plusfinalcontinuation planned. RC NOT READY;
pendingbuild/Miipointedidentity/remainingacceptance remainopen. No newagents,
productsource/build/publication/device actions.
