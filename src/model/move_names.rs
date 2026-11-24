use core::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU64, Ordering};

use super::frame_strip::Move;
use super::hash40::hash40;

pub const ROW_MOVE_NAMES: &str = "move_names";
const HASH_MASK: u64 = 0xff_ffff_ffff;
const VALID: u64 = 1 << 63;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadStep { Idle, Wait, Try, GiveUp }

pub fn load_step(on: bool, published: bool, arcropolis: Option<bool>, failed: bool, tries: u8, since_last_ms: u64) -> LoadStep {
    if !on || published { return LoadStep::Idle; }
    let Some(mounted) = arcropolis else { return LoadStep::GiveUp };
    if failed || tries >= 3 { return LoadStep::GiveUp; }
    if mounted && (tries == 0 || since_last_ms >= 2_000) { LoadStep::Try } else { LoadStep::Wait }
}

/// These numbers are stored by other code and in files: append only, never renumber, never above 31.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Normal {
    Unknown = 0, Jab1 = 1, Jab2 = 2, Jab3 = 3, RapidJab = 4, RapidJabFinisher = 5,
    DashAttack = 6, ForwardTilt = 7, UpTilt = 8, DownTilt = 9, ForwardSmash = 10,
    UpSmash = 11, DownSmash = 12, NeutralAir = 13, ForwardAir = 14, BackAir = 15,
    UpAir = 16, DownAir = 17, Grab = 18, DashGrab = 19, PivotGrab = 20, Pummel = 21,
    ForwardThrow = 22, BackThrow = 23, UpThrow = 24, DownThrow = 25, LedgeAttack = 26,
    GetupAttack = 27, TripAttack = 28,
}

impl Normal {
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Jab1, 2 => Self::Jab2, 3 => Self::Jab3, 4 => Self::RapidJab,
            5 => Self::RapidJabFinisher, 6 => Self::DashAttack, 7 => Self::ForwardTilt,
            8 => Self::UpTilt, 9 => Self::DownTilt, 10 => Self::ForwardSmash, 11 => Self::UpSmash,
            12 => Self::DownSmash, 13 => Self::NeutralAir, 14 => Self::ForwardAir, 15 => Self::BackAir,
            16 => Self::UpAir, 17 => Self::DownAir, 18 => Self::Grab, 19 => Self::DashGrab,
            20 => Self::PivotGrab, 21 => Self::Pummel, 22 => Self::ForwardThrow, 23 => Self::BackThrow,
            24 => Self::UpThrow, 25 => Self::DownThrow, 26 => Self::LedgeAttack,
            27 => Self::GetupAttack, 28 => Self::TripAttack, _ => Self::Unknown,
        }
    }

    pub fn of(motion: u64) -> Self {
        const MOTIONS: &[(u64, Normal)] = &[
            (hash40("attack_11"), Normal::Jab1), (hash40("attack_12"), Normal::Jab2),
            (hash40("attack_13"), Normal::Jab3), (hash40("attack_100_start"), Normal::RapidJab),
            (hash40("attack_100"), Normal::RapidJab), (hash40("attack_100_end"), Normal::RapidJabFinisher),
            (hash40("attack_dash"), Normal::DashAttack), (hash40("attack_s3_hi"), Normal::ForwardTilt),
            (hash40("attack_s3_s"), Normal::ForwardTilt), (hash40("attack_s3_lw"), Normal::ForwardTilt),
            (hash40("attack_hi3"), Normal::UpTilt), (hash40("attack_lw3"), Normal::DownTilt),
            (hash40("attack_s4_hi"), Normal::ForwardSmash), (hash40("attack_s4_s"), Normal::ForwardSmash),
            (hash40("attack_s4_lw"), Normal::ForwardSmash), (hash40("attack_s4_hold"), Normal::ForwardSmash),
            (hash40("attack_hi4"), Normal::UpSmash), (hash40("attack_hi4_hold"), Normal::UpSmash),
            (hash40("attack_lw4"), Normal::DownSmash), (hash40("attack_lw4_hold"), Normal::DownSmash),
            (hash40("attack_air_n"), Normal::NeutralAir), (hash40("attack_air_f"), Normal::ForwardAir),
            (hash40("attack_air_b"), Normal::BackAir), (hash40("attack_air_hi"), Normal::UpAir),
            (hash40("attack_air_lw"), Normal::DownAir), (hash40("catch"), Normal::Grab),
            (hash40("catch_dash"), Normal::DashGrab), (hash40("catch_turn"), Normal::PivotGrab),
            (hash40("catch_attack"), Normal::Pummel), (hash40("throw_f"), Normal::ForwardThrow),
            (hash40("throw_b"), Normal::BackThrow), (hash40("throw_hi"), Normal::UpThrow),
            (hash40("throw_lw"), Normal::DownThrow), (hash40("cliff_attack_quick"), Normal::LedgeAttack),
            (hash40("down_attack_u"), Normal::GetupAttack), (hash40("down_attack_d"), Normal::GetupAttack),
            (hash40("slip_attack"), Normal::TripAttack),
        ];
        MOTIONS.iter().find(|(hash, _)| *hash == motion).map_or(Self::Unknown, |(_, normal)| *normal)
    }

    pub const fn name(self) -> Option<&'static str> {
        Some(match self {
            Self::Unknown => return None, Self::Jab1 => "Jab 1", Self::Jab2 => "Jab 2", Self::Jab3 => "Jab 3",
            Self::RapidJab => "Rapid jab", Self::RapidJabFinisher => "Rapid jab finisher",
            Self::DashAttack => "Dash attack", Self::ForwardTilt => "Forward tilt", Self::UpTilt => "Up tilt",
            Self::DownTilt => "Down tilt", Self::ForwardSmash => "Forward smash", Self::UpSmash => "Up smash",
            Self::DownSmash => "Down smash", Self::NeutralAir => "Neutral air", Self::ForwardAir => "Forward air",
            Self::BackAir => "Back air", Self::UpAir => "Up air", Self::DownAir => "Down air", Self::Grab => "Grab",
            Self::DashGrab => "Dash grab", Self::PivotGrab => "Pivot grab", Self::Pummel => "Pummel",
            Self::ForwardThrow => "Forward throw", Self::BackThrow => "Back throw", Self::UpThrow => "Up throw",
            Self::DownThrow => "Down throw", Self::LedgeAttack => "Ledge attack", Self::GetupAttack => "Getup attack",
            Self::TripAttack => "Trip attack",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot { Neutral, Side, Up, Down, Final }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FighterId {
    pub chara: u64,
    pub specials: [u8; 4],
}

#[derive(Debug)]
pub struct Refused;

pub struct Names { bytes: Box<[u8]> }

fn u16_at(data: &[u8], at: usize) -> Result<u16, Refused> {
    let end = at.checked_add(2).ok_or(Refused)?;
    Ok(u16::from_le_bytes(data.get(at..end).ok_or(Refused)?.try_into().map_err(|_| Refused)?))
}

fn u32_at(data: &[u8], at: usize) -> Result<u32, Refused> {
    let end = at.checked_add(4).ok_or(Refused)?;
    Ok(u32::from_le_bytes(data.get(at..end).ok_or(Refused)?.try_into().map_err(|_| Refused)?))
}

fn decode_text(data: &[u8]) -> Result<String, Refused> {
    let mut at: usize = 0;
    let mut units = Vec::new();
    loop {
        let unit = u16_at(data, at)?;
        at += 2;
        match unit {
            0 => break,
            0x000e => {
                let size = usize::from(u16_at(data, at.checked_add(4).ok_or(Refused)?)?);
                at = at.checked_add(6).and_then(|at| at.checked_add(size)).ok_or(Refused)?;
                if at > data.len() { return Err(Refused); }
            }
            0x000f => {
                at = at.checked_add(4).ok_or(Refused)?;
                if at > data.len() { return Err(Refused); }
            }
            _ => units.push(unit),
        }
    }
    String::from_utf16(&units).map_err(|_| Refused)
}

fn label_key(label: &str) -> Option<(u64, Slot, u8)> {
    let (slot, variant, suffix) = if let Some(suffix) = label.strip_prefix("nam_final_arts_") {
        (Slot::Final, 0, suffix)
    } else {
        let rest = label.strip_prefix("nam_special_")?;
        let (word, rest) = rest.split_once('_')?;
        let slot = match word {
            "normal" => Slot::Neutral, "side" => Slot::Side, "upper" => Slot::Up,
            "lower" => Slot::Down, _ => return None,
        };
        let (number, suffix) = rest.split_once('_')?;
        if number.len() != 2 || !number.bytes().all(|b| b.is_ascii_digit()) { return None; }
        (slot, number.parse::<u8>().ok().filter(|number| *number < 16)?, suffix)
    };
    if suffix.is_empty() || suffix.starts_with("help_") { return None; }
    Some((hash40(&format!("ui_chara_{suffix}")) & HASH_MASK, slot, variant))
}

impl Names {
    pub fn build(msbt: &[u8]) -> Result<Self, Refused> {
        if msbt.get(..8) != Some(&b"MsgStdBn"[..]) || msbt.get(8..10) != Some(&[0xff, 0xfe][..])
            || msbt.get(0x0c) != Some(&1) || usize::try_from(u32_at(msbt, 0x12)?).map_err(|_| Refused)? != msbt.len()
        { return Err(Refused); }
        let count = usize::from(u16_at(msbt, 0x0e)?);
        let mut at = 0x20usize;
        let (mut labels, mut texts) = (None, None);
        for _ in 0..count {
            let body = at.checked_add(16).ok_or(Refused)?;
            let size = usize::try_from(u32_at(msbt, at.checked_add(4).ok_or(Refused)?)?).map_err(|_| Refused)?;
            let end = body.checked_add(size).ok_or(Refused)?;
            let section = msbt.get(body..end).ok_or(Refused)?;
            match msbt.get(at..at.checked_add(4).ok_or(Refused)?).ok_or(Refused)? {
                b"LBL1" if labels.is_none() => labels = Some(section),
                b"TXT2" if texts.is_none() => texts = Some(section),
                b"LBL1" | b"TXT2" => return Err(Refused),
                _ => {}
            }
            at = end.checked_add(15).ok_or(Refused)? & !15;
            if at > msbt.len() { return Err(Refused); }
        }
        let (labels, texts) = (labels.ok_or(Refused)?, texts.ok_or(Refused)?);
        let text_count = usize::try_from(u32_at(texts, 0)?).map_err(|_| Refused)?;
        let text_head = text_count.checked_mul(4).and_then(|n| n.checked_add(4)).ok_or(Refused)?;
        if text_head > texts.len() { return Err(Refused); }
        for i in 0..text_count {
            let start = u32_at(texts, 4 + i * 4)? as usize;
            let end = if i + 1 == text_count { texts.len() } else { u32_at(texts, 8 + i * 4)? as usize };
            if start < text_head || start > end || end > texts.len() { return Err(Refused); }
        }
        let groups = usize::try_from(u32_at(labels, 0)?).map_err(|_| Refused)?;
        let label_head = groups.checked_mul(8).and_then(|n| n.checked_add(4)).ok_or(Refused)?;
        if label_head > labels.len() { return Err(Refused); }
        let mut kept = Vec::new();
        for group in 0..groups {
            let count = u32_at(labels, 4 + group * 8)? as usize;
            let mut pos = u32_at(labels, 8 + group * 8)? as usize;
            if pos > labels.len() || (count != 0 && pos < label_head) || count > (labels.len() - pos) / 5 { return Err(Refused); }
            for _ in 0..count {
                let len = usize::from(*labels.get(pos).ok_or(Refused)?);
                pos = pos.checked_add(1).ok_or(Refused)?;
                let end = pos.checked_add(len).ok_or(Refused)?;
                let bytes = labels.get(pos..end).ok_or(Refused)?;
                if !bytes.is_ascii() { return Err(Refused); }
                let label = core::str::from_utf8(bytes).map_err(|_| Refused)?;
                let index = u32_at(labels, end)? as usize;
                pos = end.checked_add(4).ok_or(Refused)?;
                if let Some((chara, slot, variant)) = label_key(label) {
                    if index >= text_count { return Err(Refused); }
                    let start = u32_at(texts, 4 + index * 4)? as usize;
                    let end = if index + 1 == text_count { texts.len() } else { u32_at(texts, 8 + index * 4)? as usize };
                    let text = decode_text(texts.get(start..end).ok_or(Refused)?)?;
                    let mut len = text.len().min(u8::MAX as usize);
                    while !text.is_char_boundary(len) { len -= 1; }
                    kept.extend_from_slice(&chara.to_le_bytes()[..5]);
                    kept.push((slot as u8) | (variant << 4));
                    kept.push(u8::try_from(len).map_err(|_| Refused)?);
                    kept.extend_from_slice(&text.as_bytes()[..len]);
                }
            }
        }
        Ok(Self { bytes: kept.into_boxed_slice() })
    }

    pub fn special(&self, chara: u64, slot: Slot, variant: u8) -> Option<&str> {
        if variant >= 16 { return None; }
        let mut at = 0;
        while at < self.bytes.len() {
            let head = self.bytes.get(at..at + 7)?;
            let mut hash = [0; 8];
            hash[..5].copy_from_slice(&head[..5]);
            let len = usize::from(head[6]);
            let end = at.checked_add(7)?.checked_add(len)?;
            let text = self.bytes.get(at + 7..end)?;
            if u64::from_le_bytes(hash) == (chara & HASH_MASK) && head[5] == ((slot as u8) | (variant << 4)) {
                return core::str::from_utf8(text).ok();
            }
            at = end;
        }
        None
    }
    pub fn bytes(&self) -> usize { self.bytes.len() }

    #[cfg(feature = "diag")]
    pub fn entry_count(&self) -> usize {
        let mut at = 0;
        let mut count = 0;
        while at < self.bytes.len() {
            let Some(head) = self.bytes.get(at..at + 7) else { break };
            at += 7 + usize::from(head[6]);
            count += 1;
        }
        count
    }
}

#[derive(Clone, Copy)]
pub struct Name<'a>(&'a str);

impl<'a> Name<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut end = source.len().min(96);
        while !source.is_char_boundary(end) { end -= 1; }
        Self(&source[..end])
    }
    pub fn as_str(&self) -> &'a str { self.0 }
}

impl core::fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { f.write_str(self.as_str()) }
}

pub struct Published {
    on: AtomicBool,
    names: AtomicPtr<Names>,
    kind: AtomicI32,
    player: AtomicU64,
    target: AtomicU64,
}
pub static PUBLISHED: Published = Published::new();

impl Default for Published {
    fn default() -> Self { Self::new() }
}

fn pack(id: Option<FighterId>) -> u64 {
    id.map_or(0, |id| {
        id.specials.iter().enumerate().fold(VALID | (id.chara & HASH_MASK), |bits, (i, variant)| {
            bits | (u64::from((*variant).min(2)) << (40 + i * 2))
        })
    })
}

fn unpack(bits: u64) -> Option<FighterId> {
    (bits & VALID != 0).then(|| FighterId {
        chara: bits & HASH_MASK,
        specials: core::array::from_fn(|i| ((bits >> (40 + i * 2)) & 3) as u8),
    })
}

impl Published {
    pub const fn new() -> Self {
        Self { on: AtomicBool::new(false), names: AtomicPtr::new(core::ptr::null_mut()),
            kind: AtomicI32::new(-1), player: AtomicU64::new(0), target: AtomicU64::new(0) }
    }
    pub fn set_on(&self, on: bool) { self.on.store(on, Ordering::Release); }
    pub fn is_on(&self) -> bool { self.on.load(Ordering::Acquire) }
    pub fn publish(&self, names: Box<Names>) {
        let ptr = Box::into_raw(names);
        if self.names.compare_exchange(core::ptr::null_mut(), ptr, Ordering::AcqRel, Ordering::Acquire).is_err() {
            // SAFETY: publication failed, so no reader can see this allocation.
            drop(unsafe { Box::from_raw(ptr) });
        }
    }
    pub fn table(&self) -> Option<&'static Names> {
        let ptr = self.names.load(Ordering::Acquire);
        // SAFETY: the successful publication leaks the table permanently.
        unsafe { ptr.as_ref() }
    }
    pub fn set_player(&self, player: Option<(i32, FighterId)>) {
        self.kind.store(-1, Ordering::SeqCst);
        self.player.store(pack(player.map(|(_, id)| id)), Ordering::SeqCst);
        self.kind.store(player.map_or(-1, |(kind, _)| kind), Ordering::SeqCst);
    }
    pub fn player(&self) -> Option<(i32, FighterId)> {
        let before = self.kind.load(Ordering::SeqCst);
        let id = unpack(self.player.load(Ordering::SeqCst))?;
        let after = self.kind.load(Ordering::SeqCst);
        (before >= 0 && before == after).then_some((before, id))
    }
    pub fn set_target(&self, fighter: Option<FighterId>) { self.target.store(pack(fighter), Ordering::Release); }
    pub fn target(&self) -> Option<FighterId> { unpack(self.target.load(Ordering::Acquire)) }
}

pub fn recorded_identity(published: Option<(i32, FighterId)>, take_kind: i32) -> Option<FighterId> {
    published.and_then(|(kind, id)| (kind == take_kind).then_some(id))
}

#[derive(Clone, Copy)]
pub struct Namer<'a> {
    on: bool,
    names: Option<&'a Names>,
    fighter: Option<FighterId>,
}

impl<'a> Namer<'a> {
    pub const OFF: Namer<'static> = Namer { on: false, names: None, fighter: None };
    pub fn new(on: bool, names: Option<&'a Names>, fighter: Option<FighterId>) -> Self { Self { on, names, fighter } }
    pub fn live() -> Namer<'static> { Namer::new(PUBLISHED.is_on(), PUBLISHED.table(), PUBLISHED.player().map(|(_, id)| id)) }
    pub fn target() -> Namer<'static> { Namer::new(PUBLISHED.is_on(), PUBLISHED.table(), PUBLISHED.target()) }
    pub fn recorded(fighter: Option<FighterId>) -> Namer<'static> { Namer::new(PUBLISHED.is_on(), PUBLISHED.table(), fighter) }
    pub fn is_on(&self) -> bool { self.on }
    pub fn name(&self, mv: Move, normal: Normal) -> Name<'a> {
        if !self.on { return Name::new(mv.name()); }
        let special = match mv {
            Move::NeutralSpecial => Some((Slot::Neutral, 0, "Neutral special")),
            Move::SideSpecial => Some((Slot::Side, 1, "Side special")),
            Move::UpSpecial => Some((Slot::Up, 2, "Up special")),
            Move::DownSpecial => Some((Slot::Down, 3, "Down special")),
            _ => None,
        };
        if let Some((slot, at, fallback)) = special {
            let found = self.names.zip(self.fighter).and_then(|(names, id)| {
                let variant = id.specials[at];
                names.special(id.chara, slot, variant).or_else(|| (variant != 0).then(|| names.special(id.chara, slot, 0)).flatten())
            });
            return Name::new(found.unwrap_or(fallback));
        }
        Name::new(normal.name().unwrap_or(match mv {
            Move::SideTilt => "Forward tilt", Move::SideSmash => "Forward smash", _ => mv.name(),
        }))
    }
    pub fn final_smash(&self) -> Name<'a> {
        let found = self.on.then(|| self.names.zip(self.fighter).and_then(|(names, id)| names.special(id.chara, Slot::Final, 0))).flatten();
        Name::new(found.unwrap_or("Final Smash"))
    }
}

#[cfg(test)]
#[path = "move_names_tests.rs"]
mod tests;
