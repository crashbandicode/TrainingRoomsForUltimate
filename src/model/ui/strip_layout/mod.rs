//! Pane placement shared by the layout builder and the live frame strip.
pub mod card;
pub mod strip;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub rot: f32,
}

impl Place {
    pub const fn new(x: f32, y: f32, w: f32) -> Self {
        Self { x, y, w, rot: 0.0 }
    }
}

#[derive(Clone, Copy)]
pub struct LayoutSpec {
    pub strip: strip::StripSpec,
    pub card: card::CardSpec,
    pub lobby_drop: f32,
    pub covered_alpha: u8,
    pub keep_drill_history: bool,
}

pub const C: LayoutSpec = LayoutSpec {
    strip: strip::C,
    card: card::C,
    lobby_drop: 96.0,
    covered_alpha: 89,
    keep_drill_history: false,
};

impl LayoutSpec {
    pub const fn history_row(&self, enabled: bool, parry: bool, card: bool) -> bool {
        parry || (enabled && (!card || self.keep_drill_history))
    }

    pub const fn scene_drop(&self, waiting: bool) -> f32 {
        if waiting {
            self.lobby_drop
        } else {
            0.0
        }
    }

    /// Fighter anchors and pane bounds use the HUD canvas, with y increasing upward. `card`: the card drawn (a
    /// sequence's or a single combo's) and the lines of its reason, which grow it.
    pub fn opacity(
        &self,
        drop: f32,
        history: bool,
        card: Option<(bool, usize)>,
        anchors: impl IntoIterator<Item = [f32; 2]>,
    ) -> u8 {
        let strip = self.strip.bounds(history, card.is_some());
        let card = card.map(|(sequence, reason_lines)| self.card.grown(sequence, reason_lines));
        let hits = |[x, y, w, h]: [f32; 4], [fx, fy]: [f32; 2]| {
            (fx - (x + self.strip.origin_x)).abs() <= w / 2.0
                && (fy - (y + self.strip.origin_y - drop)).abs() <= h / 2.0
        };
        if anchors
            .into_iter()
            .any(|anchor| hits(strip, anchor) || card.is_some_and(|b| hits(b, anchor)))
        {
            self.covered_alpha
        } else {
            u8::MAX
        }
    }
}
