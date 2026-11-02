# Copy-package roster pointer lifetime

Owned 13.0.5 ELF SHA256:
`0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2`.
Complete shell static disassembly retained under
`artifacts/trainingrooms-nro-archive/pkg6/copy-roster-lifetime/`.
This is static evidence, not runtime acceptance or REA Evidence.

- Acquire `main+0x17efb40..0x17eff9c`: the incoming roster x1 is copied to
  x20 at 0x17efb58. Reads use its eight 0x80-byte records and kind/costume fields.
  The only call/tail target is 0x17effa0, receiving G68 in x0 and scalar
  costume/kind in w1/w2. The roster pointer is neither stored nor forwarded.
  Tail exit 0x17eff8c restores callee-saved x20; the other exit returns at 0x17eff9c.
- Release `main+0x17f0290..0x17f069c`: x20 receives x1 at 0x17f02c0;
  x24 addresses records during the Kirby loop, ending at offset 0x400.
  The only call/tail target is 0x17e2fe0, receiving G68 and the constructed key.
  No roster pointer is stored or forwarded. The exits return at 0x17f067c or
  tail-call at 0x17f069c after restoring callee-saved registers.

Thus each call may use a call-local 1024-byte native-layout buffer. The retained
`Acquisition` value contains all roster fields, member and costume needed to
reconstruct the identical release arguments. Acquisition-before-release order
and lock-free native calls remain unchanged. The compiled stack gate is still
required; this finding does not authorize enlarging any stack budget.

REA CLI 6.4.0 was attempted once under 1 GiB/60 s and returned
`{"code":"UNKNOWN","message":"write EPIPE"}`. No REA result was obtained.
Existing llvm-objdump produced the complete listings; initial shorter release
ranges were extended through its final tail exit before drawing this conclusion.
