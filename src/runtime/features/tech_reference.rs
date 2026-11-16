//! Diagnostic identity of the final stopped hit, not the first hit in a stop interval.
//! A zero reaction record is an observed cleared record, not a positive launch event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    pub attacker: u32,
    pub slot: u8,
    pub damage: u32,
    pub kb: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invalid {
    NoHit,
    Attacker,
    Slot,
    Damage,
    Knockback,
}
impl Invalid {
    pub fn field(self) -> &'static str {
        match self {
            Self::NoHit => "no_positive_hit",
            Self::Attacker => "attacker",
            Self::Slot => "slot",
            Self::Damage => "damage",
            Self::Knockback => "kb",
        }
    }
}
#[derive(Clone, Copy)]
pub struct Tracker {
    hit: Option<Identity>,
}
impl Tracker {
    pub const fn new() -> Self {
        Self { hit: None }
    }
    /// Positive native hit records supersede earlier hits while stopped.
    pub fn stopped(&mut self, record: Identity) -> Result<Identity, Invalid> {
        let kb = f32::from_bits(record.kb);
        if kb.is_finite() && kb > 0.0 {
            self.hit = Some(record);
            return Ok(record);
        }
        self.validate(record)
    }
    /// Called at k0 against the last stopped capture. Only native +0 clearing
    /// may lack a new KB; attacker, slot and total damage must still be exact.
    pub fn validate(&self, record: Identity) -> Result<Identity, Invalid> {
        let hit = self.hit.ok_or(Invalid::NoHit)?;
        if record.attacker != hit.attacker {
            return Err(Invalid::Attacker);
        }
        if record.slot != hit.slot {
            return Err(Invalid::Slot);
        }
        if record.damage != hit.damage {
            return Err(Invalid::Damage);
        }
        if record.kb != 0 && record.kb != hit.kb {
            return Err(Invalid::Knockback);
        }
        Ok(hit)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn first() -> Identity {
        Identity {
            attacker: 0x30000,
            slot: 0,
            damage: 0x420c7ae1,
            kb: 0x42ac85a6,
        }
    }
    #[test]
    fn later_stopped_hit_replaces_the_earlier_reference() {
        let mut t = Tracker::new();
        let a = first();
        t.stopped(a).unwrap();
        // Retained Luma failure shape; second KB is a synthetic distinct bit pattern.
        let b = Identity {
            attacker: 0x101f0000,
            slot: 1,
            damage: 41.420_f32.to_bits(),
            kb: a.kb + 1,
        };
        assert_eq!(t.stopped(b), Ok(b));
        assert_eq!(t.stopped(Identity { kb: 0, ..b }), Ok(b));
        assert_eq!(t.validate(Identity { kb: 0, ..b }), Ok(b));
        assert_ne!(t.validate(Identity { kb: 0, ..b }), Ok(a));
    }
    #[test]
    fn every_identity_change_between_final_capture_and_k0_is_invalid() {
        let mut t = Tracker::new();
        let a = first();
        t.stopped(a).unwrap();
        for (record, reason) in [
            (
                Identity {
                    attacker: a.attacker + 1,
                    ..a
                },
                Invalid::Attacker,
            ),
            (
                Identity {
                    slot: a.slot + 1,
                    ..a
                },
                Invalid::Slot,
            ),
            (
                Identity {
                    damage: a.damage + 1,
                    ..a
                },
                Invalid::Damage,
            ),
            (Identity { kb: a.kb + 1, ..a }, Invalid::Knockback),
        ] {
            assert_eq!(t.validate(record), Err(reason));
            assert!(!reason.field().is_empty());
        }
    }
    #[test]
    fn cleared_record_cannot_hide_attacker_slot_or_damage_changes() {
        let mut t = Tracker::new();
        let a = first();
        t.stopped(a).unwrap();
        for record in [
            Identity {
                kb: 0,
                attacker: a.attacker + 1,
                ..a
            },
            Identity {
                kb: 0,
                slot: a.slot + 1,
                ..a
            },
            Identity {
                kb: 0,
                damage: a.damage + 1,
                ..a
            },
        ] {
            assert!(t.stopped(record).is_err());
            assert!(t.validate(record).is_err());
        }
    }
    #[test]
    fn absent_hit_or_nonpositive_nonfinite_kb_is_never_a_reference() {
        let mut t = Tracker::new();
        assert_eq!(
            t.stopped(Identity { kb: 0, ..first() }),
            Err(Invalid::NoHit)
        );
        t.stopped(first()).unwrap();
        for kb in [
            (-0.0_f32).to_bits(),
            (-1.0_f32).to_bits(),
            f32::NAN.to_bits(),
            f32::INFINITY.to_bits(),
        ] {
            assert_eq!(
                t.stopped(Identity { kb, ..first() }),
                Err(Invalid::Knockback)
            );
        }
        assert_eq!(Invalid::NoHit.field(), "no_positive_hit");
    }
}
