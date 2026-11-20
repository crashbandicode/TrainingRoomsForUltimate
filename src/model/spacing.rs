pub const ROW: &str = "spacing_grader";

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

pub fn native(record_p: [f32; 3], shape_p: [f32; 3], pen: f32, radius: f32) -> Option<Grade> {
    let same_contact = record_p
        .iter()
        .zip(shape_p.iter())
        .all(|(record, shape)| record.to_bits() == shape.to_bits());
    if same_contact {
        grade(pen, radius)
    } else {
        None
    }
}

pub fn read_contact(read: impl Fn(usize) -> Option<u64>, accessor: usize) -> Option<Grade> {
    use crate::model::hitbox::layout;
    use crate::model::sites_hitbox::{damage_record as record_layout, hurt_shape as shape_layout};

    fn pointer(read: &impl Fn(usize) -> Option<u64>, at: usize) -> Option<usize> {
        let pointer = usize::try_from(read(at)?).ok()?;
        (pointer >= 0x1000 && pointer & 7 == 0 && pointer >> 47 == 0).then_some(pointer)
    }

    fn field_u32(read: &impl Fn(usize) -> Option<u64>, at: usize) -> Option<u32> {
        let word = read(at & !7)?;
        Some((word >> ((at & 7) * 8)) as u32)
    }

    fn field_f32(read: &impl Fn(usize) -> Option<u64>, base: usize, offset: usize) -> Option<f32> {
        let at = base.checked_add(offset)?;
        field_u32(read, at).map(f32::from_bits)
    }

    let record_module = pointer(&read, accessor.checked_add(record_layout::MODULE)?)?;
    let record = pointer(&read, record_module.checked_add(record_layout::RECORDS)?)?;
    let part_at = record.checked_add(record_layout::PART)?;
    let part = (field_u32(&read, part_at)? & 0xff) as usize;

    let hit_module = pointer(&read, accessor.checked_add(layout::HIT_MODULE)?)?;
    let total_at = hit_module.checked_add(layout::HIT_PART_TOTAL)?;
    let total = usize::try_from(field_u32(&read, total_at)? as i32).ok()?;
    if part >= total {
        return None;
    }

    let parts = pointer(&read, hit_module.checked_add(layout::HIT_PARTS)?)?;
    let part_base = parts.checked_add(part.checked_mul(layout::HIT_PART_STRIDE)?)?;
    let shape = pointer(&read, part_base.checked_add(shape_layout::SHAPE)?)?;

    let record_contact = [
        field_f32(&read, record, record_layout::CONTACT)?,
        field_f32(&read, record, record_layout::CONTACT + 4)?,
        field_f32(&read, record, record_layout::CONTACT + 8)?,
    ];
    let shape_contact = [
        field_f32(&read, shape, shape_layout::CONTACT)?,
        field_f32(&read, shape, shape_layout::CONTACT + 4)?,
        field_f32(&read, shape, shape_layout::CONTACT + 8)?,
    ];
    let pen = field_f32(&read, shape, shape_layout::PEN)?;
    let radius = field_f32(&read, shape, shape_layout::RADIUS)?;
    native(record_contact, shape_contact, pen, radius)
}

#[cfg(test)]
#[path = "spacing_tests.rs"]
mod tests;
