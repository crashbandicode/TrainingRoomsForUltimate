# HB1 render routes — bounded static study (worker C, 2026-11-04)

Decision HB1 (a) (`dcbb2d1`) draws hurtboxes yellow; the operator asked for an
isolated visualization sphere ("option A") by default. `OPTION-A-DESIGN.md`
(stream/slim-phase3-rc6, 2026-11-03) found **no safe route within its scope**
— not impossibility. A-vs-B goes back to the operator; nothing here picks one.

## The one interface (already in `p23/hb1`)

`model::hitbox::HurtboxReader` (the HitModule reads) and
`reconcile_frame(..., hurt: Option<&mut dyn HurtboxReader>, fx: &mut impl
Effects)`: a route is an `Effects` implementation (request/alive/kill per
sphere) plus the runtime wiring `Some(GameHurt { .. })` at the dispatch. The
runtime passes `None`: no hurt sphere. The diag build already runs the reader
once per fighter per frame as the `[Hitbox] hurt census` (parts, first pass,
read cost), over the three relied `hurtbox_*` rows restored in `sites_hitbox`;
the shipped build reads no HitModule.

## Route A — a separate visualization renderer or effect system

- **Proven (static, owned ELF `0789f0a4…`):** the lower system constructor
  `0xa3d30` allocates per-system storage (descriptor fields `+0x20/+0x24` →
  `system+0x70/+0x6c`, allocations `0xa41b4–0xa42a8`); `req_follow` dispatches
  through the shared singleton `B+0x5334ad8`, so today's path is not isolated.
- **Not proven:** safe GPU/device/allocator ownership, draw submission, frame
  update order, shader/texture registration, teardown, and bone-follow for a
  second system; no authored sphere resource exists in the available
  `ef_common.eff`; a new bank or name is not a private pool (the name map is
  global; emitters come from the system-wide free count).
- **One trace boot settles:** `R-pool` (`0xa4f60`: system counts and free
  emitter count across two different resources) and `C-follow` (`0x44f924`)
  would qualify CPU-side sharing; the visual contract (geometry, alpha, tint,
  radius on real bones) still needs an authored resource and an Eden
  screenshot row — a trace cannot substitute for it.

## Route B — the fork's shared-pool `sys_shield` route

- **Proven (Eden, stream `hitbox`):** the persistent-sphere HandleSet design
  kept hitboxes, grab boxes and hurtboxes visible with `failed=0` (hb-03:
  Steve 19 / Luigi 23 spheres; shield bubble alone while shielding; KO effects
  with 54 held); idle pass 225–231 ticks (~12 µs) on hb-03's diag build.
- **Known limits:** the pool is the game's manager-wide free list
  (`M+0x19460/+0x19468`, 56→55→54 in ly-01): Ridley's first pass wanted 55
  spheres and 2 were refused (hb-04), recovering only when other effects end;
  while On, script effects are slept by design, and refusals land on whatever
  else asks the pool; Ridley's part census comes from the diag census line
  (`[Hitbox] hurt census`, per fighter kind) without a trace boot.
- **One trace boot settles (fallback):** the Ridley part census at the natural
  part-update moment `main+0x469e50` (x19 = executing HitModule, 128 B; per
  the lr02 disposition), when the census line is absent or contradicts hb-04.

## Done since this study was written (C2, 2026-11-04)

The three relied `hurtbox_*` guard rows (`hurtbox_module` `0x203db10`,
`hurtbox_groups` `0x469af0`, `hurtbox_part_shape` `0x468490`) are back in
`sites_hitbox`, byte-identical to before the RC7 removal (taken out by
`cab6603`; restored from its parent), so boot blocks the feature if the
game's bytes differ. The want scratch is sized for hit + hurt spheres
together (`WANT_SCRATCH`: 8 slots × 16 + 64 parts × 16), overflow counted in
`Counts::dropped`; the wiring stays open, the dispatch still passes `None`.
