//! Borrowed line snapshots are bound to stage/form/Training receipt lifetimes.
lib_unit_tests! {
use super::surface::{Line, Source, StageEpoch, Surfaces, Walk};
use super::Vec2;
use crate::model::cpu::role::{RoleTable, Scene};
fn epoch(roles: &RoleTable, stage: i32, form: u32) -> StageEpoch {
    StageEpoch {
        stage,
        form,
        training: roles.receipt(0).unwrap(),
    }
}
fn populated(epoch: StageEpoch) -> Surfaces {
    let mut surfaces = Surfaces::EMPTY;
    surfaces.bind(epoch);
    surfaces.begin(0x1000, 1);
    assert!(surfaces.push(Line {
        id: 0x1000,
        ends: [Vec2::default(), Vec2 { x: 20., y: 0. }],
        normal: Vec2 { x: 0., y: 1. },
        floor: true,
        wall: false,
        passable: false
    }));
    surfaces
}
struct NoOldReads;
impl Source for NoOldReads {
    fn read(&mut self, _: usize) -> Option<Line> {
        panic!("retired epoch dereferenced an old line")
    }
    fn next(&mut self, _: Line, _: usize) -> Option<(usize, usize)> {
        panic!("retired epoch followed an old neighbour")
    }
}
#[test]
fn stage_form_and_training_epoch_discard_the_entire_borrowed_walk() {
    let mut roles = RoleTable::new();
    roles.observe_scene(Some(Scene::Training));
    let original = epoch(&roles, 3, 0);
    roles.observe_scene(None);
    roles.observe_scene(Some(Scene::Training));
    let reentered = epoch(&roles, 3, 0);
    assert_ne!(
        original, reentered,
        "the same scene enum/stage/form has a new generation"
    );
    for changed in [
        StageEpoch {
            stage: 4,
            ..original
        },
        StageEpoch {
            form: 1,
            ..original
        },
        reentered,
    ] {
        let mut surfaces = populated(original);
        assert!(surfaces.bind(changed));
        assert!(surfaces.lines().next().is_none());
        assert_eq!(surfaces.state, Walk::Broken);
        assert_eq!(surfaces.step(&mut NoOldReads), Walk::Broken);
    }
}
#[test]
fn unchanged_epoch_preserves_discovery_and_begin_preserves_its_binding() {
    let mut roles = RoleTable::new();
    roles.observe_scene(Some(Scene::Training));
    let same = epoch(&roles, 3, 0);
    let mut surfaces = populated(same);
    assert!(!surfaces.bind(same));
    assert_eq!(surfaces.state, Walk::More);
    assert_eq!(surfaces.lines().next().unwrap().id, 0x1000);
}
}
