# ui-16 probe stack fault: addressed evidence

Lead integration, 2026-11-03 02:0xZ: the investigation below is retained from
Slim `e6e04d1`. The table now records the owned 3440-byte bound separately from
143376 external bytes and allocates 200704 bytes using the unchanged reserve
policy. Rust and ELF-report checks include both contributions. The original
24 KiB allocation fails the new external-path regression; a growing external
path fails again. Six Python scanner tests pass; targeted Rust, the changed
native build, and the same-loader runtime reproduction remain pending.
The raw receipt SHA-256 is
`bd648a5b2e765c77e27d450495acea03493a3ba30a49f8b1a47b12caf6e7c9bd`;
the lead independently verified all 16 listed assembly hashes.

Investigation authorized by the lead at 2026-11-03 01:24Z, bounded until 02:09Z, and closed after the lead's 01:55Z contract approval. Read-only inspection of the deployed standalone ARCropolis binary was explicitly authorized at 01:27Z. No upstream/fork source or TrainingModpack binary was read. Slim made no product edit, stack increase, live control or probe deletion; the lead owns the shared-table correction.

## Identities and raw evidence

- Frozen TrainingRooms source `e1d81d78215bf975ca0e67e1707dae0a46279508`, diagnostic NRO `331da18e8748fdbd6a5fdb48acd225e9ffa4675638eb8646f6d06dca2a286347`; ELF `db3380f5ce38a7664a01d803b9e333273f0fae3b47d9463d830ae95eda1fb0c6` freshly checked.
- Deployed loader SHA-256 `92171b2f4dbd7afe9361a55922f01de7f46ffae09f05185c1cea4ab79dba9a1c` matches retained `artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/c1162/ally-partb/payload/plugins/libarcropolis.nro`.
- Early console snapshot SHA-256 `3220b9ff89d78fbea5d05b352b69af84cd6995c123ce1ef9ef064d39fa185322`, 309283 bytes, preserved under `artifacts/trainingrooms-nro-archive/slim/ui-16/early-console.log`. This is an early snapshot, not final health or restoration evidence.
- Same directory contains `stack-receipt.json`, `probe-entry.asm`, `file-open.asm`, `arc-1496d0.asm`, `arc-20008.asm`, `arc-57f24.asm`, and `arc-open-candidate.asm`.
- Existing llvm-objcopy wrapped the raw NRO as an ELF data section only for disassembly. This is **not** the original loader ELF or successful REA analysis. NRO text begins at file/virtual offset zero, length `0x1af000`; addressed observations below use NRO-relative addresses. No code was reconstructed for use in TrainingRooms.

## Exact frame match

The console records `tr-probe` stack `[0x109afa0000, 0x109afa6000)`, size 24576. Its first unmapped write is `0x109af82f88 = 0x109afa3038`. The exception later reports R27 `0x109afa3010` and SP `0x109af82dd0`.

Loader function `+0x149430` saves 0x60 bytes, then subtracts `0x20000` and `0x2e0`: **131904 bytes in this frame alone**. Let `S = 0x109af82f88 - 0x158 = 0x109af82e30`, the frame's post-prologue SP.

| Binary operation | Prediction | Actual evidence |
| --- | --- | --- |
| Entry SP is S + 0x20340 | `0x109afa3170` | Inside registered probe stack |
| `+0x1494c0` stores x8 at S+0x158; x8 is S+0x20208 | address `0x109af82f88`, value `0x109afa3038` | First unmapped write matches both |
| `+0x1494a4/+0x1494b0` sets x27=S+0x201e0 | `0x109afa3010` | Exception R27 matches |
| `+0x1496cc` calls through x23, return `+0x1496d0`; `+0x20008` saves LR at S−0x60 | address `0x109af82dd0`, value `0x2772fef6d0` | Unmapped write matches both |
| `+0x2006c` indirect call returns at `+0x20070` | `0x2772ec6070` | Subsequent saved return value matches |

Module base `0x2772ea6000` is supported by the first NRO-load record immediately following the ARCropolis read, its registered hook targets, and these exact instruction/value matches. An independent UI-owned module map was requested; no second controller was started. The match establishes the insufficient caller-stack contract for this actual external frame, beyond mere address proximity.

The 18560-byte paint high-water report is not a bound on this execution: the 128 KiB subtraction jumps below the painted allocation. `threads.rs::untouched` scans only inside the registered allocation and therefore cannot see these out-of-allocation stores.

## Caller and correction boundary

Frozen probe entry is `+0x1d5a0`. It calls the external size function at `+0x1d738`, load function at `+0x1d820`, Switch File::open at `+0x1da20`, read at `+0x1db48`, and read_to_end at `+0x1dbb0`. The last observed probe rows are mounted, first stage texture size, successful arcrop_load_file, and successful mods:/ read. No corresponding arc:/ result follows. Diagnostic draining means console print order alone does not timestamp individual calls.

Loader dynsym identifies arcrop_load_file at `+0x2a3c0` and size at `+0x28110`. The only direct BL to the large frame is `+0x57f24`, within `+0x574c0`; that caller itself reserves `0x2880` bytes. Its direct callers include `+0x5a828` and `+0x112a48`. The latter is within `+0x11275c` (frame 0xf0), called at `+0x111084` and `+0x113094`. Exact filesystem ABI registration and the remaining nested-call bound are still being qualified. Do not substitute plausible function names for these addressed observations.

The current Probe table covers a 3440-byte plugin-only static lower bound, measured zero, and the common 16 KiB SDK reserve. That reserve cannot cover this known 131904-byte external frame. The lowest observed write address is 144000 bytes below the registered top. Applying the existing quarter-margin plus 16 KiB reserve to that **observed lower bound** rounds to 192 KiB; this is not yet a sufficient full-callgraph bound or an approved allocation. Further nested calls and the exact filesystem path must be accounted for before proposing the final worker contract.

The probe intentionally compares arcrop_load_file, mods:/, and arc:/, including ranged reads. Replacing one route with another would change its assertion. A root correction must preserve those observations and receive a fresh exact-binary run with zero total Unmapped, complete probe rows, and adequate stack reserve. Final RC acceptance additionally requires all hook allocation deltas zero. ui-16 remains a failed development run.

## Registered read ABI and nested calls (01:51Z checkpoint)

The UI owner confirmed that its existing control schema has no guest/module/thread/memory inspection method. There is no independent named module map or saved-stack inspection available; no new debugger is requested. The exact module attribution above rests on the load record, registered hook addresses, and multiple instruction/address/value matches, not on an invented independent map.

The loader's object factory at `+0x5bc0c..0x5bc64` installs C++ vtable `+0x1f7288`, a payload at object+0x10, and Rust vtable `+0x1f0668` at object+0x18. The C++ read slot (+0x10) resolves to `+0x13e5f8`, whose referenced diagnostic string is `FAccessor::read`. That adapter loads the payload/vtable pair and calls the Rust read slot (+0x18) at `+0x13e644`. Its relocation target `+0x1f0680` resolves to `+0x5a554`, containing the direct call `+0x5a828` to `+0x574c0`. That function calls the large frame at `+0x57f24`. This qualifies an actually installed filesystem read ABI, rather than a guessed function name. The exact virtual mount and live parent return were not independently captured.

Below the large frame, `+0x20008` reserves 0x60 and calls the read vtable at +0x18. Vtable `+0x1f8b18` resolves that slot to `+0x16bd3c`, which reserves 0x50. Its buffered copy calls PLT `+0x1adf70` (`memcpy`): GOT `+0x1fab10` plus module base equals exception R16 **0x27730a0b10**. Its alternative read branches call PLT `+0x1adfc0`, whose relocation is `nn::fs::ReadFile(unsigned long*, FileHandle, long, void*, unsigned long)`. The nested saved LR at S−0xb0 is consequently `0x109af82d80 = 0x2772ec6070`, matching the raw fault.

The normal decoder branch has a deeper known nested path than the 176-byte buffered-read frames: `+0x14d624` (0x130), `+0x148254` (0xe0), `+0x1486d0` (0x30), `+0x1479ac` (0x40), `+0x147a34` (0x50), totaling **720 bytes** before the allocator/SDK boundary. The complete `+0x14d624` body extends to `+0x14e5a8`; its early return at `+0x14da20` is not the end of the function. Its later direct callees at `+0x14e5a8/+0x14e630` use only 0x20 each. Bit-reader `+0x147e28` uses 0x40 and recurses at most once: it recurses only when requested bits exceed the current sum, then passes that exact sum unchanged to the inner call, which takes the equality branch. This is an addressed control-flow observation, not an unbounded recursion assumption.

Accounting for the qualified read route is 3440 plugin bytes + 64 adapter + 320 Rust read + 10368 parent + 131904 large frame + 720 nested path = **146816 bytes**, then the existing quarter margin (36704) and SDK reserve (16384), rounded to **200704 bytes (196 KiB)**. At 01:55Z the lead independently reviewed the raw assembly, relocations and exact fault matches and approved this worker contract correction. It is not an absolute whole-loader bound: indirect error/destructor paths and SDK internals remain covered by the existing reserve policy rather than fully enumerated here. The observed live prefix is 11920 bytes before the large frame, less than this conservative 14192-byte prefix. The original 192 KiB arithmetic from the lowest observed write alone is insufficient for this longer known branch. Slim has not changed product code or the shared thread table; the lead will compose the correction with its exact Heartbeat static metadata update.

Preserve the owned ELF's 3440-byte scanner result separately from the 143376-byte qualified external contribution. A larger allocation without the external contract would leave the original verification blind spot. The fresh composed candidate still requires the same deployed loader and fixture, all three probe routes and ranged assertions, zero total Unmapped, and an in-allocation watermark with the existing SDK reserve free. No successful rerun is claimed here. `binary-receipt.json` hashes all retained assembly and `abi-relocations.json`; the bounded investigation is complete, while runtime acceptance remains open.


## ui-17 completed-path measurement (2026-11-03)
Exact r10 NRO7af7fb32 with the same ARC92171b2f and B3 fixture completed every Probe path, including final EFFN ranges. Initial console SHA2566f4d597b192077824843bd03565fc0758a64b0ad7d6e4cbf275de79f77ae2d8e records exit used151920/size200704/free48784/low=false and zero Unmapped. This completed-path evidence supersedes the missing measurement, but low=false only covers the SDK reserve. The unchanged policy requires max(3440+143376,151920) +25% +16384, rounded to4096 =208896 (204KiB). Main records measured151920 and sets204KiB; owned/external static bounds remain separate. Original196KiB runtime result fails the full margin policy and remains retained. New source/compiled stack gate and fresh actual runtime measurement remain pending; initial-console health is not full-session UI acceptance.
