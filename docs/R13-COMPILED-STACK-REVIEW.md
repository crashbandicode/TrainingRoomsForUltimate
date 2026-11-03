# r13 compiled stack review and scanner limitation

2026-11-03T05:16Z, frozen5d92df4, ELF2080ba7ab3009857c79a90302bf6ef434f6722392ff814b6e94636e3ed4a8dca.
Successful job/source continuity, all1308 source hashes and source archive were
independently checked. Evidence is in the parent project's
artifacts/trainingrooms-nro-archive/slim/r13-stack-review. No new compiler or
product edit; native/runtime acceptance remains with the lead.

The Menu root correction works. Its persistent frame fell10080->6736, while
Worker::step now has its own4272-byte frame and6192 direct depth. The known reload
path is16+6736+6192=12944; whole Menu maximum31280 remains below recorded32432
and fits the existing57344 allocation/policy. Three call traversal orders all
report31280. This establishes frame separation without changing read behavior,
chunk size, queue semantics or stack allocation.

The unmodified scanner reports IniWriter3248>table3216; its allocation28672
remains sufficient because historical measured9344 still dominates policy.
However, this32-byte difference is **not established production frame growth**.
Every function on that selected path has the same frame as r12. The changed
selected tail runs from File::open224938 through cstr224c54(frame112) and
spec_extend1cd194(frame48), instead of the former formatting tail. The existing
scanner memoizes per-node results while cutting recursion according to the
current on-stack set. Those memoized partial results depend on traversal context.

A bounded same-ELF/same-edge/same-frame probe changes only callee iteration order:

| Order | Heartbeat | Menu | INI writer |
| --- | ---: | ---: | ---: |
| Existing integer set | 4992 | 31280 | 3248 |
| Ascending function name | 5168 | 31280 | 3248 |
| Descending function name | 4992 | 31280 | 3072 |

All nine reported paths contain unique function names and retain their exact
edges/frames in order-probe.json. They are direct-path lower-bound candidates,
not complete recursive bounds. Ascending order also exposes Heartbeat5168 above
table5136. Therefore do not chase metadata differences with repeated native
builds or attribute all variation to LTO frame growth. The lead owns the scanner
contract/correction and resulting measured metadata. No arbitrary padding or
allocation increase is proposed. Both known Heartbeat and INI paths remain
covered by their existing28KiB allocations under the unchanged policy.

The raw default table reports all other present rows passing, with PracticeTrace
feature-off. Probe has owned3296<=3440, external143376, measured151920, allocated
208896; the existing204KiB policy still passes. Generic owned-hook budgets have
no failures, subject to their known indirect/recursion limits.

Callback source files and native site tables are byte-identical to r12; detail.json
binds nine file hashes. Actual compiled values were measured again:

- Four Hitbox callbacks retain48-byte frames/192 direct depths, plus Skyline768
  and the previously qualified native prefixes. Known local totals remain
  960/1264/1200/1200, not whole native ancestry.
- count_readiness retains5728/7088; with Skyline768 and native272, known total8128.
- Q15 constructor_context304/2224, plus Skyline768 at native entry.
- Developer replacement48/912, three indirect calls at1b2d94,1b2e2c,1b2e5c;
  its first is the original native call. No inline768 frame applies here.
- completion_note256/2176; it remains beneath the existing capture callbacks.
- UI callback32, tick2928, HUD draw6880/8880. The registered DRAWS symbol remains
  400510 and the actual indirect call is nowa2234. The known pass path is
  32+2928+8880+768+160=12768, below16384 by3616; registration substitutes native48
  for160, totaling12656. Do not use the generic8672 direct-only result as the HUD
  bound. Native setter/SDK/recursive pane walks and live heap/headroom remain open.

R13 is not an all-table PASS: the existing traversal reports the INI failure,
and the order probe exposes another known Heartbeat path above its metadata.
Menu behavior correction passes this review. Final scanner disposition, coherent
gates, zero-allocation/headroom runtime and Hitbox classification remain pending.
