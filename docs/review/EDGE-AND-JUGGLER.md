# Guard the Edge + Juggler: design draft (for the build after rc.15)

Research and design by the edge-juggle-opus lane (Claude Opus 5.5 high). There's no code yet. This is a **draft**: it's
based on reading the game's own CPU code statically, and **the Eden traces that confirm it are pending** (phase P0).
Full documents: [RESEARCH.md](edge-juggle/RESEARCH.md) and [DESIGN.md](edge-juggle/DESIGN.md).

## What the game's CPU does wrong (from its own code; to be confirmed on Eden)

**Recovering (Guard the Edge):**
- it always goes to the **nearer ledge**;
- it air-dodges only when already close to the ledge;
- it jumps at fixed heights;
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
