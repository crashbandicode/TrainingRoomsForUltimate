//! The "Set up…" help lines of Guard the Edge and Juggler against the wizard they open.
lib_unit_tests! {
use super::table::{Control, Editor, ROWS};
use crate::model::btt::Pad;
use crate::model::edge_juggle::{Drill, Stage, Wizard, WizardStep};
use crate::model::practice_games::Game;

fn help(game: Game) -> &'static str {
    ROWS.iter()
        .find(|r| matches!(r.control, Control::Editor(Editor::EdgeJuggleSetup(g)) if g == game))
        .expect("the Set up row")
        .help
}
fn wizard(drill: Drill, stage: Stage) -> Wizard {
    Wizard::new(drill, stage, [0.0; 2], 100, (0, 0), None).expect("a wizard")
}
fn steps(drill: Drill, stage: Stage) -> Vec<WizardStep> {
    let mut wizard = wizard(drill, stage);
    let mut seen = vec![wizard.step];
    while wizard.step != WizardStep::Preview {
        wizard.advance();
        seen.push(wizard.step);
    }
    seen
}

/// Guard the Edge's line said "Choose the start area, the CPU's point and your point". The wizard has no area step: it
/// opens on the CPU's start, where X takes the default and Y tries the other side. Found by the stale-docs pass,
/// 2026-11-27.
#[test]
fn the_set_up_help_lines_name_the_wizards_own_steps() {
    let stage = Stage::new([-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0], [[-80.0, 0.0], [80.0, 0.0]])
        .expect("a stage");
    assert_eq!(steps(Drill::GuardEdge, stage), [WizardStep::Cpu, WizardStep::Player, WizardStep::Preview]);
    assert_eq!(
        help(Game::GuardEdge),
        "Place the CPU's start, then your own, then preview both.\nOn the CPU's start, X takes the default and Y tries the other side."
    );
    assert_eq!(
        steps(Drill::Juggler, stage),
        [WizardStep::Cpu, WizardStep::Percent, WizardStep::Player, WizardStep::Preview]
    );
    assert_eq!(
        help(Game::Juggler),
        "Choose the CPU's height, position and percent, then your point.\nPreview both positions before starting."
    );
    assert!(ROWS.iter().all(|r| !r.help.contains("start area")), "no help line names a step no wizard has");

    // X and Y on the CPU's start, as the picker's own line says ("X default start · Y other side").
    let mut edge = wizard(Drill::GuardEdge, stage);
    let default = edge.setup.cpu;
    edge.steer(stage, &Pad { stick: [1.0, 0.0], ..Pad::default() });
    assert_ne!(edge.setup.cpu, default);
    edge.set_side_default(stage, false);
    assert_eq!(edge.setup.cpu, default, "X: the default start");
    edge.set_side_default(stage, true);
    assert!(default[0] < 0.0 && edge.setup.cpu[0] > 0.0, "Y: the other side's start");
    // Only there: not on a later step, and not in Juggler's wizard.
    edge.advance();
    let later = edge;
    edge.set_side_default(stage, true);
    assert_eq!(edge, later);
    let mut juggler = wizard(Drill::Juggler, stage);
    let before = juggler;
    juggler.set_side_default(stage, true);
    assert_eq!(juggler, before);
}
}
