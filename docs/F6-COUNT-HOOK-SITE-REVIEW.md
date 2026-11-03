# F6 count-hook replacement review

Reviewed proposal: hook 0x155c188, displaced far window [0x155c188,0x155c198), busy callback sets w9=6 and w10=5 while preserving requested w8 and old x23. This supersedes the withdrawn 0x155c184 four-word proposal. Box acknowledged at actual clock 2026-11-03 04:47:32Z, deadline 05:02:32Z. Some upstream headings were future-labeled 04:49/04:53; they are not timestamps of this review.

## Bounded result

Site-specific incoming NZCV/x16/x17 liveness, direct interior-target scan and native cooldown/no-commit flow qualify for the 188 proposal. This does NOT yet qualify activation: actual Skyline relocation of the displaced B.EQ at 194, final source/guard correspondence, compiled callback stack and runtime remain required.

Evidence: /home/intpa/smash-training-lobby/tmp-f6-worker/count-site-review/audit.json and function.asm. Exact owned ELF SHA256 0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2. Fresh full native disassembly 155c020..155d9c8 SHA256 4e5396f8f753a6b4931c018614213e8a1d104ba550a1754c3332e202ca1dec41. Independent scan used executable ELF sections (producer scan used executable load segments), all aligned B/BL/B.cond/CB/TB encodings. Unit bounded 512 MiB/60s; actual7.905s/96.4MiB.

## Window and control flow

Old hook 155c16c is 4 mod8. Its far overwrite is five words [16c,180); native CBZ x12 at164 ->174 enters its interior. Independent scan reproduced exactly that incoming edge. Old placement is rejected.

Withdrawn184 is also4mod8. Its far overwrite is [184,198), FIVE words, including194 B.EQ. The four-word assumption was false. Neither producer statement nor this review treats184 as a four-word literal-copy hook.

Proposed188 is0mod8. Four displaced words are:

| Address | Word | Native instruction |
| --- | --- | --- |
| 155c188 | 0aaa7d4a | BIC w10,w10,w10,ASR#31 |
| 155c18c | 6b17011f | CMP w8,w23 |
| 155c190 | b901c68a | STR w10,[x20,#1c4] |
| 155c194 | 5400c060 | B.EQ 155d9a0 |

Independent scan found ZERO direct targets anywhere in [188,198), including entry188 and interior18c/190/194. In particular it scanned194, which the producer initial [184,194) scan excluded. New64-byte guard words188..1c8 are preserved in audit.json. This does not prove absence of computed indirect targets, other builds or runtime patches.

## Incoming state

BIC at188 is the non-flag-setting instruction. CMP18c overwrites NZCV before B.EQ194 consumes it. Thus incoming NZCV is dead. Freshly computed CMP flags must survive to the relocated branch. The actual Skyline context does not save NZCV; this site does not require incoming flag preservation.

There are ZERO integer x16/w16/x17/w17 operands anywhere in the native function155c020..155d9c8. No incoming scratch value is used on the local continuation or return paths. Ordinary native call boundaries retain their ABI assumptions; this review does not recursively inspect all callees. Vector register names are not conflated with integer scratch.

At188 the frame already has native SP-0x110, original LR saved atSP+0x108. The hook uses the established inline context/stack mechanism; compiled callback budget remains a separate lead gate.

## Register meanings and busy path

x20 captures native x0 at155c04c. Native current quantity is LDR w10,[x20,#19c] at130 and SXTW x23,w10 at13c. Native requested quantity w8 comes from menu+0xb40 at12c (or native zero/default path0f4); the scan loop150..17c uses x10/x11/x12/x23/x9 and does not overwrite w8.

At180 native loads old cooldown w9 from[x20,#1c4]; at184 it already computes w10=w9-1. Therefore at188 x9 is NOT the manager pointer. New callback must reacquire a canonical manager; it cannot reuse the old hook assumption that x9 is the manager.

With busy output w9=6,w10=5:
-188 BIC leaves5 (positive sign);190 stores5 to the existing cooldown field.
-18c compares UNCHANGED requestedw8/currentw23. Equal ->194 B.EQ goes directly to common return155d9a0.
-Unequal ->198 CMP w9,#1 sees6;19c B.GT also goes to155d9a0.
-No path reaches the first native quantity commit155c1f4 STR w9,[x20,#19c], or the subsequent creation/removal logic.
-Return155d9a0..155d9c0 only restores native d9/d8, x29/x30 and callee-saved GPRs, addsSP0x110 and RET.

Setting onlyw9=6 at188 would be insufficient: native SUB184 already ran, so w10 must also be5 to preserve the intended cooldown store.

Five is an existing native deferral, not an invented timer: original phase3 CMP16c/B.EQ170 ->155c238 MOV w8,#5 ->23c STR w8,[x20,#1c4] ->240 B155d9a0. The proposal extends that same native cooldown/refusal outcome to a separately qualified managed-busy condition; correctness of that busy predicate is outside this native-placement review. It must not be used to disguise a completion/refusal as a successful count change.

## Remaining integration checks

Lead actual Skyline tail B or LDR/BR/NOP does not change NZCV, but that literal-tail fact alone does not qualify the displaced B.EQ194 relocation. Both near one-word and far four-word hook behavior must preserve the native CMP/branch outcome. Far branch target155d9a0 is outside the overwritten window. Final188 guard, callback w9/w10 writes, unchanged w8/x23, fresh manager acquisition and original regression must match this proposal before activation.

Initial read used the producer relative artifact path under the lobby artifact root and failed because the actual file is under the pkg6 worktree. Explicit rg located it; fresh owned ELF extraction independently supplied the authoritative disassembly. No other error, code edit, native hunt, build, boot, new hook, transport or peer.

## Exact source follow-up: 110fb779cc067dd2143fec9a9c8026a9f454b441

The committed adapter matches the reviewed188 proposal. Full callback read confirms only x9=6/x10=5 are changed on blocked count transitions; requestedw8/currentx23 remain untouched. Canonical manager is freshly read only after BASE!=0 and symbol==BASE+0x52b94f8. Helpers read current manager-owned entry identity and phase, not body cells. The old phase3 scan now remains entirely native.

Independent binary comparison of ALL FOUR64-byte guards passed: new188 e4b556b9768295440a3c6c0f134a244f76cbfc9f578977c6d46bfa1a474e47e5; inputs12c b5fc38915e9e8f48c44e72cfbb4b352e8ab4fb185373c2583b9ebf5e2126cc26; retry238 000019bfa52755110b09146b32300167a4e58da0dfe4a49f949e7f965469ae0d; retained scan16c c0410f8180e8ac06b0cbae4ba765f1e6f1e561cacf803636378f6a4d540b1c2e. All previous guards remain byte-identical; old readiness window is now a relied scan guard. Both legacy comparison and seam gate include every row before installation.

Source receipt /home/intpa/smash-training-lobby/tmp-f6-worker/count-site-review/source-review.json, actual UTC04:52:16.376637. Adapter SHA4918953833e556aa233f3975f956d2ef1074673e77525b00d3f6a973ade180bf; seam SHAcd776981cb08b8d59cca8748e5ea0bb37934370b15a62241666afec5fe3291c0. New original branch-window regression inspected but not executed; Switch-only callback remains uncompiled. No additional blocking source mismatch found within this bounded review; lead conditional relocation/compiled/runtime gates remain.

One attempted audit-script write failed before execution with ParserError: Missing type name after '[' due nested quote escaping. Rewrote regex quote literals as hex22; saved script and bounded verification then passed152ms/90.8MiB. The initial proposal REPORT.md and receipt.json remain unchanged snapshots; this exact-source addendum and source-review.json are subsequent evidence.
