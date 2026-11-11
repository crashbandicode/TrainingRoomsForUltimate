//! Host model of the native HUD inputs (WP-P, `redesign/fable/HUD.md` §5.1): the eight
//! cards, the texture table, and the four event classes the operator named — direct and
//! rotational swaps (publication), L+R+A native reset, state load (no card change), and
//! native form changes. The properties H1–H6 of WORKPLAN §2.12 are the tests below. The
//! model is an oracle of native behaviour as read from the listings (event `0x1d` paints a
//! static portrait by kind and sets the animation byte `record+0x23b`; the bind sets
//! `+0x1a8 = 1` and registers the table entry; native Training reset rebuilds every card
//! from the CSS identity unless the mod admits the live body), not a copy of production code.
use super::presentation::{presentation_after_settle, verify, HudExpectation, Presentation, PresentationIntent, Settlement, TextureKey};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudCard { pub owner: u32, pub kind: u32, pub bust_mode: bool, pub texture: Option<TextureKey>, pub anim: u8, pub waiting_icon: Option<u32> }

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudModel { pub cards: [Option<HudCard>; 8], pub table: BTreeMap<u8, TextureKey> }

impl HudModel {
    pub fn with_css(css: [u32; 2]) -> Self {
        let mut m = Self::default();
        for (owner, kind) in css.iter().copied().enumerate() { m.event_0x1d(owner as u32, kind, 0); }
        m
    }
    /// Synthetic or native portrait event: static portrait by kind, animation byte set,
    /// any bust binding dropped (the applier draws by kind while `+0x1a8 == 0`).
    pub fn event_0x1d(&mut self, owner: u32, kind: u32, _active_index: u8) {
        let waiting = self.cards[owner as usize].and_then(|c| c.waiting_icon).map(|_| kind);
        self.cards[owner as usize] = Some(HudCard { owner, kind, bust_mode: false, texture: None, anim: 1, waiting_icon: waiting });
    }
    /// Exact bust bind: `+0x1a8 = 1` and the table entry for this owner's helper row.
    pub fn bind_bust(&mut self, owner: u32, key: TextureKey) {
        if let Some(card) = self.cards[owner as usize].as_mut() {
            card.bust_mode = true; card.texture = Some(key);
            self.table.insert(0x12 + owner as u8, key);
        }
    }
    pub fn set_waiting_icon(&mut self, owner: u32, kind: u32) {
        if let Some(card) = self.cards[owner as usize].as_mut() { card.waiting_icon = Some(kind); }
    }
    /// Native Training reset: every card is rebuilt from the CSS identity unless the mod
    /// admitted the live body (`allow_portrait` `x8 = 0`), in which case the live kind is
    /// kept; busts are lost either way until a rebind.
    pub fn native_reset(&mut self, css: [u32; 2], admitted: [Option<u32>; 2]) {
        for owner in 0..2u32 {
            let kind = admitted[owner as usize].unwrap_or(css[owner as usize]);
            self.event_0x1d(owner, kind, 0);
        }
    }
    /// PT/Aegis Down-B: native delivers its own `0x1d` for the new member.
    pub fn native_form_change(&mut self, owner: u32, kind: u32) { self.event_0x1d(owner, kind, 0); }
    /// The value-only readback of one card.
    pub fn observed(&self, owner: u32) -> Option<HudExpectation> {
        self.cards[owner as usize].map(|c| HudExpectation { kind: c.kind, bust: c.texture.filter(|_| c.bust_mode), pair: false, waiting_icon: c.waiting_icon })
    }
    /// The publisher applied to the model: the kind event, then the bust bind when the
    /// presentation is an exact Mii, the pair plate as its lead's event.
    pub fn publish(&mut self, owner: u32, presentation: Presentation) {
        match presentation {
            Presentation::Kind { kind, active_index } => self.event_0x1d(owner, kind, active_index),
            Presentation::Pair { lead } => self.event_0x1d(owner, lead, 0),
            Presentation::DefaultMii { class } => self.event_0x1d(owner, class, 0),
            Presentation::ExactMii { class, texture } => {
                self.event_0x1d(owner, class, 0);
                self.bind_bust(owner, texture);
            }
        }
    }
    /// Publish the intent after a settle outcome and read it back (rules 1–4 of §2.12).
    pub fn settle_and_publish(&mut self, intent: &PresentationIntent, settlement: Settlement) -> Result<(), &'static str> {
        let presentation = presentation_after_settle(intent, settlement);
        self.publish(intent.stamp.physical_owner, presentation);
        let expected = PresentationIntent { presentation, ..*intent }.expectation();
        self.readback(intent.stamp.physical_owner, &expected)
    }
    pub fn readback(&self, owner: u32, expected: &HudExpectation) -> Result<(), &'static str> {
        let Some(card) = self.cards[owner as usize] else { return Err("no card for the owner"); };
        // The pair plate is not a separate card value in this model (both actors share the lead's card).
        verify(&HudExpectation { pair: false, ..*expected }, Some(card.kind), card.texture.filter(|_| card.bust_mode), false, card.waiting_icon)
    }
    /// Lifecycle: L+R+A re-derives from the current intent and republishes (ordinary admit
    /// or exact rebind), then verifies.
    pub fn on_native_reset(&mut self, css: [u32; 2], intents: [Option<&PresentationIntent>; 2]) -> [Result<(), &'static str>; 2] {
        let admitted = [0, 1].map(|o| intents[o].map(|i| i.presentation.kind()));
        self.native_reset(css, admitted);
        [0u32, 1].map(|o| match intents[o as usize] {
            Some(intent) => { self.publish(o, intent.presentation); self.readback(o, &intent.expectation()) }
            None => Ok(()),
        })
    }
    /// Lifecycle: a save-state load changes no card; verify only.
    pub fn on_state_load(&self, owner: u32, intent: &PresentationIntent) -> Result<(), &'static str> {
        self.readback(owner, &intent.expectation())
    }
}

