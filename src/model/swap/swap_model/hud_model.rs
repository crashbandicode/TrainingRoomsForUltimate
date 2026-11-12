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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::super::presentation::*;
    use super::*;
    use crate::engine::swap::character_forms::Forms;
    use crate::engine::mii::inventory::{MiiClass, CHARINFO_LEN};
    use crate::engine::mii::lifetime::{Configuration, Tokens};
    use crate::engine::mii::native_values::RowIds;

    fn gunner(hat: u16) -> Configuration {
        let mut charinfo = [0x5au8; CHARINFO_LEN];
        charinfo[0] = hat as u8;
        let tokens = Tokens { outfit: 75, hat, colour: 0, voice: 37, specials: [2, 2, 2, 1] };
        let rows = RowIds { outfit: (0x13u64 << 56) | (75u64 << 40) | 0x1bfef7a5f7, hat: (0x5au64 << 56) | (u64::from(hat) << 40) | 0x0fa9efa5ea };
        Configuration::from_observed(MiiClass::Gunner, tokens, charinfo, rows).unwrap()
    }
    fn stamp(owner: u32) -> Stamp { Stamp { scene_epoch: 1, incarnation: 0x1000, logical_owner: owner, physical_owner: owner, revision: 1 } }
    fn intent(kind: u32, c: Option<Configuration>, owner: u32, ctx: Context) -> PresentationIntent {
        PresentationIntent::from_parts(kind, Forms::starting(kind), 0, c, ctx, stamp(owner)).unwrap()
    }

    fn from_parts_matches_todays_portrait_intent_decisions_and_the_form_rows() {
        // Exact(c) -> exact bust; Original/preset Mii -> default class portrait; NotMii -> kind.
        let exact = intent(0x4a, Some(gunner(125)), 0, Context::Training);
        assert!(matches!(exact.presentation, Presentation::ExactMii { class: 0x4a, .. }));
        assert_eq!(exact.expectation().bust, Some(TextureKey::of(&gunner(125))));
        let preset = intent(0x4a, None, 0, Context::Training);
        assert_eq!(preset.presentation, Presentation::DefaultMii { class: 0x4a });
        assert_eq!(preset.expectation(), HudExpectation { kind: 0x4a, bust: None, pair: false, waiting_icon: None });
        let fox = intent(0x7, None, 1, Context::Training);
        assert_eq!(fox.presentation, Presentation::Kind { kind: 0x7, active_index: 0 });
        // A configuration of another class never becomes the exact bust of this kind.
        let wrong = intent(0x48, Some(gunner(125)), 0, Context::Training);
        assert_eq!(wrong.presentation, Presentation::DefaultMii { class: 0x48 });
        // Ice Climbers: two actors -> pair plate; PT: three kinds, one actor -> native form follow.
        let ic = Forms::starting(0x4b);
        if ic.actor_count == 2 {
            let pair = PresentationIntent::from_parts(0x4b, ic, 0, None, Context::Training, stamp(0)).unwrap();
            assert_eq!(pair.presentation, Presentation::Pair { lead: 0x4b });
            assert!(pair.expectation().pair);
        }
        let pt = Forms::starting(0x27);
        if pt.count > 1 && pt.actor_count == 1 {
            let i = PresentationIntent::from_parts(0x27, pt, 0, None, Context::Training, stamp(0)).unwrap();
            assert!(i.rule.native_form_follow);
            assert_eq!(i.after_native_form(0x28, 1).presentation, Presentation::Kind { kind: 0x28, active_index: 1 });
        }
        // Texture keys separate faces with equal class and tokens.
        assert_ne!(TextureKey::of(&gunner(125)), TextureKey::of(&gunner(0)));
    }

    fn h6_contexts_either_present_or_are_refused_at_planning_never_skipped_after() {
        assert!(PresentationIntent::from_parts(0x7, Forms::starting(0x7), 0, None, Context::QpWaiting, stamp(4)).is_ok());
        assert_eq!(PresentationIntent::from_parts(0x7, Forms::starting(0x7), 0, None, Context::QpWaiting, stamp(1)),
            Err(Unsupported::Context { context: Context::QpWaiting, physical_owner: 1 }));
        assert_eq!(PresentationIntent::from_parts(0x7, Forms::starting(0x7), 0, None, Context::Training, stamp(4)),
            Err(Unsupported::Context { context: Context::Training, physical_owner: 4 }));
        let cpu4 = PresentationIntent::from_parts(0x4a, Forms::starting(0x4a), 0, Some(gunner(125)), Context::LdnWaiting, stamp(4)).unwrap();
        assert_eq!(cpu4.expectation().waiting_icon, Some(0x4a));
        assert_eq!(ordering_for(TransitionKind::PhaseBuild), Ordering::AfterBuildVerified);
        assert_eq!(ordering_for(TransitionKind::CanonicalReload), Ordering::AfterReady);
    }

    fn h1_h2_publication_after_every_settle_matches_the_derived_intent_and_degraded_still_publishes() {
        let mut m = HudModel::with_css([0x3, 0x9]);
        let exact = intent(0x4a, Some(gunner(125)), 0, Context::Training);
        // Adopted: exact bust bound and read back.
        assert_eq!(m.settle_and_publish(&exact, Settlement::Adopted), Ok(()));
        assert_eq!(m.observed(0).unwrap().bust, Some(TextureKey::of(&gunner(125))));
        // Degraded (M3 shape): the live Mii kind is published, not the bust; readback passes
        // against the degraded presentation and the card is never left on the previous fighter.
        let mut m2 = HudModel::with_css([0x3, 0x9]);
        assert_eq!(m2.settle_and_publish(&exact, Settlement::Degraded), Ok(()));
        assert_eq!(m2.observed(0), Some(HudExpectation { kind: 0x4a, bust: None, pair: false, waiting_icon: None }));
        // Diverged: the card shows the live kind read back.
        let mut m3 = HudModel::with_css([0x3, 0x9]);
        assert_eq!(m3.settle_and_publish(&exact, Settlement::Diverged { live_kind: 0x3, active_index: 0 }), Ok(()));
        assert_eq!(m3.observed(0).unwrap().kind, 0x3);
        // A sequence over the four classes × both roles keeps readback == derived intent.
        let steps = [(0u32, 0x7u32, None), (1, 0x4a, Some(gunner(125))), (0, 0x4a, Some(gunner(0))), (1, 0x9, None), (0, 0x3, None)];
        for (owner, kind, c) in steps {
            let i = intent(kind, c, owner, Context::Training);
            assert_eq!(m.settle_and_publish(&i, Settlement::Adopted), Ok(()), "owner {owner} kind {kind:#x}");
            assert_eq!(m.observed(owner), Some(i.expectation()));
        }
        // The expectation (the rule) mismatches a stale card: a readback finds it.
        let stale = intent(0x7, None, 0, Context::Training);
        assert_eq!(m.readback(0, &stale.expectation()), Err("card kind differs from the expectation"));
    }

    fn h3_native_reset_never_leaves_a_mii_card_static_when_a_configuration_is_the_target() {
        let mut m = HudModel::with_css([0x3, 0x9]);
        let exact = intent(0x4a, Some(gunner(125)), 0, Context::Training);
        let fox = intent(0x7, None, 1, Context::Training);
        m.settle_and_publish(&exact, Settlement::Adopted).unwrap();
        m.settle_and_publish(&fox, Settlement::Adopted).unwrap();
        // The reset alone rebuilds from CSS (Samus/Mario) and drops the bust.
        let mut bare = m.clone();
        bare.native_reset([0x3, 0x9], [None, None]);
        assert_eq!(bare.observed(0).unwrap().kind, 0x3);
        assert_eq!(bare.observed(0).unwrap().bust, None);
        // The lifecycle hook re-derives from the current intents: ordinary admitted, exact rebound.
        let results = m.on_native_reset([0x3, 0x9], [Some(&exact), Some(&fox)]);
        assert_eq!(results, [Ok(()), Ok(())]);
        assert_eq!(m.observed(0), Some(exact.expectation()));
        assert_eq!(m.observed(1), Some(fox.expectation()));
    }

    fn h4_h5_a_state_load_changes_no_card_and_replay_previews_keep_the_bust() {
        let mut m = HudModel::with_css([0x3, 0x9]);
        let exact = intent(0x4a, Some(gunner(125)), 0, Context::Training);
        m.settle_and_publish(&exact, Settlement::Adopted).unwrap();
        let before = m.clone();
        // Save on A, swap to B, load: the load applies position only (B123PRE row 8); verify only.
        assert_eq!(m.on_state_load(0, &exact), Ok(()));
        assert_eq!(m, before);
        // Replay X/Y/B (preview, restart, close) emit no portrait event: the bust stays bound.
        for _ in ["X", "Y", "B"] { assert_eq!(m.readback(0, &exact.expectation()), Ok(())); }
        // A load after a stale card (M5 shape) is reported, not silently accepted.
        let mut stale = HudModel::with_css([0x3, 0x9]);
        stale.publish(0, Presentation::DefaultMii { class: 0x4a });
        assert_eq!(stale.on_state_load(0, &exact), Err("exact bust not bound"));
    }

    fn native_form_change_follows_the_member_and_the_waiting_icon_tracks_cpu4() {
        let mut m = HudModel::with_css([0x27, 0x9]);
        let pt = PresentationIntent::from_parts(0x27, Forms::starting(0x27), 0, None, Context::Training, stamp(0)).unwrap();
        m.native_form_change(0, 0x28);
        let after = pt.after_native_form(0x28, 1);
        assert_eq!(m.readback(0, &after.expectation()), Ok(()));
        assert_eq!(m.readback(0, &pt.expectation()), Err("card kind differs from the expectation"));
        let mut w = HudModel::default();
        w.event_0x1d(4, 0x9, 0); w.set_waiting_icon(4, 0x9);
        let wario = PresentationIntent::from_parts(0x2d, Forms::starting(0x2d), 0, None, Context::QpWaiting, stamp(4)).unwrap();
        w.publish(4, wario.presentation);
        assert_eq!(w.readback(4, &wario.expectation()), Ok(()));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("from_parts_matches_todays_portrait_intent_decisions_and_the_form_rows", from_parts_matches_todays_portrait_intent_decisions_and_the_form_rows as fn()),
            ("h6_contexts_either_present_or_are_refused_at_planning_never_skipped_after", h6_contexts_either_present_or_are_refused_at_planning_never_skipped_after as fn()),
            ("h1_h2_publication_after_every_settle_matches_the_derived_intent_and_degraded_still_publishes", h1_h2_publication_after_every_settle_matches_the_derived_intent_and_degraded_still_publishes as fn()),
            ("h3_native_reset_never_leaves_a_mii_card_static_when_a_configuration_is_the_target", h3_native_reset_never_leaves_a_mii_card_static_when_a_configuration_is_the_target as fn()),
            ("h4_h5_a_state_load_changes_no_card_and_replay_previews_keep_the_bust", h4_h5_a_state_load_changes_no_card_and_replay_previews_keep_the_bust as fn()),
            ("native_form_change_follows_the_member_and_the_waiting_icon_tracks_cpu4", native_form_change_follows_the_member_and_the_waiting_icon_tracks_cpu4 as fn()),
        ]);
    }
    }
}
