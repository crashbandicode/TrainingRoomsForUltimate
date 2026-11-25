//! Diag probe ordered by tr-primary-2, 2026-11-25 17:15:18Z.
//! On a new Training floor run or reset, logs `[StageGeometry] probe` ray and classification results.
use super::stage_geometry::RayProbe;
use crate::model::stage_geometry::Probe;
use crate::runtime::game;

static PROBE_LAST: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
static PROBE_TICK: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// After a Training reset the probe logs again where Player 1 then stands.
pub(super) fn rearm() {
    PROBE_LAST.store(0, core::sync::atomic::Ordering::Release);
}

/// Shows how the ray reads "a solid above" on a new floor run or after a reset.
pub(super) unsafe fn frame(fighter: usize) {
    use crate::model::stage_geometry::{classify, floor_run};
    use crate::model::sites_pkg2 as sites;
    use crate::runtime::diag_log::diagnostic;
    use crate::runtime::game::natives;
    use core::sync::atomic::Ordering;
    if fighter == 0 || !crate::runtime::scene::is_native_training()
        || PROBE_TICK.fetch_add(1, Ordering::Relaxed) % 30 != 0
    {
        return;
    }
    if ![sites::POS_X_OFFSET, sites::POS_Y_OFFSET, sites::SITUATION_KIND_OFFSET, sites::STAGE_ID_CALL_OFFSET,
        crate::model::sites_pkg4::FIGHTER_MANAGER_GET_ENTRY_OFFSET].into_iter().all(game::granted)
    {
        return;
    }
    let p1 = natives::fighter_accessor(fighter);
    if p1 == 0 || natives::module_accessor(0) != Some(p1) || !natives::is_grounded(p1) {
        return;
    }
    let (Some(blast), Some(mut stage)) = (super::tech_surfaces::training_blast_bounds(), RayProbe::new(p1)) else {
        return;
    };
    let at = [natives::pos_x(p1), natives::pos_y(p1)];
    let Ok(Some(run)) = floor_run(&mut stage, blast, at) else { return };
    let [left, right] = run.ends;
    let key = [left[0], left[1], right[0], right[1]]
        .into_iter()
        .fold(0x9e37_79b9_u64, |key, value| key.rotate_left(17) ^ u64::from(value.to_bits())) | 1;
    if PROBE_LAST.swap(key, Ordering::AcqRel) == key {
        return;
    }
    let [_, _, top, bottom] = blast;
    let middle = [left[0].midpoint(right[0]), left[1].midpoint(right[1])];
    diagnostic(format_args!(
        "[StageGeometry] probe stage={} p1=({:.3},{:.3}) run=({:.3},{:.3})..({:.3},{:.3}) ledges={:?}",
        natives::stage_id(), at[0], at[1], left[0], left[1], right[0], right[1], run.ledges
    ));
    for (name, sample) in [
        ("under", [middle[0], middle[1].midpoint(bottom + 1.0)]),
        ("beside", [left[0] - 1.0, left[1] - 1.0]),
        ("over", [middle[0], middle[1] + 20.0]),
    ] {
        let up = stage.first(sample, top, false);
        let class = classify(&mut stage, blast, sample);
        diagnostic(format_args!(
            "[StageGeometry] probe sample={name} at=({:.3},{:.3}) up={up:?} class={class:?}",
            sample[0], sample[1]
        ));
    }
    let back = stage.first([middle[0], middle[1] - 0.5], bottom, false);
    diagnostic(format_args!("[StageGeometry] probe back={back:?}"));
}
