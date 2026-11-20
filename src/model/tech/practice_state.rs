//! Machine-readable view of the same Tech wizard and progression used by its HUD.

use super::{
    picker::{Picker, Spot},
    session::{Grade, Length, Phase, Session},
    surface::Kind,
    wizard::{Step, Wizard},
};
use crate::model::practice_state::Fields;

pub enum View<'a> {
    Session,
    Paused,
    Discovering,
    Wizard(&'a Wizard, Option<bool>),
    Placing(&'a Picker),
    Failed,
    Cancelled,
}

impl Grade {
    pub const fn practice_name(self) -> &'static str {
        match self {
            Self::Perfect(_) => "perfect",
            Self::Held => "held",
            Self::Early(_) => "early",
            Self::Locked(_) => "locked",
            Self::Late(_) => "late",
            Self::Missed => "missed",
            Self::AfterHitstun => "after_hitstun",
            Self::Untechable => "untechable",
            Self::TooFast(_) => "too_fast",
            Self::NoContact => "no_contact",
            Self::Unavailable => "grading_unavailable",
        }
    }
}

pub fn project_tech_state(session: &Session, view: View<'_>) -> Fields {
    let step = match session.phase {
        Phase::Off => "starting",
        Phase::Place => "place",
        Phase::Delay(_) => "delay",
        Phase::Flight => "flight",
        Phase::Hold(_) => "result",
        Phase::Done => "done",
    };
    let phase = match view {
        View::Session => step,
        View::Paused => "paused",
        View::Discovering => "discovering",
        View::Wizard(..) => "wizard",
        View::Placing(_) => "placing",
        View::Failed => "failed",
        View::Cancelled => "cancelled",
    };
    let summary = &session.summary;
    let mut fields = Fields {
        phase,
        step,
        score: Some(summary.perfect + summary.held),
        attempts: Some(summary.attempts),
        streak: Some(summary.streak),
        best: Some(summary.best),
        session_frames: Some(session.elapsed),
        remaining_frames: match session.length {
            Length::Frames(total) => Some(total.saturating_sub(session.elapsed)),
            _ => None,
        },
        last_outcome: session.result.map_or("", Grade::practice_name),
        ..Fields::default()
    };

    match view {
        View::Wizard(wizard, blocked) => {
            let (step, selection, value) = match wizard.step {
                Step::Spot => ("spot", spot_kind(wizard.selected), None),
                Step::Angle => ("angle", "angle", Some(wizard.degrees as u32)),
                Step::Distance => ("distance", "distance", Some(wizard.distance as u32)),
                Step::Hit if wizard.hit_focus => ("hit", "damage", Some(wizard.hit as u32)),
                Step::Hit => ("hit", "speed", Some(wizard.speed as u32)),
                Step::Preview => {
                    let status = match blocked {
                        Some(false) => "clear",
                        Some(true) => "blocked",
                        None => "unchecked",
                    };
                    fields.wizard_player_point = wizard.start_marker().map(|p| [p.x, p.y]);
                    ("preview", status, None)
                }
            };
            fields.wizard_step = step;
            fields.wizard_selection = selection;
            fields.wizard_value = value;
            fields.wizard_point = spot_point(wizard.selected);
        }
        View::Placing(picker) => {
            let spot = picker.hovered.or(picker.selected);
            fields.wizard_step = "spot";
            fields.wizard_selection = spot_kind(spot);
            fields.wizard_point = spot_point(spot);
        }
        _ => {}
    }
    fields
}

fn spot_kind(spot: Option<Spot>) -> &'static str {
    match spot.and_then(|spot| spot.line.kind()) {
        Some(Kind::Floor) => "floor",
        Some(Kind::Wall) => "wall",
        Some(Kind::Ceiling) => "ceiling",
        None => "",
    }
}

fn spot_point(spot: Option<Spot>) -> Option<[f32; 2]> {
    spot.map(|spot| [spot.point.x, spot.point.y])
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::tech::{
        picker::Spot,
        surface::Line,
        wizard::{Action, Outcome},
        Vec2,
    };

    #[test]
    fn tech_practice_reports_the_live_session_and_result() {
        let mut session = Session::IDLE;
        session.start(Length::Frames(200));
        assert_eq!(project_tech_state(&session, View::Session).phase, "place");
        session.placed(2);
        let held = project_tech_state(&session, View::Session);
        assert_eq!((held.phase, held.session_frames), ("delay", Some(0)));
        session.tick(false, None);
        session.tick(false, None);
        assert_eq!(project_tech_state(&session, View::Session).phase, "flight");

        session.contact_grade(Some(Grade::Perfect(Some(2))));
        let result = project_tech_state(&session, View::Session);
        assert_eq!(
            (result.phase, result.last_outcome, result.score, result.attempts),
            ("result", "perfect", Some(1), Some(1))
        );
        assert_eq!((result.streak, result.best), (Some(1), Some(1)));
        assert_eq!((result.session_frames, result.remaining_frames), (Some(2), Some(198)));
        assert_eq!(
            (project_tech_state(&session, View::Paused).phase, project_tech_state(&session, View::Paused).step),
            ("paused", "result")
        );
    }

    #[test]
    fn tech_practice_reports_actual_wizard_steps_and_selections() {
        use crate::model::{settings::Settings, tech_settings};

        let mut wizard = Wizard::begin(tech_settings::read(&Settings::factory().menu), false);
        let session = Session::IDLE;
        let fields = |wizard: &Wizard| project_tech_state(&session, View::Wizard(wizard, None));
        assert_eq!((fields(&wizard).wizard_step, fields(&wizard).wizard_point), ("spot", None));
        assert!(wizard.set_spot(Spot {
            point: Vec2 { x: 7., y: 11. },
            line: Line {
                id: 42,
                ends: [Vec2 { x: -20., y: 11. }, Vec2 { x: 20., y: 11. }],
                normal: Vec2 { x: 0., y: 1. },
                floor: true,
                wall: false,
                passable: false,
            },
        }));
        let point = fields(&wizard);
        assert_eq!((point.phase, point.wizard_selection, point.wizard_point), ("wizard", "floor", Some([7., 11.])));
        for (step, selection, value) in [
            ("angle", "angle", 90),
            ("distance", "distance", 30),
            ("hit", "speed", 100),
        ] {
            assert_eq!(wizard.action(Action::Next), Outcome::None);
            let view = fields(&wizard);
            assert_eq!((view.wizard_step, view.wizard_selection, view.wizard_value), (step, selection, Some(value)));
        }
        wizard.adjust(1., -1.);
        let view = fields(&wizard);
        assert_eq!((view.wizard_selection, view.wizard_value), ("damage", Some(13)));
        assert_eq!(wizard.action(Action::Next), Outcome::None);
        let preview = project_tech_state(&session, View::Wizard(&wizard, Some(false)));
        assert_eq!(
            (preview.wizard_step, preview.wizard_selection, preview.wizard_player_point),
            ("preview", "clear", Some([7., 41.]))
        );
    }
    }
}

