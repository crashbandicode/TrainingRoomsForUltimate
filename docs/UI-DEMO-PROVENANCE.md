# Waiting-menu demonstration provenance

The README clip is actual Eden footage recorded on 2026-11-03, run `ui21-02`
on the owned Yoga lane. It shows Solo Steve waiting, B+Up opening the
TrainingRooms menu, then R selecting the Developer page. No simulated UI,
redrawn frame, speed change, crop, or intermediate cut is used.

- Compiled source: `758cee509531c9a2d8eabb3c8663cca8c9fda649` (r16).
- NRO SHA-256: `88abde8e01a5e4be11c6e31a7e25a302a7cd1daffda175d424a6c7ddc5769bf6`.
- Original `waiting-menu.gif`: `a41ab3dc710bcb1171bbce958c07938e660a21c366e64ee21a616b6358ccc118`, 200 frames / 20 seconds / 1280×720.
- Published `waiting-menu-r16.gif`: `b11085824cb37551cb73c3f99f1decfd0e981d9d18313eb7feeee67197547b1b`, 95 frames / 9.5 seconds / 1280×720.
- Transformation: original seconds 0.2–9.7, 10 fps, palette re-encoding only.
- Actual first/middle/last frames were inspected after encoding. All three show the intended start/action/result; full-resolution original open/page/reopen screenshots were also inspected.

The original 20-second clip additionally retains close and reopen. The short
README clip does not claim to show those later actions. The exact run's full
console contained 115 hook-allocation samples, all allocation/delta zero, and
zero Unmapped/fatal/low-stack records. Both archive loads allocated zero;
Training/waiting retirement occurred after 5001/5000 ms. Pooled buffer memory
remained allocated. The run stopped deliberately, and the full profile and
harness were restored and independently verified before release.

This is development evidence for waiting-menu text rendering only. Input-log
contrast failed in the same run and remains open. Other screen, multi-CPU,
hitbox, first-use and online release gates are not qualified by this clip.

The evidence bundle is retained privately under
`tmp-ui-screens/ui21-visible-pending`: raw 45-file manifest SHA-256
`70538e72ff7c799c4bac135e52accc9e6f8548674e8d7f6f11071deaf32fba03`,
archive SHA-256 `7b440d1d401a474919569713262c79804ee3a12ce9dc731f89fb5b1c864e59ec`.
It includes the original run, console and GIFs. Local screenshots, control
receipts, metadata and review contacts are retained alongside it.

An initial helper trim incorrectly lasted 19.8 seconds because its second-pass
`-t` applied to the palette input rather than the output. That file and its
metadata are retained as failed media evidence. A local helper copy corrected
the argument scope; the installed skill was not changed. Inspection of the
published output independently verified 9500 ms before it was added here.
