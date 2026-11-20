# Guard the Edge + Juggler: design (ships in rc.15)

**Scope (operator, 2026-11-20 07:47Z):** Guard the Edge and Juggler are part of the rc.15 card build. Projectile Madness
is the next build. An earlier draft of this page said "the build after rc.15"; that was the primary's misreading.

Research and design by the edge-juggle-opus lane (Claude Opus 5.5 high). There's no code yet. The static findings were **checked on Eden** (P0, 2026-11-20 01:21Z: 21,143 per-frame
stock-AI records on the exact card build; [P0 results](edge-juggle/evidence/P0-RESULTS.md)). Confirmed: always the nearer
ledge, identical starts repeat identical paths, and the air dodge only from about 20 units out. Corrected: the CPU jumps on
its first actionable frame, not late. Supported (one sample): the drift goes toward you. Not yet covered: starts that
force an up-special, recovering against an active edge-guard, and juggle rates.
Full documents: [RESEARCH.md](edge-juggle/RESEARCH.md) and [DESIGN.md](edge-juggle/DESIGN.md).

## What the game's CPU does wrong (from its own code; recovery points checked on Eden)

**Recovering (Guard the Edge):**
- it always goes to the **nearer ledge**, and identical starts repeat identical paths (confirmed);
- it air-dodges only when already close to the ledge, about 20 units out (confirmed);
- it jumps on its first actionable frame every time (corrected on Eden; the static read said fixed heights);
- it sometimes **attacks a nearby edge-guarder instead of recovering**.

**Being juggled (Juggler):**
- it air-dodges only when it predicts a hit on its own path;
- it rarely double jumps;
- its **DI and drift point toward you**.

## Perfect mode builds on the vanilla CPU

Perfect = the stock level-9 CPU plus **one small correction per weak spot**. The corrections sit right after the game's AI
decides, at a hook TR already owns (where Mash out already drives the CPU's controller). There's no new decision system
and no teleporting; every correction is a real controller input. Level 9 mode runs none of the corrections.

![Perfect mode architecture](edge-juggle/illustrations/A-perfect-architecture.png)

![Recovery routes](edge-juggle/illustrations/B-recovery-routes.png)

## Six questions for you (recommendation first)

**Decided (operator, 2026-11-20 ~02:06Z):** Q1 **all four areas**; Q2 agreed; Q3 the Juggler clock starts at **release**; Q4 you pick your start in the wizard; Q5 a **reaction-speed setting** offering all three options; Q6 agreed. Also added: Perfect must handle each fighter's niche recoveries, including recovering **under the stage to the opposite ledge** (e.g. Pikachu, whose Thunder can hit you while crossing). Whether that's possible is decided from the stage's real collision geometry, so stages that block it (e.g. Yoshi's Story) are handled. Design v2 is in progress; implementation has started (Codex GPT-6.2 Sol, paired with the Opus designer).

1. **Start areas for Guard the Edge:** two (off-stage lower left / lower right, as you asked) or four (adding upper left
   / upper right)? Recommended: **two**. ![Q1](edge-juggle/illustrations/Q1-edge-areas.png)
2. **How the CPU starts in Guard the Edge:** in free fall, after a real launch, or both? Recommended: **free fall first,
   launch later**. ![Q2](edge-juggle/illustrations/Q2-edge-start.png)
3. **Juggler clock:** from your first hit or from release; does a KO keep the airtime? Recommended: **from the first
   hit; a KO keeps its airtime**. ![Q3](edge-juggle/illustrations/Q3-juggle-clock.png)
4. **Juggler: where do you start?** Recommended: **under the CPU**. ![Q4](edge-juggle/illustrations/Q4-juggle-p1.png)
5. **Perfect's reaction speed:** frame-perfect, a human floor, or a setting? Recommended: **a 12-frame floor**.
   ![Q5](edge-juggle/illustrations/Q5-perfect-reaction.png)
6. **While a game runs:** should TR suspend its other CPU settings and set CPU Behavior to CPU (Lv. 9), then restore
   them on Stop? Recommended: **yes**. ![Q6](edge-juggle/illustrations/Q6-cpu-settings.png)

The pictures are mockups, not game footage.

## Design v2 (2026-11-20 02:11Z)

- **Recovery planner (Perfect):** near ledge, the **opposite ledge under the stage**, high to stage, or late and low. It decides from the stage's live collision lines, the CPU's position, velocity, jumps and specials, and per-fighter limits. It penalises repeating a route, and Pikachu's Thunder counts as a threat to you on the way out.
- **Niche-recovery table:** your list is treated as candidates, all marked **unverified** until an Eden trace on the release shows them. Verified first: Pikachu, Sora, R.O.B., Diddy and Game & Watch, each on Final Destination plus one stage that can't be crossed.
- **Stages:** whether a crossing is possible comes from a geometry test under the main stage, for each fighter's limits. There's no hand-made list; stages shaped like Yoshi's Story fail the test.
- **Reaction setting:** Frame-perfect / Human (12 frames) / Custom (1–30 frames).

![Four start areas](edge-juggle/illustrations/V2-edge-four-areas.png)

![Recovery planner](edge-juggle/illustrations/V2-recovery-planner.png)
