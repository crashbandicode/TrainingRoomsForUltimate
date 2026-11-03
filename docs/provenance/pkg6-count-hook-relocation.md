# Count-hook placement correction

The previous inline site `155c16c` is unsafe in Skyline's far placement mode.
It is four modulo eight, so the installer overwrites five instructions through
`155c17c`. Native `CBZ x12` at `155c164` targets `155c174`, inside that patch.
A matching instruction guard did not make this placement safe.

The corrected site is `155c188`, eight-byte aligned. Its four displaced words
are `BIC w10,w10,w10,ASR #31`, `CMP w8,w23`,
`STR w10,[x20,#0x1c4]`, and `BEQ 155d9a0`. The nearby null-entry branch remains
intact. A whole-executable aligned direct B/BL/B.cond/CB/TB scan of the owned
game ELF found no incoming edge into this replacement span. This does not
prove absence of computed indirect targets.

At this point w8 is the requested count, x23 the previous count, w9 the
cooldown, and w10 its already-computed decrement. The callback reacquires the
canonical manager singleton; x9 is no longer a manager pointer. For an
unfinished new seat, active managed install, failed promotion reservation or
unavailable manager, it substitutes w9=6 and w10=5. Native then stores its
existing five-tick cooldown and the original `CMP w9,#1; BGT` returns before
the committed-count store. Equal/invalid counts leave the context untouched.
Normal readiness leaves the original decrement and count reconciliation intact.

Incoming NZCV is dead at the displaced CMP. Its newly produced flags must
survive both conditional-branch relocation and the trampoline tail. The owned
function `155c020..155d9c0` has no x16/x17 operand; return continuations and the
actual Skyline relocation are independent review gates, not assumed by this
source change. No native activation or runtime acceptance is claimed here.

Inputs: owned game ELF SHA256
`0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2`;
retained addressed disassembly in
`artifacts/trainingrooms-nro-archive/pkg6/f9-native-count/pkg6-count-promote-full.txt`;
direct-edge receipt `/home/intpa/smash-training-lobby/tmp-pkg6/count-interior-audit.json`.
The seam retains a full 64-byte guard at the new site, plus the existing input
and retry guards. Original regression checks the actual null-entry branch,
its untouched destination, new alignment and exact displaced words/BEQ target.
The test and native adapter remain uncompiled pending the coherent candidate.

## Integrated loader qualification

Exact Yoga Skyline 9959ecb1 static conditional relocation is qualified in adjacent count-conditional-qualification.json. Far BEQ emits condition-preserving BEQ +8, B +20, LDR x17 literal and BR x17 to original155d9a0; false path continues via the qualified tail to155c198. Near hook displaces only BIC; original CMP/STR/BEQ remain. Both preserve newly produced flags. Combined with F6 exact110fb77 source/liveness review, this closes static placement review only. Compiled callback stack and live count transitions remain mandatory.
