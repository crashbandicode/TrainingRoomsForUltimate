# Measured stack policy review after sh-01

Bounded read-only review assigned by the lead at 05:45Z, 2026-11-03.
No allocation, reserve, source, or runtime changes were made.

The two policy failures predate r13. They are missing measured-table evidence,
with a small additional Heartbeat frame increase, rather than evidence of a
new large buffer or a paint-gap artifact.

| Evidence | Heartbeat maximum | MiiCatalog maximum |
| --- | ---: | ---: |
| f6-31, r10 | 10944 | 8016 |
| f6-37, r13 | 11184 | 8016 |
| sh-01, r13 | 11056 | 8016 |
| Existing measured table | 9072 | 0 |
| Existing allocation | 28672 | 24576 |
| Minimum under unchanged policy | **32768** | **28672** |

The formula remains page-rounded `max(static + external, measured) * 1.25 +
16384`, using integer quarter division. Heartbeat needs 30364 before page
rounding at 11184; MiiCatalog needs 26404 at 8016. Even r10's 10944 already
required 32768. Preserve f6-37 record #767's larger 11184 rather than replacing
it with sh-01's smaller maximum. `low=false` only checks the 16 KiB reserve;
it does not check the additional quarter margin.

## Compiled and source comparison

Both binaries were parsed with the corrected context-sensitive scanner.

| Root | r10 entry / frame / direct depth | r13 entry / frame / direct depth |
| --- | --- | --- |
| Heartbeat detached closure | `1d1b4` / 2656 / 5056 | `1dda0` / 2768 / 5168 |
| MiiCatalog detached closure | `1ba98` / 448 / 5872 | `1c684` / 448 / 5872 |

Heartbeat's first measured mark grows by exactly the same 112 bytes as its
entry frame: 10944→11056. The two added developer hook counters increase the
counter arrays from 44 to 46 entries; the compiled initial Snapshot clear
grows from 712 to 744 bytes, and the loop carries several snapshot/observation
copies. The compiled prologue grows from `0x60 + 0xa00` to `0x60 + 0xa70`.
The drain loop itself is unchanged; the only diag_log source addition is the
monotonic `async_writer_ready` accessor. This supports bounded counter-frame
growth, not a new 12 KiB stack copy: the drain buffer remains static.

MiiCatalog's replay_store source is byte-identical between r10/r13, as is the
measurement/stack-management source. Its entry frame and complete modeled
direct depth are unchanged. The three observed runs have exactly the same
8016-byte mark. Its 5872 direct path includes read_saved_mii_catalog, sorting
and panic branches; that path is a static lower bound, not the observed call
stack. Receiver waits, filesystem/native calls and SDK entry code are outside
complete direct-path accounting.

One **known indirect Heartbeat edge** also needs explicit accounting:
boot.rs registers `seam_install::report_late` in TICK. The r13 closure's
`blr x8` at `1de5c` runs with its 2768-byte frame. `report_late` at `b2b44`
has frame 832 and modeled direct depth 2928, giving an owned lower-bound
path of **5696**. The generic scanner's 5168 omits this edge. A documented
static bound of at least 5696 includes it; measured 11184 still determines
the allocation. This does not make indirect/SDK coverage complete.

## Painting semantics and limits

The actual queried regions are thread-stack kind `0xb`, with sizes matching
the table allocations. They are not the ambiguous larger heap regions seen
for some UI fibers. `threads::diag::start` saves SP in x21, logs it, then at
`e4e68` subtracts exactly `0x400` and calls the leaf paint loop at `e4e70`.
It paints every word from the region base to that limit; scans only read.
The stack registry's reader count protects a slot during exit, and these two
detached threads use separate permanent slots rather than the shared boot stack.

Consequently the deliberately unpainted initial floor is `top - loggedSP +
1024`: Heartbeat 4208 (4096 in r10), MiiCatalog 1888. Both are far below the
observed maxima. The unchanged gap cannot produce the reported 11056/8016
by itself. Reports include normal diagnostic/measurement execution, which is
real stack use of this diagnostic binary and must remain in its budget.

The paint records do **not** contain a PC, saved call chain, or attribution
of the deepest write. They cannot establish precisely which ordinary SDK,
filesystem, wait, or diagnostic call set the mark, nor conclusively rule out
all unrelated corruption. Repeated identical MiiCatalog marks and the exact
Heartbeat frame delta support real existing execution depth. No captured
evidence supports discarding either mark or changing the margin policy.

## Other retained measurements

These measured metadata increases also need preservation; their allocations
remain adequate under the greater of the retained static and measured bounds:

| Thread | Old measured | Reviewed maximum | Existing allocation / required |
| --- | ---: | ---: | ---: |
| SettingsLoad | 9488 | 9616 | 28672 / 28672 |
| Menu | 13488 | 27072 | 57344 / 57344 |
| Archive | 7712 | 7840 | 28672 / 28672 |
| ReplaySave | 24400 | 25248 | 57344 / 57344 |

All raw sh-01, f6-31 and f6-37 stack records are retained, including lower
observations and boot/shared-stack sizes. No historical maximum is reduced.
The lead owns any table/allocation correction, targeted gates, coherent native
build, and subsequent live verification. This review grants no runtime or RC PASS.

Evidence: `artifacts/trainingrooms-nro-archive/slim/sh01-stack-review/` under
the shared project parent. Its receipt binds both ELF/assembly identities,
four source comparisons, addressed root/paint assembly, raw log hashes,
per-run maxima, paint floors and all 17 policy rows. The reviewed scanner is
the same composed tool already tested in the earlier scanner-root receipt.
