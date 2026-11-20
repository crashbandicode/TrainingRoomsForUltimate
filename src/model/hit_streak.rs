//! An attack is one fighter motion plus its live damaging articles.

use core::fmt::{self, Write};

#[derive(Clone, Copy)]
pub struct Motion {
    pub status: i32,
    pub kind: u64,
    pub frame: f32,
}

impl Motion {
    const EMPTY: Self = Self { status: 0, kind: 0, frame: 0.0 };

    fn follows(self, before: Self) -> bool {
        self.status == before.status && self.kind == before.kind
            && !(self.frame <= 1.0 && before.frame >= 2.0)
    }
}

// The owned pool has 64 entries. Leave room for ArticleModule instances alongside it.
const WEAPONS: usize = 128;

#[derive(Clone, Copy)]
struct Projectile {
    id: u32,
    motion: u32,
    seen: bool,
    attack: bool,
    hit: bool,
}

impl Projectile {
    const EMPTY: Self = Self { id: 0, motion: 0, seen: false, attack: false, hit: false };
}

struct Body {
    owner: Option<u32>,
    motion: Motion,
    serial: u32,
    attack: bool,
    hit: bool,
    weapons: [Projectile; WEAPONS],
}

impl Body {
    const EMPTY: Self = Self {
        owner: None,
        motion: Motion::EMPTY,
        serial: 1,
        attack: false,
        hit: false,
        weapons: [Projectile::EMPTY; WEAPONS],
    };

    fn has_weapon(&self, serial: u32) -> bool {
        self.weapons.iter().any(|weapon| weapon.motion == serial)
    }

    fn begin(&mut self, motion: Motion, attack: bool) -> bool {
        let mut missed = false;
        if !motion.follows(self.motion) {
            missed = self.attack && !self.hit && !self.has_weapon(self.serial);
            self.serial = self.serial.wrapping_add(1).max(1);
            (self.attack, self.hit) = (false, false);
        }
        self.motion = motion;
        if attack && !self.attack {
            self.arm(self.serial);
        }
        missed
    }

    fn arm(&mut self, serial: u32) {
        if serial == self.serial {
            self.attack = true;
        }
        for weapon in self.weapons.iter_mut().filter(|weapon| weapon.motion == serial) {
            weapon.attack = true;
        }
    }

    fn expired_miss(&mut self) -> bool {
        let mut missed = false;
        for i in 0..self.weapons.len() {
            let weapon = self.weapons[i];
            if weapon.motion == 0 {
                continue;
            }
            if weapon.seen {
                self.weapons[i].seen = false;
            } else {
                self.weapons[i] = Projectile::EMPTY;
                missed |= weapon.attack && !weapon.hit && weapon.motion != self.serial && !self.has_weapon(weapon.motion);
            }
        }
        missed
    }

    fn credit(&mut self, source: u32) -> bool {
        let serial = if self.owner == Some(source) {
            if !self.attack || self.hit {
                return false;
            }
            self.serial
        } else {
            let Some(weapon) = self.weapons.iter().find(|weapon| weapon.motion != 0 && weapon.id == source) else {
                return false;
            };
            if weapon.hit {
                return false;
            }
            weapon.motion
        };
        if serial == self.serial {
            self.hit = true;
        }
        for weapon in self.weapons.iter_mut().filter(|weapon| weapon.motion == serial) {
            weapon.hit = true;
        }
        true
    }
}

pub struct Streak {
    enabled: bool,
    current: u32,
    best: u32,
    bodies: [Body; 2],
}

impl Streak {
    pub const EMPTY: Self = Self {
        enabled: false,
        current: 0,
        best: 0,
        bodies: [Body::EMPTY; 2],
    };

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Fresh round (or Off). The best stays with the game for the session.
    pub fn restart(&mut self, enabled: bool) {
        for body in &mut self.bodies {
            *body = Body::EMPTY;
        }
        (self.enabled, self.current) = (enabled, 0);
    }

    pub fn counts(&self) -> (u32, u32) {
        (self.current, self.best)
    }

    pub fn write(&self, out: &mut impl Write) -> fmt::Result {
        if self.enabled {
            write!(out, "  Streak {} (best {})", self.current, self.best)?;
        }
        Ok(())
    }

    /// Begin a sample for one of P1's bodies. A new motion closes the fighter's portion of its old attack.
    pub fn begin(&mut self, owner: u32, motion: Motion, attack: bool) -> usize {
        let index = self.bodies.iter().position(|body| body.owner == Some(owner))
            .or_else(|| self.bodies.iter().position(|body| body.owner.is_none()))
            .unwrap_or(0);
        if !self.enabled {
            return index;
        }
        let body = &mut self.bodies[index];
        if body.owner != Some(owner) {
            *body = Body::EMPTY;
            body.owner = Some(owner);
        }
        if body.begin(motion, attack) {
            self.current = 0;
        }
        index
    }

    /// Attribute a weapon to the move that spawned it. Its first damaging frame can come after that motion ends.
    /// A weapon without a damaging frame opens no attempt by itself.
    pub fn projectile(&mut self, index: usize, id: u32, attacking: impl FnOnce() -> bool) {
        if !self.enabled {
            return;
        }
        let body = &mut self.bodies[index];
        let index = body.weapons.iter().position(|weapon| weapon.motion != 0 && weapon.id == id)
            .or_else(|| body.weapons.iter().position(|weapon| weapon.motion == 0));
        let Some(index) = index else { return };
        let weapon = &mut body.weapons[index];
        if weapon.motion == 0 {
            *weapon = Projectile { id, motion: body.serial, seen: true, attack: body.attack, hit: body.hit };
        }
        weapon.seen = true;
        let serial = weapon.motion;
        if !weapon.attack && attacking() {
            body.arm(serial);
        }
    }

    pub fn end(&mut self, index: usize) {
        if self.enabled && self.bodies[index].expired_miss() {
            self.current = 0;
        }
    }

    /// A scored target hit, using the actual damaging object and its P1 owner.
    pub fn hit(&mut self, owner: u32, source: u32) {
        if self.enabled && self.bodies.iter_mut().find(|body| body.owner == Some(owner)).is_some_and(|body| body.credit(source)) {
            self.current = self.current.saturating_add(1);
            self.best = self.best.max(self.current);
        }
    }
}

#[cfg(test)]
#[path = "hit_streak_tests.rs"]
mod tests;
