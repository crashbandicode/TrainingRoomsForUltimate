//! The travel statuses from F13 trace boot `f13-trace-0a17-1` (REPORT.md §1). Kinds come from the game's
//! `FIGHTER_KIND_*` constants; each entry into a status is one part. F13's drill steps build on this table.

/// Minimum movement in game units before a travel's heading is known.
pub const MOVED: f32 = 1.0;

const TRAVEL: [(i32, i32); 10] = [
    (0x08, 0x1e1), // Pikachu
    (0x13, 0x1e1), // Pichu
    (0x35, 0x1e7), // Greninja
    (0x36, 0x1e1), // Palutena
    (0x11, 0x1e3), // Zelda
    (0x10, 0x1e7), // Sheik
    (0x19, 0x1e6), // Mewtwo
    (0x07, 0x1e1), // Fox
    (0x14, 0x1e2), // Falco
    (0x2f, 0x1e1), // Wolf
];

/// The fighter's travel status, if it is in the trace table.
pub fn travel_status(kind: i32) -> Option<i32> {
    TRAVEL.iter().find(|(fighter, _)| *fighter == kind).map(|(_, status)| *status)
}

/// The nearest eight-way screen direction of a movement, once it has moved far enough.
pub fn heading(dx: f32, dy: f32) -> Option<u8> {
    if dx * dx + dy * dy < MOVED * MOVED {
        return None;
    }
    let (x, y) = (dx.abs(), dy.abs());
    Some(if y <= x * 0.414_213_56 {
        if dx > 0.0 { 0 } else { 4 }
    } else if x <= y * 0.414_213_56 {
        if dy > 0.0 { 2 } else { 6 }
    } else {
        match (dx > 0.0, dy > 0.0) {
            (true, true) => 1,
            (false, true) => 3,
            (false, false) => 5,
            (true, false) => 7,
        }
    })
}
