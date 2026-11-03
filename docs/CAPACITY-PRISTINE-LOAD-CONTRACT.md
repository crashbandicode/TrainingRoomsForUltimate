# Capacity load verification before owned lifecycle hooks

## Observed failure

Frozen r10 `d9e1cd35ac087d26e1c63122b43763724d7e69a2` boots with
`NativeSourceCapacity` load clients disabled. In f6-31, diagnostic #40 records
successful lifecycle installation, followed by #43's full-range hash mismatch:
`+0x653240 len=592 expected=0xee56df013a817f73 actual=0xe5f015350709dd84`.
The retained console has zero Unmapped accesses; that does not establish that
disabled clients work.

`runtime/roster.rs` installs `character_lifecycle` before `character_roster`.
The lifecycle initializer installs `reload_trace` at `0x65325c`, 28 bytes into
the later capacity check's `[0x653240, 0x653490)` range. The capacity initializer
then hashes already patched instructions as if they were pristine. This owned
registration/order conflict is sufficient to explain the rejection.

All thirteen configured load-range FNV hashes match the owned 13.0.5 ELF
`0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2`.
The 64-byte entry window and full ranges are retained privately under
`artifacts/trainingrooms-nro-archive/slim/capacity-guard/stock-audit.json` and
adjacent binary files. Eight reviewed frozen source files match r10's source
manifest. The runtime log contains a hash, not live bytes: no reconstruction of
the actual patch target or exclusion of additional runtime modifications is
claimed.

## Repair contract

Verify every existing full load range and grouped actor vtable before lifecycle
patches. Carry a non-copyable successful proof tied to that exact text base into
the existing capacity initializer. A failed check, absent proof, zero base or
different base must refuse load activation. No hook is installed by verification.

Keep activation in its existing position after ordinary Mii validation; retain
the capacity creation guards and later 64-byte seam gate. Do not replace expected
hashes with patched hashes, skip changed bytes, normalize hooks, weaken guards,
or move whole initializers. `listener_removed` at `0x652214` lies inside the
ordinary gate's destructor contract `[0x652160, 0x652310)`, so simply moving
capacity activation earlier would introduce another ordering conflict.

Other known load-range hooks are installed later: body-bank preparation at
`0x654dc0`, context/pair hooks inside the `0x655320` range, and capacity's own
actor cleanup at `0x6b79a0`. Their existing activation order remains intact.
The proof is a boot-local value, not serialized state or a global readiness flag.
`LOAD_VERIFIED` must still publish only after the actor cleanup hook is installed.

## Required evidence

Original host probes must execute the production fingerprint/proof helpers on a
synthetic byte fixture, including the observed interior-offset patch sequence,
preexisting corruption, late verification, absent proof, zero/different base,
and successful unchanged activation. No private native bytes enter test fixtures.
Native compilation remains deferred to the next coherent candidate; targeted
host tests, exact native callbacks/stack checks and a fresh strict boot showing
managed clients enabled are required before runtime acceptance. Lead independent
review is required; pkg6 retains the provider implementation beyond this narrow
verification boundary. The original investigation box is 03:24–04:09Z.

## Implemented development slice

The boot-local `VerifiedLoad` token and existing FNV calculation live in the new
pure `model/swap/pristine_load.rs`, included directly by the native capacity
module and its host regression. `roster::init` obtains the optional proof before
lifecycle installation; the character-roster initializer passes it to capacity
at its original activation point. No global proof cache was introduced.

One admitted Surface standalone rustc 1.98.1 job, edition2021 with `-Dwarnings`,
passed four original tests at03:34Z and released immediately. Source hashes were
equal before/after. Executed binary:
`098544437f385badd9f02306ff582966326577b1281f8ecd2a028788f16abb11`.
Raw test log:
`28235120c4f61e760985a31e1278608f99b7566a944bb003bcd7b0ca6c573cba`.
Receipt: `artifacts/trainingrooms-nro-archive/slim/capacity-proof-host/receipt.json`.
This qualifies the pure helpers, not native compilation or boot wiring. Full
253-file carry verification against m5q passed. Lead review, targeted Cargo
`pristine_load`, and next-candidate native/runtime qualification remain required.
