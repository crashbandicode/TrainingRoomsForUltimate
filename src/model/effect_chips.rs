//! The DI and SDI direction chips drawn from the game's own effect textures (stream u4b; SPEC-core-07 section 4.6, MENU-DRAFT
//! *Direction chips*, decisions Q17 and N10): which texture a chip draws, where and how large, and the page's texture life cycle
//! (request when the grid opens, release when it closes or changes, drop at once when the layout goes away).
//!
//! A chip of the menu has two picture panes, the tile and the pointer (`panes::IDX_CHIP`, `IDX_CHIP_NEEDLE`). The tile keeps the
//! chip's square and weight colour; the pointer carries the effect's main layer, turned to the chip's direction the way the game
//! turns the effect (one picture, rotated about Z, never mirrored). DI draws the visual's streak (the emitter texture of the first
//! sampler that is tallest for its width: `ef_cmn_line09` in 13.0.5), SDI the burst (the first-sampler texture that is widest:
//! `ef_cmn_impact13`), stretched x2.0 along the SDI vector (N10). Neutral keeps the ring; Left / Right are In / Out.

use super::frame::ChipVisual;

/// The furthest reach of the visual from the chip centre, as a fraction of the chip size (W9 rule 7, the mockup's `chip_direction`).
pub const REACH: f32 = 0.46;
/// The SDI burst is drawn twice as long along the SDI vector (N10, x2.0 confirmed 2026-11-02).
pub const SDI_STRETCH: f32 = 2.0;
/// The DI streak's width for its length: the streak emitter's base scale (1.2 wide, 3.5 long; W9's parameter table).
pub const DI_WIDTH_RATIO: f32 = 1.2 / 3.5;

/// One candidate texture of a visual: its emitter sampler and size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub sampler: usize,
    pub width: u32,
    pub height: u32,
}

/// Which candidate the chips draw: DI the first-sampler texture with the smallest width / height (the streak), SDI the one with the
/// largest (the burst band). `None` for the arrow grids or when no first-sampler texture exists.
pub fn pick_layer(visual: ChipVisual, candidates: &[Candidate]) -> Option<usize> {
    let ratio = |c: &Candidate| c.width as f32 / c.height.max(1) as f32;
    let firsts = candidates.iter().enumerate().filter(|(_, c)| c.sampler == 0 && c.width > 0 && c.height > 0);
    match visual {
        ChipVisual::None => None,
        ChipVisual::Di => firsts.fold(None, |best: Option<(usize, f32)>, (i, c)| match best {
            Some((_, r)) if r <= ratio(c) => best,
            _ => Some((i, ratio(c))),
        }),
        ChipVisual::Sdi => firsts.fold(None, |best: Option<(usize, f32)>, (i, c)| match best {
            Some((_, r)) if r >= ratio(c) => best,
            _ => Some((i, ratio(c))),
        }),
    }
    .map(|(i, _)| i)
}

/// Where the pointer pane goes for a chip: centre, size and the pane's Z rotation (degrees).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub rot: f32,
}

/// The layer of a chip whose direction is `angle` degrees (counter-clockwise from the screen's right), centred at `(cx, cy)` in a chip of
/// side `chip`. The texture's +Y runs along the direction (both effects are authored along +Y), its origin end sits on the chip centre.
pub fn place(visual: ChipVisual, angle: f32, cx: f32, cy: f32, chip: f32, tex_w: u32, tex_h: u32) -> Placement {
    let h = REACH * chip;
    let w = match visual {
        ChipVisual::Sdi => h / SDI_STRETCH * tex_w as f32 / tex_h.max(1) as f32,
        _ => h * DI_WIDTH_RATIO,
    };
    let (dx, dy) = (angle.to_radians().cos(), angle.to_radians().sin());
    Placement { x: cx + dx * h / 2.0, y: cy + dy * h / 2.0, w, h, rot: angle - 90.0 }
}

/// What the page holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    Nothing,
    /// A texture of this visual is bound to the pointers.
    Bound(ChipVisual),
    /// This visual could not be built (not resident, a guard failed, the pool is full): the arrows stay until the grid closes.
    Failed(ChipVisual),
}

/// What the page does this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Keep,
    /// Unbind the pointers and retire the texture (or forget a failure).
    Drop,
    Request(ChipVisual),
    DropThenRequest(ChipVisual),
}

/// The page's step for the visual the frame wants (`ChipVisual::None` when the menu is closed or shows another view).
pub fn plan(held: Held, wanted: ChipVisual) -> Step {
    match (held, wanted) {
        (Held::Nothing, ChipVisual::None) => Step::Keep,
        (Held::Bound(_) | Held::Failed(_), ChipVisual::None) => Step::Drop,
        (Held::Bound(v) | Held::Failed(v), w) if v == w => Step::Keep,
        (Held::Bound(_), w) => Step::DropThenRequest(w),
        (Held::Failed(_) | Held::Nothing, w) => Step::Request(w),
    }
}

/// The page's state as words, so the UI tick and the view read it with plain atomic loads (no lock on the game's UI thread: the
/// stream/sync rule that a game-thread path never waits on a lock).
pub fn visual_code(v: ChipVisual) -> u8 {
    match v {
        ChipVisual::None => 0,
        ChipVisual::Di => 1,
        ChipVisual::Sdi => 2,
    }
}

pub fn visual_of(code: u8) -> ChipVisual {
    match code {
        1 => ChipVisual::Di,
        2 => ChipVisual::Sdi,
        _ => ChipVisual::None,
    }
}

pub fn held_code(h: Held) -> u8 {
    match h {
        Held::Nothing => 0,
        Held::Bound(v) => 0x10 | visual_code(v),
        Held::Failed(v) => 0x20 | visual_code(v),
    }
}

pub fn held_of(code: u8) -> Held {
    match code & 0xf0 {
        0x10 => Held::Bound(visual_of(code & 0x0f)),
        0x20 => Held::Failed(visual_of(code & 0x0f)),
        _ => Held::Nothing,
    }
}

/// What the view draws for a bound chip: visual, texture size and tint (alpha is always opaque).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerWord {
    pub visual: ChipVisual,
    pub width: u32,
    pub height: u32,
    pub tint: [u8; 4],
}

/// `visual` (8 bits) | width (16) | height (16) | tint RGB (24); 0 means no layer.
pub fn pack_layer(l: LayerWord) -> u64 {
    if l.visual == ChipVisual::None {
        return 0;
    }
    u64::from(visual_code(l.visual)) << 56
        | u64::from(l.width.min(0xffff)) << 40
        | u64::from(l.height.min(0xffff)) << 24
        | u64::from(l.tint[0]) << 16
        | u64::from(l.tint[1]) << 8
        | u64::from(l.tint[2])
}

pub fn unpack_layer(word: u64) -> Option<LayerWord> {
    let visual = visual_of((word >> 56) as u8);
    if visual == ChipVisual::None {
        return None;
    }
    Some(LayerWord {
        visual,
        width: ((word >> 40) & 0xffff) as u32,
        height: ((word >> 24) & 0xffff) as u32,
        tint: [(word >> 16) as u8, (word >> 8) as u8, word as u8, 255],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_state_words_round_trip() {
        for h in [Held::Nothing, Held::Bound(ChipVisual::Di), Held::Bound(ChipVisual::Sdi), Held::Failed(ChipVisual::Di), Held::Failed(ChipVisual::Sdi)] {
            assert_eq!(held_of(held_code(h)), h);
        }
        for v in [ChipVisual::None, ChipVisual::Di, ChipVisual::Sdi] {
            assert_eq!(visual_of(visual_code(v)), v);
        }
        let l = LayerWord { visual: ChipVisual::Sdi, width: 256, height: 128, tint: [200, 244, 255, 255] };
        assert_eq!(unpack_layer(pack_layer(l)), Some(l));
        assert_eq!(unpack_layer(0), None);
        assert_eq!(pack_layer(LayerWord { visual: ChipVisual::None, ..l }), 0);
    }

    /// The 13.0.5 pictures in `pictures()` order: DI tri00, line09, line11, flare01; SDI line00, impact13, mask00, grade00.
    const DI: [Candidate; 4] = [
        Candidate { sampler: 0, width: 64, height: 128 },
        Candidate { sampler: 0, width: 64, height: 256 },
        Candidate { sampler: 0, width: 64, height: 128 },
        Candidate { sampler: 0, width: 32, height: 32 },
    ];
    const SDI: [Candidate; 4] = [
        Candidate { sampler: 0, width: 64, height: 128 },
        Candidate { sampler: 0, width: 256, height: 128 },
        Candidate { sampler: 1, width: 128, height: 128 },
        Candidate { sampler: 2, width: 64, height: 64 },
    ];

    #[test]
    fn di_draws_the_streak_and_sdi_the_burst() {
        assert_eq!(pick_layer(ChipVisual::Di, &DI), Some(1), "line09, 64 x 256");
        assert_eq!(pick_layer(ChipVisual::Sdi, &SDI), Some(1), "impact13, 256 x 128");
        assert_eq!(pick_layer(ChipVisual::None, &DI), None);
        assert_eq!(pick_layer(ChipVisual::Sdi, &SDI[2..]), None, "only later samplers: no burst");
        assert_eq!(pick_layer(ChipVisual::Di, &[]), None);
    }

    #[test]
    fn equal_layer_ratios_keep_the_first_sampler_zero_picture() {
        let candidates = [
            Candidate { sampler: 1, width: 1, height: 1000 },
            Candidate { sampler: 0, width: 64, height: 128 },
            Candidate { sampler: 0, width: 128, height: 256 },
        ];
        assert_eq!(pick_layer(ChipVisual::Di, &candidates), Some(1));
        assert_eq!(pick_layer(ChipVisual::Sdi, &candidates), Some(1));
    }

    #[test]
    fn the_layer_runs_from_the_centre_along_the_direction() {
        let p = place(ChipVisual::Di, 90.0, 100.0, 50.0, 184.0, 64, 256);
        let reach = REACH * 184.0;
        assert!((p.h - reach).abs() < 1e-3);
        assert!((p.x - 100.0).abs() < 1e-3 && (p.y - (50.0 + reach / 2.0)).abs() < 1e-3, "{p:?}");
        assert_eq!(p.rot, 0.0, "Up: the texture's +Y is already up");
        let out = place(ChipVisual::Di, 0.0, 0.0, 0.0, 184.0, 64, 256);
        assert!((out.x - reach / 2.0).abs() < 1e-3 && out.y.abs() < 1e-3 && out.rot == -90.0, "{out:?}");
    }

    #[test]
    fn the_sdi_burst_is_stretched_two_times_along_the_vector() {
        let p = place(ChipVisual::Sdi, 90.0, 0.0, 0.0, 184.0, 256, 128);
        // Unstretched, the 2:1 band would be h/2 tall for its width; drawn twice as tall.
        assert!((p.h / (p.w * 128.0 / 256.0) - SDI_STRETCH).abs() < 1e-4, "{p:?}");
        let di = place(ChipVisual::Di, 90.0, 0.0, 0.0, 184.0, 64, 256);
        assert!((di.w / di.h - DI_WIDTH_RATIO).abs() < 1e-6, "the DI streak keeps the emitter's proportions");
    }

    #[test]
    fn the_page_requests_on_open_releases_on_close_and_swaps_between_grids() {
        assert_eq!(plan(Held::Nothing, ChipVisual::None), Step::Keep);
        assert_eq!(plan(Held::Nothing, ChipVisual::Di), Step::Request(ChipVisual::Di));
        assert_eq!(plan(Held::Bound(ChipVisual::Di), ChipVisual::Di), Step::Keep);
        assert_eq!(plan(Held::Bound(ChipVisual::Di), ChipVisual::Sdi), Step::DropThenRequest(ChipVisual::Sdi));
        assert_eq!(plan(Held::Bound(ChipVisual::Sdi), ChipVisual::None), Step::Drop, "grid closed or menu hidden");
    }

    #[test]
    fn a_failed_visual_is_not_retried_every_frame_but_after_the_grid_closes() {
        assert_eq!(plan(Held::Failed(ChipVisual::Di), ChipVisual::Di), Step::Keep);
        assert_eq!(plan(Held::Failed(ChipVisual::Di), ChipVisual::None), Step::Drop);
        assert_eq!(plan(Held::Failed(ChipVisual::Di), ChipVisual::Sdi), Step::Request(ChipVisual::Sdi));
    }
}
