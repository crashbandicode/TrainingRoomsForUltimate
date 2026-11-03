# Count promotion diagnostic boundary review

The existing count188 and completion callbacks keep their native predicates. Both new hook observers now return immediately unless `runtime::diag_log::async_writer_ready()` is true, before touching the changed-tuple mutex, consuming a budget, formatting, or calling diagnostic. Main620d3e8 already owns this API (Slim23ec705); this old worker branch does not duplicate its shared logger. Native qualification therefore applies to the exact composed main candidate. Boot registration remains the existing boot logging path.

Main logger WRITER has one store, true/Release, and readiness loads Acquire. A true observation stays true through diagnostic: its pre-writer synchronous drain branch cannot run from these observers. Observations still format after source and limiter guards drop. Pure 0654cf0 three-test evidence remains valid for unchanged limiter/dispatch code; these two Switch-only guards are not host runtime evidence.

## Conservative line bound

Receipt and expanded upper strings: `artifacts/trainingrooms-nro-archive/pkg6/count-format-bound/`; generator `/tmp/pkg6-count-format-bound.py` (retained in handoff). This is a static derived-Debug schema bound, not a native formatter execution. Every unsigned field, even u8/u32, is charged twenty decimal digits (target usize64). All field/type/punctuation strings follow the actual derived Debug structures. Only fixed ASCII outcome and body-field literals are allowed; the generator asserts their maximum source lengths. SourceToken includes all four fields; PromotionToken includes its full SourceToken again; each NativeIdentity includes all three body pairs. Count has both helper and both source/promotion tuples. Completion compares both expected and maximum actual Ok(identity)/Err(failure) shapes, including full EntryState.

| Record | Conservative bytes |
| --- | ---: |
| count Observation | 1273 |
| completion PromotionObservation | 1405 |
| timestamp + sequence prefix | 43 |
| cut marker + newline reserve | 10 |
| maximum required ring room | 1458 |
| existing diag LINE_MAX | 12288 |

StackLine512 is a first formatting attempt, not a512-byte line cap: on overflow `diagnostic` passes original Arguments to ByteRing::push, which formats directly into the ring. No larger stack buffer or allocation is introduced. Available ring room can still be smaller than LINE_MAX; dropped/truncated counters, `[cut]`, missing record endpoints and observer budget exhaustion must remain explicit unknown/failure during runtime review. The bound proves no maximum-line truncation with sufficient ring room; it does not claim capacity or callback-stack acceptance. Lead must inspect compiled callbacks/Debug frames and fresh measured margins before runtime admission.
