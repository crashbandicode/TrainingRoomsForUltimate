# F6 a5020 demand-observer relocated-window review

**PASS for the bounded native displaced-window and existing helper/caller liveness scope.** No conflict found. This is not callback, compiled stack, integration, or runtime acceptance. The unqualified global93660 observer remains excluded. Lead assignment2026-11-03T06:28:21Z allowed15minutes; no product source, hook, compiler, host claim or Eden change was made.

## Inputs and reproducibility

Native13.0.5 ELF SHA2560789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2, /home/intpa/trainingrooms-build-pkg6-host/70d4927/research/game-13.0.5/main-13.0.5.elf.
Loaded Skyline NSO9959ecb13eab6db81a07cee04a789b1905fd605409fc84efeb7ec43da7e29280 was independently rehashed and re-decompressed into owned scratch. All three embedded segment SHA256 checks passed; reconstructed ELF9016cb82b8d129c6422e525eaa27a2587a7101adedb48f2a37d8e17c875ace27 exactly matches lead's retained artifact. This is actual shipped implementation evidence, not upstream source ABI inference.

Raw directory /home/intpa/smash-training-lobby/tmp-f6-worker/helper-demand-abi; ranges.json binds fresh LLVM21 disassembly. Prior /tmp-f6-worker/hitbox-abi/receipt.json's16files were rehashed unchanged, including whole helper/constructor and both known callers. No new tool installation/transport; owned LLVM fallback.

## Exact site and incoming paths

Four words at aligneda5020 (mod8=0):
`12800008 f900003f d00210e0 913e5400`
= MOV W8,-1; STR XZR,[X1]; ADRP X0,42c3000; ADD X0,X0,f95.

Full64-byte guard:
`080080123f0000f9e01002d000543e91280800b926000014187d4093084e8052155f089b150400b4350000f9687e40b909050011697e00b9280800b9e00315aa`.

Descriptor patha4fc0 loads W8 demand,a4fc4 loads W9 available from manager+80,a4fc8 CMP,a4fcc B.GT a5020. Fallbacka5010 sets W8=0,a5014 loads W9,a5018 CMP,a501c B.LE a4fd0; fallthrough therefore also signedW8>W9. No flag-setting instruction intervenes. Both are signed32 comparisons; negative values must remain signed and marker absence cannot identify another failure.

Whole executable B/BL/B.cond/CB/TB scan found only explicita4fcc→a5020, no target insidea5024..a502c. Separate executable ADR and LDR/PRFM literal-target scan found none into16-byte window; allocated-section exact64-bit pointer-literal scan found none for its four instruction addresses. These scans do not exhaust arbitrary computed targets/ADRP+ADD/MOVZ-MOVK dataflow; no expanded whole-program claim.

## Actual Skyline installation and relocation

A64InlineHook4500 invokes PLT103c4 at45e8. Stub103c4..103d0 uses GOT17588; dynamic relocation1026 resolves to defined A64HookFunction43d8, which calls40f4 at448c. Fresh skyline-yoga-inspection.json binds this symbol/relocation chain and context relocations.

Installer40f4 compares absolute((destination-source)>>2) against1fffffe at4138..4150:
- Near branch42d0 relocatesONE word via3bcc at4350 and patches nativeB at436c..4380. OnlyMOVW8 is relocated; returna5024 runs nativeSTR,ADRP,ADD,STRW8.
- Far branch sets4words at418c because(site+8)&7=0. Calls3bcc at4228, installs LDRX17/BRX17 plus destination literal4254..4284. No fifth displaced instruction at this alignment. Four-word continuation isa5030.
- Relocation-loop return tail3d94..3e88 emits either a directB or optionalNOP+LDRX17/BRX17+absolute continuation. These do not alter X0/X1/W8 or flags; X17 liveness is addressed below.

The relocation loop tries classifiers2a60,2e60,3218,366c; unmatched words copied by3d14..3d30. MOV12800008,STRf900003f,ADD913e5400 match none and are copied exactly. ADRPd00210e0 matches (word&9f000000)==90000000 and reaches394c. These mask checks were repeated against actual words in relocation-arithmetic.json.

At397c..39ac it reconstructs original source-page plus immediate-page delta. ADRP imm21=421e, delta421e000, page(a5028)=a5000, result42c3000 relative to native text base. The actual implementation's32-bit intermediate is nonnegative/in-range for THIS instruction; no general negative-ADRP correctness claim.

At39f0 it checks whether target page lies in original relocated interval via2780; interval initialized3be8..3c00 is[source,source+4*count). Nativebase+42c3000 lies outside[base+a5020,base+a5030), so branch3a00 always selects external-target3a9c.

External expansion3a9c..3b7c emits:
- optional NOP when relocated literal would otherwise be unaligned;
- `58000040` = LDR X0,[PC+8];
- `14000003` = B PC+12, skipping the following8literalbytes;
- QUAD nativebase+42c3000.

Following originalADD X0,X0,f95 is copied and produces exactly nativebase+42c3f95. This path does not depend on ADRP page-distance from the trampoline; both nearby/far trampoline placements use the absolute literal when the four-word path is selected. Near hook displacement of onlyMOV leaves ADRP at originalPC.

Eight independent arithmetic cases cover two page-aligned ASLR bases, trampolinePC mod8=0/4, nearby/far placements; decode emitted LDR/B words, verify aligned literal, skipped data and finalX0. Wrongly copying originalADRP at relocatedPC is a negative control and yields the wrong page in everycase. This is arithmetic verification of inspected emission, NOT execution of Skyline or the callback. Initial manual expectedimm constant4222 was wrong; assertion failed before receipt generation, corrected to independently decoded421e and reran. No failed product/runtime test was hidden.

## Preserved native behavior and liveness

On both installed paths, MOVW8 gives X8=00000000ffffffff, STRXZR writes64-bit0 at[X1], and X0 becomes formatbase+42c3f95. Originala5030 writes32-bitffffffff at[X1+8], a5034 branchesa50cc. OriginalBL93660 runs the unchanged no-op varargs-save stub, thena50d0 setsW0=0 and epilogue returns false. Stores occur in their original order, and no original register argument/result is replaced by the literal expansion. Callback must leave saved context unmodified; Slim owns that source gate.

Actual contextd4a4..d5d4 does not preserve incomingNZCV or x16/x17 (d5c8/d5cc replace scratch before save, d538 callbackBLR, d5c0 return usesX16). This is acceptable here only because:
- All instructions froma5020 through unchanged93660 and false epilogue consume neither oldNZCV nor integerX16/X17.
- Full helper/wrappera4f60..a51b4 has no integer16/17 operand.
- Previously qualified wrappera51a0 has two discovered direct callers356d2dc and356ffa4. The false result at356d2e8 TBZ goes to356db48, thencommon epilogue356cfa0; no oldflags/scratch use. Both constructor callers453210 and3563760 retain prior addressed proof.
- Second helper caller356ffa8 TBZ on false reaches357001c SUBS before flag consumer3570024; full continuation has no integer16/17 operand. Prior report covers branches, loops and return35706e4.
- Caller correlation cannot waive the second native call path; both remain covered within prior directcaller scope. No blanket computed-call absence or recursivecallee proof is added.

At entry W8/W9 are original signed values, X19 manager, X1 constructorobject+1d0 for the correlated invocation, nativeLR356d2e0, SPconstructor-40. The MOV has not overwritten W8 yet. New marker proves only emitter-demand rejection. Existing helper-return capture has its own qualified position beforea50d8 registerrestore; absence of this new marker leaves other failure causes unresolved.

## Acceptance limits

The specific relocation and known native liveness gate is closed with raw evidence above. Slim/lead still owe seven guard/coexistence composition, nonmutating bounded callback source/tests, actual compiled callback/frame qualification, integration identity and runtime observation on a corrected candidate. No newhook was installed, no global93660 approval, no root capacity explanation from static possibility, no cap/retry/readiness change. Both matched F6 runtime lanes remain held pending corrected coherent qualification.
