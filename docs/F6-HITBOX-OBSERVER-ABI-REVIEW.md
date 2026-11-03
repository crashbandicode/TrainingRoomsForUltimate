# F6 Hitbox observer incoming-state audit

Completed 2026-11-03, bounded 15-minute assignment acknowledged 04:16:28Z (deadline 04:31:28Z). Scope: incoming NZCV and x16/x17 at 0x356cec0, 0xa50d8, 0x356d970 and 0x356cfb8. No implementation, hook, build, boot, transport or peer was added.

## Result

The four sites pass this bounded incoming-state liveness review for the owned native functions and all discovered direct callers. This is not blanket Skyline ABI or observer activation acceptance. Relocation/RET classification, actual callback stack, source integration and runtime acceptance remain lead-owned. Incoming flags need not survive at these sites; flags produced by displaced CMP at 0x356d974 MUST survive through displaced CSET at 0x356d978.

Owned game ELF SHA256: 0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2. The helper/constructor are exactly the efe3643 observer contract ranges. Fresh disassembly and its hashes are in ranges.json, ranges-2.json and ranges-3.json. All artifacts are in /home/intpa/smash-training-lobby/tmp-f6-worker/hitbox-abi; receipt.json binds every final input/output.

Read actual lead Skyline context disassembly d4a4..d5d0. d538 BLR invokes the callback; no MRS/MSR NZCV surrounds it. d5c0 overwrites x16 and d5c4 branches through it. Stub d5c8 sets x17. GPR/vector saves therefore do not prove preservation of original x16/x17 or NZCV. Lead supplied artifact identity: Yoga subsdk9 SHA256 9959ecb13eab6db81a07cee04a789b1905fd605409fc84efeb7ec43da7e29280. I did not independently repeat NSO decompression.

## Addressed proofs

| Hook | NZCV | x16/x17 |
| --- | --- | --- |
| 356cec0 entry | Straight-line prologue reaches CMP w5,w8 at 356cef4 before first conditional flag use B.LT at 356cef8. | Whole constructor contains no x16/x17 occurrence until defining ADD x16,x9,x10 at 356d9a8 and LDR x17,[x12,#98] at 356d9b8. All scratch uses belong to that locally initialized loop; early returns do not read them. |
| a50d8 helper epilogue | Three pair loads and RET at a50e4 do not use flags. Both direct wrapper callers' continuations are qualified below. | The entire a4f60..a51b4 helper/wrapper has no x16/x17 operand. Epilogue restores x20..x24/x19..x23; x29/x30 were restored at a50d4. Both immediate caller continuations contain no scratch read before a new call/return or locally defining constructor loop. |
| 356d970 phase | LDR q0 at 356d970 does not read flags; CMP w10,#0 at 356d974 overwrites them before CSET w9,ne at 356d978. | Nonempty branch defines x16 at 356d9a8 before 356d9ac use, x17 at 356d9b8 before 356d9bc use. Empty branch at 356d984 goes to 356da54 and uses neither scratch register before common return. No incoming-scratch dependency. |
| 356cfb8 constructor return | Four pair loads, ADD sp at 356cfc8 and RET at 356cfcc have no flag use. Both direct constructor callers are qualified below. | No scratch use in this epilogue. Caller continuations contain none before their next call or return. |

### Shared-helper return paths, including uncorrelated invocation

Executable B/BL scan found only wrapper tail B at a51b0 -> a4f60; two direct callers of wrapper a51a0: 356d2dc and 356ffa4. Callback correlation does not constrain native execution, so both were checked.

- Constructor continuation 356d2e0 MOV w8,w0; 356d2e4 MOV w0,wzr; 356d2e8 TBZ w8,#0 -> 356db48. True path 356d2ec LDR x8,[x22]; 356d2f0 CBZ -> 356db30. Neither branch reads NZCV. Live-success path overwrites flags at 356d2fc CMP w9,w8 before B.NE 356d300. Failure tails 356db30/356db48..356db68 only move/load/store and branch to common epilogue 356cfa0; no flags/scratch use before returning to the qualified constructor callers.
- Second caller 356ffa4 returns to TBZ w0,#0 at 356ffa8. Success calls 8a3e0 at 356ffb8 without consuming incoming flags/scratch; failure reaches 357001c SUBS w8,w8,#1 before B.EQ 3570024. The continuation through its loops and return at 35706e4 contains no x16/x17 operand. Inspected continuation 356fc24..3570710, including success/failure, backward loop, and common return. Calls are normal ABI boundaries, not claims that this review recursively inspected their callees.
- No x16/x17 operands exist in either helper or the inspected second-caller continuation. Vector v16/v17 and q16/q17 are different registers and were not confused with integer scratch registers.

### Constructor return paths, including second direct caller

Executable B/BL scan found TWO constructor callers, not only the observation-correlated manager: 3563760 and 453210.

- Manager 3563764 TBZ w0,#0 chooses failure 35637d0..35637dc or success 3563768..35637cc. Success bit tests and per-bit counters 35637e0..3563a1c use only TBZ/TBNZ and non-flag-setting moves/loads/adds/stores. All paths reach 3563660 MOV x0,x19; 3563664 BL 39c19d0. There is no NZCV consumer or x16/x17 operand between return and that next call.
- Other caller 453214 TBZ w0,#0 chooses failure 453280..45328c or success. Success first overwrites NZCV with CMP w21,#1 at 45321c before B.LT 453220. Failure reaches CMP x19,#0x18000 at 4532d0 before B.NE 4532d4. Whole continuation and its loop 45315c..4532fc contain no x16/x17 operands. Next calls are 4531d8 BL 3570a10 on another iteration or 4532dc BL 39c19d0 on exit.

## Scope and reproduction

The direct-call scan examines every aligned executable B/BL encoding in the exact ELF; call-xrefs.json records its five edges. It does not prove absence of computed indirect callers, relocation-derived targets, another build or another plugin modifying this code. This is addressed native static evidence, not REA evidence or a live execution result. Full constructor review includes 356cec0..356db70, not the older lead disassembly which stopped at 356d8fc.

Commands use local /usr/bin/pwsh -NoLogo -NoProfile -NonInteractive -Command. Fresh extraction and scan jobs used 512 MiB/60-second systemd bounds; scan finished in 3.028 seconds with 89.3 MiB peak. The final tiny tail extractions used direct LLVM commands (each completed under one second). No game process, claim, profile, controller or host queue was acquired.

An initial attempt to derive the scan script through nested quoting failed before execution, exit 1, with: SyntaxError: unterminated string literal (detected at line 1). Replaced it with an explicit saved script and reran successfully; no failed scan is counted as evidence.

Next: lead combines this liveness result with its actual relocation/RET mask proof and callback-stack/native build checks. F6 then requalifies the managed-enabled integrated candidate when exact artifact and admission are assigned; six r10 clean boots do not certify newly enabled paths.
