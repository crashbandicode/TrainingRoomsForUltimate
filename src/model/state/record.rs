//! One saved fighter state and its JSON (SPEC-state-01 §7.2, SPEC-state-02 §7, SPEC-state-05 §7): the
//! values a Save captures, the step marker of the load machine, the defaults of a never-saved record.
//!
//! The value types (`Charge`, `Robin`, `Steve`) are the fork's `practice_setup::{ChargeState,
//! RobinCharges, SteveState}` (stage m5o, ours-new; CARRY-OVER.md) with their `validate` rules and JSON
//! shape, read and written by hand instead of through serde derive. The Mii identity (`mii`) is an opaque
//! JSON value kept as it was read until package 6 gives it a type: it is written back unchanged.

use serde_json::{Map, Value};

use super::held_item::HeldItem;
use super::json::Writer;

/// The step marker of the load machine (SPEC-state-01 §5). Runtime only: it is written to the file as
/// part of the record and forced back to `NoAction` when the file is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Marker {
    Save,
    NoAction,
    KillPlayer,
    WaitForAlive,
    PosMove,
    ApplyBuff,
    NanaPosMove,
    WaitForPokemonSwitch,
    WaitForCopyAbility,
}

impl Marker {
    pub const fn name(self) -> &'static str {
        match self {
            Marker::Save => "Save",
            Marker::NoAction => "NoAction",
            Marker::KillPlayer => "KillPlayer",
            Marker::WaitForAlive => "WaitForAlive",
            Marker::PosMove => "PosMove",
            Marker::ApplyBuff => "ApplyBuff",
            Marker::NanaPosMove => "NanaPosMove",
            Marker::WaitForPokemonSwitch => "WaitForPokemonSwitch",
            Marker::WaitForCopyAbility => "WaitForCopyAbility",
        }
    }
}

/// Robin's tomes and Levin Sword (SPEC-state-02 §4).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Robin {
    pub thunder_kind: i32,
    pub yellow_book: i32,
    pub red_book: f32,
    pub green_book: i32,
    pub purple_book: i32,
    pub levin_charge: i32,
    pub levin_recharge: i32,
    pub has_levin: bool,
}

/// The special-move charge values a Save captures. The meaning of each slot depends on the fighter
/// (SPEC-state-02 §4).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Charge {
    pub int_x: Option<i32>,
    pub int_y: Option<i32>,
    pub int_z: Option<i32>,
    pub float_x: Option<f32>,
    pub float_y: Option<f32>,
    pub float_z: Option<f32>,
    pub has_charge: Option<bool>,
    pub robin: Option<Robin>,
}

impl Charge {
    /// Finite values only; the integer meanings are per fighter.
    pub fn validate(&self) -> Result<(), &'static str> {
        for (value, why) in [
            (self.float_x, "charge float_x is not finite"),
            (self.float_y, "charge float_y is not finite"),
            (self.float_z, "charge float_z is not finite"),
        ] {
            if value.is_some_and(|v| !v.is_finite()) {
                return Err(why);
            }
        }
        match &self.robin {
            Some(robin) if !robin.red_book.is_finite() => Err("Robin red book charge is not finite"),
            _ => Ok(()),
        }
    }
}

/// Sanity bound for Steve tool material kinds (`FighterPickelMaterialKind`).
pub const MAX_STEVE_MATERIAL_KIND: u32 = 15;

/// Steve's material counts and tool records. The `*_mat` fields hold a native
/// `FighterPickelMaterialKind` as a code point (the fork stored a `char`, written as a one-code-point
/// string).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Steve {
    pub mat_g1: i32,
    pub mat_wood: i32,
    pub mat_stone: i32,
    pub mat_iron: i32,
    pub mat_gold: i32,
    pub mat_redstone: i32,
    pub mat_diamond: i32,
    pub sword_mat: u32,
    pub sword_durability: f32,
    pub axe_mat: u32,
    pub axe_durability: f32,
    pub pick_mat: u32,
    pub pick_durability: f32,
    pub shovel_mat: u32,
    pub shovel_durability: f32,
}

impl Steve {
    /// Steve's starting kit (SPEC-state-05 §7).
    pub const START: Steve = Steve {
        mat_g1: 36,
        mat_wood: 18,
        mat_stone: 0,
        mat_iron: 3,
        mat_gold: 0,
        mat_redstone: 2,
        mat_diamond: 0,
        sword_mat: 1,
        sword_durability: 25.0,
        axe_mat: 1,
        axe_durability: 70.0,
        pick_mat: 1,
        pick_durability: 70.0,
        shovel_mat: 1,
        shovel_durability: 70.0,
    };

    /// Non-negative counts, bounded tool kinds, finite non-negative durability.
    pub fn validate(&self) -> Result<(), &'static str> {
        let counts = [
            self.mat_g1,
            self.mat_wood,
            self.mat_stone,
            self.mat_iron,
            self.mat_gold,
            self.mat_redstone,
            self.mat_diamond,
        ];
        if counts.iter().any(|count| *count < 0) {
            return Err("Steve material count is negative");
        }
        for (material, durability) in [
            (self.sword_mat, self.sword_durability),
            (self.axe_mat, self.axe_durability),
            (self.pick_mat, self.pick_durability),
            (self.shovel_mat, self.shovel_durability),
        ] {
            if material > MAX_STEVE_MATERIAL_KIND {
                return Err("Steve tool material kind is out of range");
            }
            if !durability.is_finite() || durability < 0.0 {
                return Err("Steve tool durability is not a finite non-negative number");
            }
        }
        Ok(())
    }
}

/// One saved fighter state (the player's or the CPU's half of a slot).
#[derive(Clone, Debug, PartialEq)]
pub struct SavedState {
    pub x: f32,
    pub y: f32,
    pub percent: f32,
    /// Facing: `1.0` right, `-1.0` left.
    pub lr: f32,
    /// The game's situation constant (ground / air / cliff / other).
    pub situation_kind: i32,
    pub marker: Marker,
    /// The saved fighter kind, `-1` when never saved.
    pub fighter_kind: i32,
    pub charge: Charge,
    pub steve: Option<Steve>,
    pub held_item: Option<HeldItem>,
    pub saved_stage: Option<i32>,
    /// Package 6 gives this a type; it is kept as read.
    pub mii: Option<Value>,
}

impl SavedState {
    /// A never-saved record (SPEC-state-01 §7.3).
    pub fn never_saved() -> Self {
        SavedState {
            x: 0.0,
            y: 0.0,
            percent: 0.0,
            lr: 1.0,
            situation_kind: 0,
            marker: Marker::NoAction,
            fighter_kind: -1,
            charge: Charge::default(),
            steve: Some(Steve::START),
            held_item: None,
            saved_stage: None,
            mii: None,
        }
    }

    /// Whether the record has ever been saved.
    pub fn is_saved(&self) -> bool {
        self.fighter_kind >= 0
    }

    /// Read one record. Every field is optional: a missing or mistyped field takes its default, an
    /// unknown key is ignored, a damaged `held_item` becomes `Unsupported`, a damaged `steve_state`
    /// is dropped. The marker is always `NoAction` (a persisted in-progress marker must never resume a
    /// load in a new process).
    pub fn from_json(value: &Value) -> Self {
        let mut record = Self::never_saved();
        let Some(object) = value.as_object() else { return record };
        let float = |key: &str, default: f32| object.get(key).and_then(Value::as_f64).map_or(default, |v| v as f32);
        let int = |key: &str, default: i32| {
            object.get(key).and_then(Value::as_i64).and_then(|v| i32::try_from(v).ok()).unwrap_or(default)
        };
        record.x = float("x", 0.0);
        record.y = float("y", 0.0);
        record.percent = float("percent", 0.0);
        record.lr = float("lr", 1.0);
        record.situation_kind = int("situation_kind", 0);
        record.fighter_kind = int("fighter_kind", -1);
        record.charge = object.get("charge").map(charge_from_json).unwrap_or_default();
        record.steve = object.get("steve_state").and_then(steve_from_json);
        record.held_item = object.get("held_item").and_then(HeldItem::from_json);
        record.saved_stage = object.get("saved_stage").and_then(Value::as_i64).and_then(|v| i32::try_from(v).ok());
        record.mii = object.get("mii").filter(|v| !v.is_null()).cloned();
        record
    }

    /// The record's JSON in the fork's field order; absent options are `null`, as the fork writes them.
    pub fn write_json(&self, out: &mut Writer) {
        out.begin_object();
        out.field_f32("x", self.x);
        out.field_f32("y", self.y);
        out.field_f32("percent", self.percent);
        out.field_f32("lr", self.lr);
        out.field_int("situation_kind", i64::from(self.situation_kind));
        out.field_str("state", self.marker.name());
        out.field_int("fighter_kind", i64::from(self.fighter_kind));
        out.key("charge");
        write_charge(&self.charge, out);
        out.key("steve_state");
        match &self.steve {
            Some(steve) => write_steve(steve, out),
            None => out.raw("null"),
        }
        out.key("held_item");
        match self.held_item {
            Some(item) => item.write_json(out),
            None => out.raw("null"),
        }
        match self.saved_stage {
            Some(stage) => out.field_int("saved_stage", i64::from(stage)),
            None => out.field_null("saved_stage"),
        }
        out.key("mii");
        match &self.mii {
            Some(mii) => out.value(mii),
            None => out.raw("null"),
        }
        out.end_object();
    }
}

fn opt_int(object: &Map<String, Value>, key: &str) -> Option<i32> {
    object.get(key).and_then(Value::as_i64).and_then(|v| i32::try_from(v).ok())
}

fn opt_float(object: &Map<String, Value>, key: &str) -> Option<f32> {
    object.get(key).and_then(Value::as_f64).map(|v| v as f32)
}

pub fn charge_from_json(value: &Value) -> Charge {
    let Some(object) = value.as_object() else { return Charge::default() };
    Charge {
        int_x: opt_int(object, "int_x"),
        int_y: opt_int(object, "int_y"),
        int_z: opt_int(object, "int_z"),
        float_x: opt_float(object, "float_x"),
        float_y: opt_float(object, "float_y"),
        float_z: opt_float(object, "float_z"),
        has_charge: object.get("has_charge").and_then(Value::as_bool),
        robin: object.get("robin_charges").and_then(robin_from_json),
    }
}

fn robin_from_json(value: &Value) -> Option<Robin> {
    let object = value.as_object()?;
    Some(Robin {
        thunder_kind: opt_int(object, "thunder_kind")?,
        yellow_book: opt_int(object, "yellow_book")?,
        red_book: opt_float(object, "red_book")?,
        green_book: opt_int(object, "green_book")?,
        purple_book: opt_int(object, "purple_book")?,
        levin_charge: opt_int(object, "levin_charge")?,
        levin_recharge: opt_int(object, "levin_recharge")?,
        has_levin: object.get("has_levin").and_then(Value::as_bool)?,
    })
}

pub fn steve_from_json(value: &Value) -> Option<Steve> {
    let object = value.as_object()?;
    let mat = |key: &str| -> Option<u32> {
        let text = object.get(key)?.as_str()?;
        let mut chars = text.chars();
        let first = chars.next()?;
        chars.next().is_none().then_some(first as u32)
    };
    let steve = Steve {
        mat_g1: opt_int(object, "mat_g1")?,
        mat_wood: opt_int(object, "mat_wood")?,
        mat_stone: opt_int(object, "mat_stone")?,
        mat_iron: opt_int(object, "mat_iron")?,
        mat_gold: opt_int(object, "mat_gold")?,
        mat_redstone: opt_int(object, "mat_redstone")?,
        mat_diamond: opt_int(object, "mat_diamond")?,
        sword_mat: mat("sword_mat")?,
        sword_durability: opt_float(object, "sword_durability")?,
        axe_mat: mat("axe_mat")?,
        axe_durability: opt_float(object, "axe_durability")?,
        pick_mat: mat("pick_mat")?,
        pick_durability: opt_float(object, "pick_durability")?,
        shovel_mat: mat("shovel_mat")?,
        shovel_durability: opt_float(object, "shovel_durability")?,
    };
    // The fork bounds material kinds on import; the values are written into native tool records.
    steve.validate().ok().map(|()| steve)
}

fn write_opt_int(out: &mut Writer, key: &str, value: Option<i32>) {
    match value {
        Some(value) => out.field_int(key, i64::from(value)),
        None => out.field_null(key),
    }
}

fn write_opt_float(out: &mut Writer, key: &str, value: Option<f32>) {
    match value {
        Some(value) => out.field_f32(key, value),
        None => out.field_null(key),
    }
}

pub fn write_charge(charge: &Charge, out: &mut Writer) {
    out.begin_object();
    write_opt_int(out, "int_x", charge.int_x);
    write_opt_int(out, "int_y", charge.int_y);
    write_opt_int(out, "int_z", charge.int_z);
    write_opt_float(out, "float_x", charge.float_x);
    write_opt_float(out, "float_y", charge.float_y);
    write_opt_float(out, "float_z", charge.float_z);
    match charge.has_charge {
        Some(value) => out.field_bool("has_charge", value),
        None => out.field_null("has_charge"),
    }
    out.key("robin_charges");
    match &charge.robin {
        Some(robin) => {
            out.begin_object();
            out.field_int("thunder_kind", i64::from(robin.thunder_kind));
            out.field_int("yellow_book", i64::from(robin.yellow_book));
            out.field_f32("red_book", robin.red_book);
            out.field_int("green_book", i64::from(robin.green_book));
            out.field_int("purple_book", i64::from(robin.purple_book));
            out.field_int("levin_charge", i64::from(robin.levin_charge));
            out.field_int("levin_recharge", i64::from(robin.levin_recharge));
            out.field_bool("has_levin", robin.has_levin);
            out.end_object();
        }
        None => out.raw("null"),
    }
    out.end_object();
}

pub fn write_steve(steve: &Steve, out: &mut Writer) {
    out.begin_object();
    out.field_int("mat_g1", i64::from(steve.mat_g1));
    out.field_int("mat_wood", i64::from(steve.mat_wood));
    out.field_int("mat_stone", i64::from(steve.mat_stone));
    out.field_int("mat_iron", i64::from(steve.mat_iron));
    out.field_int("mat_gold", i64::from(steve.mat_gold));
    out.field_int("mat_redstone", i64::from(steve.mat_redstone));
    out.field_int("mat_diamond", i64::from(steve.mat_diamond));
    for (key, material, durability_key, durability) in [
        ("sword_mat", steve.sword_mat, "sword_durability", steve.sword_durability),
        ("axe_mat", steve.axe_mat, "axe_durability", steve.axe_durability),
        ("pick_mat", steve.pick_mat, "pick_durability", steve.pick_durability),
        ("shovel_mat", steve.shovel_mat, "shovel_durability", steve.shovel_durability),
    ] {
        let text = char::from_u32(material).map(String::from).unwrap_or_default();
        out.field_str(key, &text);
        out.field_f32(durability_key, durability);
    }
    out.end_object();
}
