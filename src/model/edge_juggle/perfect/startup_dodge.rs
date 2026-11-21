use super::{AirPhysics, Point};

#[derive(Clone, Copy)]
pub(crate) struct MotionSample {
    pub(crate) status: i32,
    pub(crate) motion: u64,
    pub(crate) frame: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct AttackForecast {
    pub(crate) source: MotionSample,
    pub(crate) flags: u32,
    pub(crate) bounds: [Point; 2],
    pub(crate) countdown: [i32; 2],
}

impl AttackForecast {
    /// The owned slot must describe the live motion at this exact update.
    pub(crate) fn for_motion(
        self,
        current: MotionSample,
        facing: f32,
    ) -> Option<([u8; 2], [Point; 2])> {
        let [first, last] = self.countdown;
        let [[left, low], [right, high]] = self.bounds;
        if self.flags & 0x8400_0000 != 0x8000_0000
            || self.source.status != current.status
            || current.motion == 0
            || self.source.motion != current.motion
            || !current.frame.is_finite()
            || current.frame < 0.0
            || self.source.frame != current.frame
            || !(1..=40).contains(&first)
            || !(first..=40).contains(&last)
            || !self.bounds.into_iter().flatten().all(f32::is_finite)
            || left >= right
            || low >= high
        {
            return None;
        }
        let bounds = match facing {
            1.0 => self.bounds,
            -1.0 => [[-right, low], [-left, high]],
            _ => return None,
        };
        Some(([first as u8, last as u8], bounds))
    }
}

// Fresh neutral air-dodge intangibility as measured by Ultimate Frame Data
// (ultimateframedata.com/stats#neutralairdodges), indexed by native fighter
// kind. Bat Within and Foresight are excluded. Absent kinds use [0, 0].
const NEUTRAL_INTANGIBILITY: [[u8; 2]; 94] = [
    [3, 29], // 0x00 Mario
    [4, 32], // 0x01 Donkey Kong
    [3, 25], // 0x02 Link
    [3, 29], // 0x03 Samus
    [3, 29], // 0x04 Dark Samus
    [3, 30], // 0x05 Yoshi
    [3, 29], // 0x06 Kirby
    [2, 26], // 0x07 Fox
    [2, 26], // 0x08 Pikachu
    [3, 25], // 0x09 Luigi
    [3, 30], // 0x0a Ness
    [3, 30], // 0x0b Captain Falcon
    [4, 25], // 0x0c Jigglypuff
    [3, 30], // 0x0d Peach
    [3, 30], // 0x0e Daisy
    [4, 32], // 0x0f Bowser
    [2, 26], // 0x10 Sheik
    [3, 30], // 0x11 Zelda
    [3, 29], // 0x12 Dr. Mario
    [2, 26], // 0x13 Pichu
    [2, 27], // 0x14 Falco
    [3, 29], // 0x15 Marth
    [3, 29], // 0x16 Lucina
    [2, 26], // 0x17 Young Link
    [4, 32], // 0x18 Ganondorf
    [3, 26], // 0x19 Mewtwo
    [3, 29], // 0x1a Roy
    [3, 29], // 0x1b Chrom
    [3, 28], // 0x1c Mr. Game & Watch
    [2, 26], // 0x1d Meta Knight
    [3, 29], // 0x1e Pit
    [3, 29], // 0x1f Dark Pit
    [2, 26], // 0x20 Zero Suit Samus
    [3, 29], // 0x21 Wario
    [4, 25], // 0x22 Snake
    [4, 31], // 0x23 Ike
    [2, 25], // 0x24 Squirtle
    [3, 29], // 0x25 Ivysaur
    [3, 30], // 0x26 Charizard
    [2, 27], // 0x27 Diddy Kong
    [3, 30], // 0x28 Lucas
    [2, 26], // 0x29 Sonic
    [4, 32], // 0x2a King Dedede
    [3, 30], // 0x2b Olimar
    [3, 28], // 0x2c Lucario
    [3, 27], // 0x2d R.O.B.
    [2, 26], // 0x2e Toon Link
    [2, 25], // 0x2f Wolf
    [3, 30], // 0x30 Villager
    [3, 25], // 0x31 Mega Man
    [3, 28], // 0x32 Wii Fit Trainer
    [3, 29], // 0x33 Rosalina & Luma
    [2, 26], // 0x34 Little Mac
    [2, 27], // 0x35 Greninja
    [3, 30], // 0x36 Palutena
    [3, 29], // 0x37 PAC-MAN
    [3, 30], // 0x38 Robin
    [3, 30], // 0x39 Shulk
    [3, 30], // 0x3a Bowser Jr.
    [3, 27], // 0x3b Duck Hunt
    [3, 27], // 0x3c Ryu
    [3, 27], // 0x3d Ken
    [3, 30], // 0x3e Cloud
    [3, 29], // 0x3f Corrin
    [5, 31], // 0x40 Bayonetta
    [3, 29], // 0x41 Inkling
    [3, 31], // 0x42 Ridley
    [3, 29], // 0x43 Simon
    [3, 29], // 0x44 Richter
    [4, 30], // 0x45 King K. Rool
    [3, 30], // 0x46 Isabelle
    [3, 29], // 0x47 Incineroar
    [3, 29], // 0x48 Mii Brawler
    [3, 25], // 0x49 Mii Swordfighter
    [3, 29], // 0x4a Mii Gunner
    [3, 28], // 0x4b Popo
    [3, 28], // 0x4c Nana
    [0, 0],  // 0x4d
    [0, 0],  // 0x4e
    [0, 0],  // 0x4f
    [0, 0],  // 0x50
    [3, 30], // 0x51 Piranha Plant
    [3, 30], // 0x52 Joker
    [3, 30], // 0x53 Hero
    [3, 30], // 0x54 Banjo & Kazooie
    [3, 30], // 0x55 Terry
    [3, 30], // 0x56 Byleth
    [3, 30], // 0x57 Min Min
    [3, 30], // 0x58 Steve
    [3, 30], // 0x59 Sephiroth
    [3, 30], // 0x5a Pyra
    [6, 31], // 0x5b Mythra
    [3, 30], // 0x5c Kazuya
    [3, 30], // 0x5d Sora
];

pub(crate) fn neutral_dodge_window(kind: i32) -> Option<[u8; 2]> {
    let &[start, end] = NEUTRAL_INTANGIBILITY.get(usize::try_from(kind).ok()?)?;
    // Leave one frame at each edge for the native transition and pad delivery.
    (start != 0).then(|| [start + 1, end - 1])
}

/// A bounded forecast in world units relative to the opponent's current origin.
#[derive(Clone, Copy)]
pub(crate) struct ThreatProjection {
    pub remaining_active: [u8; 2],
    pub attack: [Point; 2],
    pub torso: [Point; 2],
    pub cpu_speed: Point,
    pub attacker_speed: Point,
    pub cpu_air: AirPhysics,
    pub invulnerable: [u8; 2],
}

impl ThreatProjection {
    pub(crate) fn dodge_covers_contact(self) -> bool {
        let [first, last] = self.remaining_active;
        let [immune_from, immune_to] = self.invulnerable;
        let [lo, hi] = self.attack;
        if first == 0
            || first > last
            || last > 40
            || immune_from == 0
            || immune_from > immune_to
            || lo[0] >= hi[0]
            || lo[1] >= hi[1]
            || !self.cpu_air.valid()
            || !self
                .attack
                .into_iter()
                .chain(self.torso)
                .flatten()
                .chain(self.cpu_speed)
                .chain(self.attacker_speed)
                .all(f32::is_finite)
        {
            return false;
        }

        let immunity = immune_from..=immune_to;
        let mut motion = [0.0; 2];
        let mut cpu_vy = self.cpu_speed[1];
        let mut found = false;
        for step in 1..=last {
            cpu_vy = (cpu_vy - self.cpu_air.gravity).max(-self.cpu_air.terminal_speed);
            motion[0] += self.cpu_speed[0] - self.attacker_speed[0];
            motion[1] += cpu_vy - self.attacker_speed[1];
            if step >= first && torso_in_box(self.torso, motion, lo, hi) {
                if !immunity.contains(&step) {
                    return false;
                }
                found = true;
            }
        }
        found
    }
}

fn torso_in_box([base, top]: [Point; 2], motion: Point, lo: Point, hi: Point) -> bool {
    let (mut enter, mut exit) = (0.0_f32, 1.0_f32);
    for axis in 0..2 {
        let origin = base[axis] + motion[axis];
        let slope = top[axis] - base[axis];
        if slope == 0.0 {
            if !(lo[axis]..=hi[axis]).contains(&origin) {
                return false;
            }
            continue;
        }
        let a = (lo[axis] - origin) / slope;
        let b = (hi[axis] - origin) / slope;
        enter = enter.max(a.min(b));
        exit = exit.min(a.max(b));
    }
    enter <= exit
}

#[cfg(test)]
#[path = "startup_dodge_tests.rs"]
mod tests;
