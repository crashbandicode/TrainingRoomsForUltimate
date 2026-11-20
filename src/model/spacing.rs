pub const ROW: &str = "spacing_grader";

use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    pub attacker: u32,
    pub receiver: u8,
    pub slot: u8,
    pub part: u8,
    pub pen: f32,
    pub radius: f32,
}

impl Contact {
    pub fn grade(&self) -> Option<Grade> {
        grade(self.pen, self.radius)
    }
}

fn pack(epoch: u32, contact: &Contact) -> [u64; 3] {
    [
        (u64::from(epoch) << 32) | u64::from(contact.attacker),
        (u64::from(contact.pen.to_bits()) << 32)
            | (u64::from(contact.receiver) << 24)
            | (u64::from(contact.slot) << 16)
            | (u64::from(contact.part) << 8),
        u64::from(contact.radius.to_bits()),
    ]
}

fn unpack(words: [u64; 3]) -> (u32, Contact) {
    let [identity, fields, radius] = words;
    (
        (identity >> 32) as u32,
        Contact {
            attacker: identity as u32,
            receiver: (fields >> 24) as u8,
            slot: (fields >> 16) as u8,
            part: (fields >> 8) as u8,
            pen: f32::from_bits((fields >> 32) as u32),
            radius: f32::from_bits(radius as u32),
        },
    )
}

struct Slot {
    sequence: AtomicU64,
    _words: [AtomicU64; 3],
}

impl Slot {
    const fn new() -> Self {
        Self {
            sequence: AtomicU64::new(0),
            _words: [const { AtomicU64::new(0) }; 3],
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

    pub fn push(&self, epoch: u32, contact: Contact) {
        let ticket = self.write.fetch_add(1, Ordering::Relaxed);
        let slot = &self.slots[ticket as usize % N];
        slot.sequence.store(ticket.wrapping_mul(2).wrapping_add(1), Ordering::Relaxed);
        for (word, value) in slot._words.iter().zip(pack(epoch, &contact)) {
            word.store(value, Ordering::Relaxed);
        }
        slot.sequence.store(ticket.wrapping_mul(2).wrapping_add(2), Ordering::Release);
    }

    pub fn drain(&self, mut f: impl FnMut(u32, Contact)) {
        let end = self.write.load(Ordering::Acquire);
        let mut next = self.read.load(Ordering::Relaxed);
        if end.wrapping_sub(next) > N as u64 {
            next = end.wrapping_sub(N as u64);
        }
        while next != end {
            let slot = &self.slots[next as usize % N];
            let sequence = next.wrapping_mul(2).wrapping_add(2);
            if slot.sequence.load(Ordering::Acquire) == sequence {
                let words = slot._words.each_ref().map(|word| word.load(Ordering::Relaxed));
                if slot.sequence.load(Ordering::Acquire) == sequence {
                    let (epoch, contact) = unpack(words);
                    f(epoch, contact);
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

pub struct Rings {
    pub received: [Ring<8>; 2],
    pub targets: Ring<8>,
}

impl Rings {
    pub const fn new() -> Self {
        Self {
            received: [const { Ring::new() }; 2],
            targets: Ring::new(),
        }
    }

    pub fn take(&self, epoch: u32) -> Taken {
        Taken {
            received: [first(&self.received[0], epoch), first(&self.received[1], epoch)],
            target: first(&self.targets, epoch),
        }
    }
}

impl Default for Rings {
    fn default() -> Self {
        Self::new()
    }
}

fn first(ring: &Ring<8>, epoch: u32) -> Option<Grade> {
    let mut grade = None;
    ring.drain(|found_epoch, contact| {
        if grade.is_none() && found_epoch == epoch {
            grade = contact.grade();
        }
    });
    grade
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Taken {
    pub received: [Option<Grade>; 2],
    pub target: Option<Grade>,
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

pub fn capture(read: impl Fn(usize) -> Option<u64>, module: usize, entry: usize, lanes: [usize; 2]) -> Option<Contact> {
    use crate::model::hitbox::layout;
    use crate::model::sites_hitbox::{damage_record, hurt_shape};

    if !valid_pointer(module) || !valid_pointer(entry) {
        return None;
    }
    let receiver = lanes.iter().enumerate().find_map(|(lane, &accessor)| {
        if !valid_pointer(accessor) {
            return None;
        }
        let at = accessor.checked_add(damage_record::MODULE)?;
        (read_pointer(&read, at)? == module).then_some(lane as u8)
    })?;

    let attacker = read_u32(&read, entry.checked_add(0x24)?)?;
    let slot = read_u8(&read, entry.checked_add(0x32)?)?;
    let part = read_u8(&read, entry.checked_add(0x33)?)?;
    let contact = read_vec3(&read, entry.checked_add(0x10)?)?;

    let hit_module = read_pointer(&read, lanes[receiver as usize].checked_add(layout::HIT_MODULE)?)?;
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
    let contact = Contact {
        attacker,
        receiver,
        slot,
        part,
        pen: read_f32(&read, shape.checked_add(hurt_shape::PEN)?)?,
        radius: read_f32(&read, shape.checked_add(hurt_shape::RADIUS)?)?,
    };
    contact.grade()?;
    Some(contact)
}

pub fn hook(
    live: bool,
    read: impl Fn(usize) -> Option<u64>,
    module: usize,
    entry: usize,
    lanes: [usize; 2],
) -> Option<Contact> {
    live.then(|| capture(read, module, entry, lanes)).flatten()
}

pub fn zone_discs(target: &crate::model::hitbox_orbs::Orb) -> [crate::model::hitbox_orbs::Orb; 2] {
    let mut good = *target;
    good.radius = 8.0 / 3.0;
    good.color = [244, 186, 64, 96];
    let mut ok = *target;
    ok.radius = 4.0 / 3.0;
    ok.color = [232, 232, 236, 96];
    [good, ok]
}

#[cfg(test)]
#[path = "spacing_tests.rs"]
mod tests;
