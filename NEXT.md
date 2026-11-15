# Deferred capacity work — next release

2026-11-15. rc.15 retains one shared LINKS = 16 and 12 events per unit for both combos and input drills. The four detailed cells and existing scrolling remain unchanged. The file read bound stays LINKS * 4096 = 65536 bytes. No 64-unit increase or non-HUD hook-copy rework is included in the rc.15 candidate.

The approved HUD root repair remains: HELPER is borrowed under try_lock through pure Renderer::draw; fixed static Changed flags defer native pane writes until after drop(helper). RUN and RENDERER are try-only on that HUD path. The old roughly 11 KB Helper-copy incident is the root cause removed by this repair.

## Measured 64-unit cost

Same-source XPS release snapshots (measurement builds, not acceptance artifacts):

| Snapshot | Commit | NRO SHA256 |
| --- | --- | --- |
| Copy 64 | 194e849df1d36abe861716d6c0d8ec6e8342cf0c | b2402176c8c342905e398259fd0178a3abe62943b47d1cc8d55e4607d7bf2178 |
| Borrow 64 | e17b7395d5540092b9014286bb43cd2389a7c039 | a4fddc110931c26bb6810ed181d04c5dde69c90acb894d83aaa2d7e10e85969c |
| Borrow 16 | 714eee9c6860fd5f6ce9b28fd77615f1a7a5d642 | 4682d81f448f1020992bca1a3953e57a18f4ab6ba46f7ad2336b541779143752 |

16 to 64: NRO .data +126976 B, .bss +0, read-only +12288 B: total static/resident **+139264 B (136 KiB)**. Raw ELF writable-static growth +123840 B is exactly HELPER +40512, INPUT_RESULT +35712, SAMPLE +17856, INPUT_STATE +11904, STAGED +11904 and STRIP +5952. Raw ELF .rodata +11904 B; section padding accounts for the NRO totals. This covers all six capacity-dependent static copies rather than only Helper.

At 64, release stack frames before non-HUD copy rework: Helper::on_link 45120 B, Helper::step 23456 B, Helper::on_clear and set_target 7984 B. HUD repair comparison: hud_draw::draw **59776 to 7968 B**, -51808 B; direct-call lower-bound depth 65488 to 13680 B. These are static disassembly results, exclude opaque/native frames, and do not prove runtime headroom.

Raw section/symbol/disassembly receipts and ELF hashes are preserved under artifacts/rc15-input-drill/measure-{copy64,borrow64,borrow16}-xps in the shared workspace, with MEMORY-REVIEW.md and HUD-LOCK-REVIEW.md. A pre-rollback patch is retained as artifacts/rc15-input-drill/pre-16-ruling.patch.

## Rework plan, deliberately deferred

1. Reset the existing Attempt/InputCapture arrays in place; install targets by reference; avoid constructing large Option/Combo/Attempt temporaries on fighter hooks. Preserve one matcher, sequence junction rules and every hit-file behavior.
2. Add bounded, allocation-free in-place transfer callbacks to the existing fixed publication buffers for staged targets, capture results and HUD samples. Preserve their atomic checks/order; pin a reader only through bounded copy/reset and release it on all exits. No blocking lock or callback reentry.
3. Validate existing concurrency cases and panic-safe reader release, seeded lock inversions/ownership violations, clear/target/sequence behavior, full-capacity grading/file round trips and refusal without partial save.
4. Build matching release 16/64 artifacts. Measure every static section/symbol and every changed hook frame again. The deferred draft passed 301 host-library tests, 12 hook-lock tests, 19 lock-order tests, and hos-lock 23 unit plus 5 lock tests, but had **no target release verification**. Do not treat those host results as target acceptance.
5. Reconsider capacity only with an explicit next-release decision. Restore any larger-capacity progress indicator only if reachable and requested; keep the four detailed cells. Then run the full gate and runtime acceptance under the next release's budget.
