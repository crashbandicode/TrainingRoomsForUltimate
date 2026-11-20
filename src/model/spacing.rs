pub const ROW: &str = "spacing_grader";

use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Grade {
    Excellent,
    Good,
    Ok,
}

impl Grade {
    pub fn of(depth: f32) -> Grade {
        if depth < 1.0 / 3.0 {
            Grade::Excellent
        } else if depth < 2.0 / 3.0 {
            Grade::Good
        } else {
            Grade::Ok
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Grade::Excellent => "Excellent",
            Grade::Good => "Good",
            Grade::Ok => "OK",
        }
    }

    pub const fn color(self) -> [u8; 4] {
        match self {
            Grade::Excellent => [72, 208, 118, 255],
            Grade::Good => [244, 186, 64, 255],
            Grade::Ok => [232, 232, 236, 255],
        }
    }
}

pub fn depth(pen: f32, radius: f32) -> Option<f32> {
    if !pen.is_finite() || !radius.is_finite() || pen < 0.0 || radius <= 0.0 {
        None
    } else {
        Some(pen.min(radius) / radius)
    }
}

pub fn grade(pen: f32, radius: f32) -> Option<Grade> {
    depth(pen, radius).map(Grade::of)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lane {
    pub damage: usize,
    pub hit: usize,
    pub id: u32,
    pub attacks: bool,
}

pub struct Lanes {
    damage: [AtomicUsize; 2],
    hit: [AtomicUsize; 2],
    identity: [AtomicU64; 2],
}

impl Lanes {
    pub const fn new() -> Self {
        Self {
            damage: [const { AtomicUsize::new(0) }; 2],
            hit: [const { AtomicUsize::new(0) }; 2],
            identity: [const { AtomicU64::new(0) }; 2],
        }
    }

    pub fn set(&self, lane: usize, value: Option<Lane>) {
        let Some(damage) = self.damage.get(lane) else { return };
        damage.store(0, Ordering::Release);
        let Some(value) = value else { return };
        self.hit[lane].store(value.hit, Ordering::Relaxed);
        self.identity[lane].store(u64::from(value.id) | u64::from(value.attacks) << 32, Ordering::Relaxed);
        damage.store(value.damage, Ordering::Release);
    }

    pub fn get(&self, lane: usize) -> Option<Lane> {
        let damage = self.damage.get(lane)?.load(Ordering::Acquire);
        if damage == 0 {
            return None;
        }
        let identity = self.identity[lane].load(Ordering::Relaxed);
        let value = Lane {
            damage,
            hit: self.hit[lane].load(Ordering::Relaxed),
            id: identity as u32,
            attacks: identity >> 32 != 0,
        };
        (self.damage[lane].load(Ordering::Acquire) == damage).then_some(value)
    }
}

impl Default for Lanes {
    fn default() -> Self {
        Self::new()
    }
}

pub fn lane(read: impl Fn(usize) -> Option<u64>, accessor: usize, id: u32, attacks: bool) -> Option<Lane> {
    use crate::model::hitbox::layout;
    use crate::model::sites_hitbox::damage_record;

    if !valid_pointer(accessor) {
        return None;
    }
    Some(Lane {
        damage: read_pointer(&read, accessor.checked_add(damage_record::MODULE)?)?,
        hit: read_pointer(&read, accessor.checked_add(layout::HIT_MODULE)?)?,
        id,
        attacks,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub lane: u8,
    pub grade: Grade,
    pub earlier: bool,
}

fn pack(epoch: u32, entry: Entry) -> u64 {
    (u64::from(epoch) << 32)
        | (u64::from(u8::from(entry.earlier)) << 16)
        | (u64::from(entry.lane) << 8)
        | u64::from(entry.grade as u8)
}

fn unpack(word: u64) -> (u32, Entry) {
    (
        (word >> 32) as u32,
        Entry {
            earlier: (word >> 16) & 1 != 0,
            lane: (word >> 8) as u8,
            grade: [Grade::Excellent, Grade::Good, Grade::Ok][usize::from((word as u8).min(2))],
        },
    )
}

struct Slot {
    sequence: AtomicU64,
    word: AtomicU64,
}

impl Slot {
    const fn new() -> Self {
        Self {
            sequence: AtomicU64::new(0),
            word: AtomicU64::new(0),
        }
    }
}

pub struct Ring<const N: usize> {
    slots: [Slot; N],
    write: AtomicU64,
    read: AtomicU64,
}

impl<const N: usize> Ring<N> {
    pub const fn new() -> Self {
        Self {
            slots: [const { Slot::new() }; N],
            write: AtomicU64::new(0),
            read: AtomicU64::new(0),
        }
    }

    pub fn push(&self, epoch: u32, entry: Entry) {
        let ticket = self.write.fetch_add(1, Ordering::Relaxed);
        let slot = &self.slots[ticket as usize % N];
        slot.sequence.store(ticket.wrapping_mul(2).wrapping_add(1), Ordering::Relaxed);
        slot.word.store(pack(epoch, entry), Ordering::Relaxed);
        slot.sequence.store(ticket.wrapping_mul(2).wrapping_add(2), Ordering::Release);
    }

    pub fn drain(&self, mut f: impl FnMut(u32, Entry)) {
        let end = self.write.load(Ordering::Acquire);
        let mut next = self.read.load(Ordering::Relaxed);
        if end.wrapping_sub(next) > N as u64 {
            next = end.wrapping_sub(N as u64);
        }
        while next != end {
            let slot = &self.slots[next as usize % N];
            let sequence = next.wrapping_mul(2).wrapping_add(2);
            if slot.sequence.load(Ordering::Acquire) == sequence {
                let word = slot.word.load(Ordering::Relaxed);
                if slot.sequence.load(Ordering::Acquire) == sequence {
                    let (epoch, entry) = unpack(word);
                    f(epoch, entry);
                }
            }
            next = next.wrapping_add(1);
        }
        self.read.store(end, Ordering::Relaxed);
    }

    #[cfg(test)]
    fn tear_for_test(&self, slot: usize) {
        self.slots[slot % N].sequence.store(1, Ordering::Relaxed);
        self.write.store(1, Ordering::Relaxed);
    }
}

impl<const N: usize> Default for Ring<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Taken {
    pub hits: [Option<Grade>; 2],
    pub earlier: Option<Grade>,
}

pub struct Grader {
    ring: Ring<16>,
    epoch: AtomicU32,
    recorded: AtomicU32,
}

impl Grader {
    pub const fn new() -> Self {
        Self {
            ring: Ring::new(),
            epoch: AtomicU32::new(0),
            recorded: AtomicU32::new(u32::MAX),
        }
    }

    pub fn hit(&self, lane: u8, grade: Grade) {
        self.ring.push(self.epoch.load(Ordering::Relaxed), Entry { lane, grade, earlier: false });
    }

    pub fn target(&self, grade: Grade) {
        let epoch = self.epoch.load(Ordering::Relaxed);
        if self.recorded.load(Ordering::Acquire) == epoch {
            self.ring.push(epoch, Entry { lane: 0, grade, earlier: true });
        }
    }

    pub fn take(&self) -> Taken {
        let epoch = self.epoch.load(Ordering::Relaxed);
        let mut taken = Taken::default();
        self.ring.drain(|found, entry| {
            if found != epoch {
                return;
            }
            if entry.earlier {
                if taken.earlier.is_none() {
                    taken.earlier = Some(entry.grade);
                }
            } else if let Some(slot) = taken.hits.get_mut(usize::from(entry.lane)) {
                if slot.is_none() {
                    *slot = Some(entry.grade);
                }
            }
        });
        taken
    }

    pub fn finish(&self, recorded: bool) {
        let next = self.epoch.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        if recorded {
            self.recorded.store(next, Ordering::Release);
        }
    }

    pub fn skip(&self) {
        let _ = self.take();
        self.finish(false);
    }
}

impl Default for Grader {
    fn default() -> Self {
        Self::new()
    }
}

fn valid_pointer(pointer: usize) -> bool {
    pointer >= 0x1000 && pointer & 7 == 0 && pointer >> 47 == 0
}

fn read_pointer(read: &impl Fn(usize) -> Option<u64>, at: usize) -> Option<usize> {
    let pointer = usize::try_from(read(at)?).ok()?;
    valid_pointer(pointer).then_some(pointer)
}

fn read_u32(read: &impl Fn(usize) -> Option<u64>, at: usize) -> Option<u32> {
    Some((read(at & !7)? >> ((at & 7) * 8)) as u32)
}

fn read_u8(read: &impl Fn(usize) -> Option<u64>, at: usize) -> Option<u8> {
    Some((read(at & !7)? >> ((at & 7) * 8)) as u8)
}

fn read_f32(read: &impl Fn(usize) -> Option<u64>, at: usize) -> Option<f32> {
    read_u32(read, at).map(f32::from_bits)
}

fn read_vec3(read: &impl Fn(usize) -> Option<u64>, at: usize) -> Option<[f32; 3]> {
    Some([read_f32(read, at)?, read_f32(read, at.checked_add(4)?)?, read_f32(read, at.checked_add(8)?)?])
}

pub fn capture(
    read: impl Fn(usize) -> Option<u64>,
    owner: impl Fn(u32) -> Option<u32>,
    module: usize,
    entry: usize,
    lanes: [Option<Lane>; 2],
) -> Option<(u8, Grade)> {
    use crate::model::hitbox::layout;
    use crate::model::sites_hitbox::hurt_shape;

    if !valid_pointer(module) || !valid_pointer(entry) {
        return None;
    }
    let receiver = lanes.iter().position(|lane| lane.is_some_and(|lane| lane.damage == module))?;
    let attacker = read_u32(&read, entry.checked_add(0x24)?)?;
    let owner = match attacker >> 28 {
        0 => Some(attacker),
        1 | 4 => owner(attacker).filter(|id| id >> 28 == 0),
        _ => None,
    }?;
    let by = lanes.iter().position(|lane| lane.is_some_and(|lane| lane.id == owner && lane.attacks))?;
    if by == receiver {
        return None;
    }
    let part = read_u8(&read, entry.checked_add(0x33)?)?;
    let contact = read_vec3(&read, entry.checked_add(0x10)?)?;

    let hit_module = lanes[receiver]?.hit;
    if !valid_pointer(hit_module) {
        return None;
    }
    let total = read_u32(&read, hit_module.checked_add(layout::HIT_PART_TOTAL)?)? as i32;
    if i32::from(part) >= total {
        return None;
    }
    let parts = read_pointer(&read, hit_module.checked_add(layout::HIT_PARTS)?)?;
    let part_base = parts.checked_add(usize::from(part).checked_mul(layout::HIT_PART_STRIDE)?)?;
    let shape = read_pointer(&read, part_base.checked_add(hurt_shape::SHAPE)?)?;
    let shape_contact = read_vec3(&read, shape.checked_add(hurt_shape::CONTACT)?)?;
    if !contact.iter().zip(shape_contact).all(|(entry, shape)| entry.to_bits() == shape.to_bits()) {
        return None;
    }
    let pen = read_f32(&read, shape.checked_add(hurt_shape::PEN)?)?;
    let radius = read_f32(&read, shape.checked_add(hurt_shape::RADIUS)?)?;
    Some((by as u8, grade(pen, radius)?))
}

pub fn hook(
    live: bool,
    read: impl Fn(usize) -> Option<u64>,
    owner: impl Fn(u32) -> Option<u32>,
    module: usize,
    entry: usize,
    lanes: [Option<Lane>; 2],
) -> Option<(u8, Grade)> {
    live.then(|| capture(read, owner, module, entry, lanes)).flatten()
}

pub fn zone_discs(target: &crate::model::hitbox_orbs::Orb) -> [crate::model::hitbox_orbs::Orb; 2] {
    let mut good = *target;
    good.radius = 8.0 / 3.0;
    good.color = crate::model::panes::linear([244, 186, 64, 255]);
    let mut ok = *target;
    ok.radius = 4.0 / 3.0;
    ok.color = crate::model::panes::linear([232, 232, 236, 255]);
    [good, ok]
}

pub const PICTURE_ALPHA: u8 = 64;

/// The target pictures' panes sit above the orb panes, so while grading a whole target is drawn see-through to show its zone
/// discs; break animations and other looks are unchanged.
pub fn picture_look(look: crate::model::panes::EggLook, grader: bool) -> crate::model::panes::EggLook {
    if grader && look == crate::model::panes::EGG_WHOLE {
        crate::model::panes::EggLook { alpha: PICTURE_ALPHA, ..look }
    } else {
        look
    }
}

#[cfg(test)]
#[path = "spacing_tests.rs"]
mod tests;
