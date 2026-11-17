//! M3 clatter-write observer: the consumer side of the frozen v1 event interface
//! (artifacts/rc15-mashout/frozen-cluster-devin/M3-EVENT-INTERFACE.md, OPUS-DESIGN-REVIEW-sora116-refreeze.md).
//!
//! Pure and host-tested. The runtime forwarder publishes one [`ClatterWrite`] per qualified native call; this module
//! decides whether an attempt's event stream is usable (C4 generation, C8 gap-free sequence, slot/tag pairing,
//! callee rule) and applies events to the model clock only under G1: `event.old` bit-equal to the model value.

/// The FighterControl data slot an event came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Slot {
    /// Vtable +0x90 → main+0x3f81a0: `remaining = max(operand, 0)`.
    Set = 0,
    /// Vtable +0x98 → main+0x3f81c0: `remaining = max(old + operand, 0)`.
    Add = 1,
}

/// One qualified native write, as published (f32 values as bits).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClatterWrite {
    pub generation: u32,
    pub seq: u32,
    pub slot: Slot,
    pub tag: u16,
    pub old: u32,
    pub operand: u32,
    pub new: u32,
    pub frame: u32,
}

/// Why an attempt's stream is unusable. Any poison makes the attempt unscored (existing no-grade notice).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Poison {
    Overflow,
    NewMismatch,
    Nested,
    WrongSlot,
    Stale,
    Lost,
    SequenceGap,
}

/// The scenario tags and the slot each one may use (C4: exclusive per scenario).
pub const fn slot_for(tag: u16) -> Option<Slot> {
    match tag {
        113 | 116 => Some(Slot::Set),
        117 => Some(Slot::Add),
        _ => None,
    }
}

/// The callee's own store rule in binary32 (S0-CALLEES.asm): Set `max(s0, 0)`; Add `max(old + s0, 0)`.
pub fn callee_result(slot: Slot, old: f32, operand: f32) -> f32 {
    let value = match slot {
        Slot::Set => operand,
        Slot::Add => old + operand,
    };
    value.max(0.0)
}

/// Validates the stream of one attempt in publication order.
#[derive(Clone, Debug)]
pub struct Stream {
    generation: u32,
    tag: u16,
    next_seq: u32,
    poison: Option<Poison>,
}

impl Stream {
    pub const fn new(generation: u32, tag: u16) -> Self {
        Self { generation, tag, next_seq: 1, poison: None }
    }

    pub const fn poison(&self) -> Option<Poison> {
        self.poison
    }

    /// Latches the first poison; later events are ignored.
    pub fn fail(&mut self, why: Poison) {
        if self.poison.is_none() {
            self.poison = Some(why);
        }
    }

    /// Accepts the next event or poisons the attempt. Returns the event only when it is usable.
    pub fn accept(&mut self, event: ClatterWrite) -> Option<ClatterWrite> {
        if self.poison.is_some() {
            return None;
        }
        let why = if event.generation != self.generation {
            Some(Poison::Stale)
        } else if slot_for(self.tag) != Some(event.slot) || event.tag != self.tag {
            Some(Poison::WrongSlot)
        } else if event.seq != self.next_seq {
            Some(Poison::SequenceGap)
        } else if callee_result(event.slot, f32::from_bits(event.old), f32::from_bits(event.operand)).to_bits() != event.new {
            Some(Poison::NewMismatch)
        } else {
            None
        };
        if let Some(why) = why {
            self.fail(why);
            return None;
        }
        self.next_seq += 1;
        Some(event)
    }
}

/// G1: between two ticks the model may adopt an event only when `event.old` is bit-equal to its current value;
/// it then continues from `event.new`. Anything else is the existing MASH-M1-MISMATCH (no adjustment).
pub fn apply(model: f32, event: &ClatterWrite) -> Result<f32, ()> {
    if model.to_bits() == event.old {
        Ok(f32::from_bits(event.new))
    } else {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(seq: u32, old: f32, operand: f32) -> ClatterWrite {
        ClatterWrite { generation: 7, seq, slot: Slot::Set, tag: 113, old: old.to_bits(), operand: operand.to_bits(),
            new: callee_result(Slot::Set, old, operand).to_bits(), frame: 100 + seq }
    }

    #[test]
    fn e_a_113_sets_are_accepted_and_applied_bit_exact() {
        // rc15-frozen113-measure-xps-2, one generation: old 49.545795 → 83.052353 → 100.756279 → 111.138443.
        let olds = [49.545_795_f32, 78.052_353, 95.756_279];
        let news = [83.052_353_f32, 100.756_279, 111.138_443];
        let mut stream = Stream::new(7, 113);
        for (i, (old, new)) in olds.iter().zip(news).enumerate() {
            let event = stream.accept(set(i as u32 + 1, *old, new)).expect("usable");
            assert_eq!(apply(*old, &event), Ok(new));
        }
        assert_eq!(stream.poison(), None);
    }

    #[test]
    fn single_event_attempt() {
        let mut stream = Stream::new(7, 113);
        let event = stream.accept(set(1, 34.278_004, 57.555_145)).unwrap();
        assert_eq!(apply(34.278_004, &event).map(f32::to_bits), Ok(57.555_145_f32.to_bits()));
    }

    #[test]
    fn g1_refuses_a_model_that_is_not_bit_equal() {
        let event = set(1, 34.278_004, 57.555_145);
        assert_eq!(apply(f32::from_bits(34.278_004_f32.to_bits() + 1), &event), Err(()));
    }

    #[test]
    fn poisons() {
        let mut s = Stream::new(7, 113);
        let mut bad = set(1, 10.0, 20.0);
        bad.new = 21.0_f32.to_bits();
        assert!(s.accept(bad).is_none());
        assert_eq!(s.poison(), Some(Poison::NewMismatch));
        assert!(s.accept(set(2, 20.0, 30.0)).is_none(), "poison latches");

        let mut s = Stream::new(7, 113);
        let mut stale = set(1, 10.0, 20.0);
        stale.generation = 6;
        assert!(s.accept(stale).is_none());
        assert_eq!(s.poison(), Some(Poison::Stale));

        let mut s = Stream::new(7, 113);
        assert!(s.accept(set(2, 10.0, 20.0)).is_none());
        assert_eq!(s.poison(), Some(Poison::SequenceGap));

        let mut s = Stream::new(7, 117);
        assert!(s.accept(set(1, 10.0, 20.0)).is_none(), "a Set on the Add scenario");
        assert_eq!(s.poison(), Some(Poison::WrongSlot));

        let mut s = Stream::new(7, 113);
        let mut leak = set(1, 10.0, 20.0);
        leak.tag = 116;
        assert!(s.accept(leak).is_none(), "116 tag in a 113 attempt");
        assert_eq!(s.poison(), Some(Poison::WrongSlot));
    }

    #[test]
    fn add_rule_and_clamp() {
        assert_eq!(callee_result(Slot::Add, 98.0, 15.0), 113.0);
        assert_eq!(callee_result(Slot::Add, 3.0, -5.0), 0.0);
        assert_eq!(callee_result(Slot::Set, 3.0, -5.0), 0.0);
        assert_eq!(slot_for(116), Some(Slot::Set));
        assert_eq!(slot_for(40), None);
    }
}
