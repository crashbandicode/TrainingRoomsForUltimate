//! The Loading rows' help lines (Fighters & Stages > Loading) against the code they describe.
lib_unit_tests! {
use super::table::{Control, ROWS};
use crate::engine::preload::cycle_preload_config::{Settings, BANK_MEMBERS_MAX, BANK_MEMBERS_MIN, READY_FIGHTERS_MIN};
use crate::engine::swap::body_bank::Members;

fn row(title: &str) -> &'static super::table::Row {
    ROWS.iter().find(|r| r.title == title).unwrap_or_else(|| panic!("no row {title}"))
}

/// Ready Fighter Bodies said "how many bodies a ready fighter keeps": the setting is per role and counts the fighter on
/// screen (`Settings::bank_members`). Swap groups said Pokémon Trainer and Pyra / Mythra groups both need 3 free Ready
/// Fighters: a group is placed whole, and Pyra / Mythra's has two bodies. Found by the docs pass, 2026-11-27.
#[test]
fn the_loading_help_lines_say_what_the_code_does() {
    for (title, help) in [
        ("Cycle Preload", "How many fighters per role are kept ready for swaps.\nOff loads each fighter when you swap."),
        ("Preload", "What is kept ready: fighters, Kirby hats, or both."),
        ("Ready Fighters", "The total number of fighters kept ready across roles,\nincluding the two on screen."),
        ("Ready Hats", "How many Kirby hats are kept ready, apart from the fighters.\nA hat of a ready fighter shares its body."),
        ("Ready Fighter Bodies",
            "How many fighters each role keeps in its own slot, counting\nthe one on screen. More means quicker swaps. Off: no groups."),
        ("Group Swap", "On: each role can keep extra groups beyond its own slot.\nOff: no extra groups. The two bank entries below stay as set."),
        ("Swap groups",
            "Which swaps are done as a group. A greyed entry says why.\nPT / Aegis groups need free Ready Fighters: PT 3, Aegis 2."),
    ] {
        assert_eq!(row(title).help, help, "{title}");
    }
    // Per role, the fighter on screen included: Off, or a slot of two or three. The total counts the two on screen.
    assert!(matches!(row("Ready Fighter Bodies").control, Control::Dropdown(list) if list.iter().copied().eq(["Off", "2", "3"])));
    assert_eq!((BANK_MEMBERS_MIN, BANK_MEMBERS_MAX, READY_FIGHTERS_MIN), (2, 3, 2));
    // Off leaves no group at all, whatever Group Swap says.
    assert!(Settings::DEFAULT.groups_enabled());
    assert!(!Settings { bank_members: 0, ..Settings::DEFAULT }.groups_enabled());
    // Group Swap Off: no group of any kind, and the two banks of the Swap groups row do not ask it.
    let off = Settings { group_swap: false, ..Settings::DEFAULT };
    assert!(!off.groups_enabled() && !off.native_groups_enabled() && !off.mii_groups_enabled());
    assert!(!off.solo_popo_groups_enabled() && !off.ic_pair_groups_enabled() && !off.pair_groups_enabled());
    assert!(off.pair_bank_enabled() && off.original_bank_enabled());
    assert_eq!(super::options::SWAP_GROUPS[..2], ["Ice Climbers pair bank", "Original fighter bank"]);
    // A group is placed whole when that many bodies are free (`body_bank`'s planner): Pokémon Trainer's is three
    // bodies and Pyra / Mythra's two (`Forms::starting`).
    assert_eq!((Members::native_group(0x24).map(Members::count), Members::native_group(0x5a).map(Members::count)), (Some(3), Some(2)));
}

/// "Reload developer config" said holding the chord re-reads the file every frame. The input hook only counts a request,
/// for Player 1 in Training with L, R and A all held; the menu worker reads dev.json (`model::developer_reload`,
/// `runtime::developer_reload`). Found by the stale-docs pass, 2026-11-27.
#[test]
fn the_developer_reload_help_line_says_who_reads_the_file() {
    use crate::model::chords::bit::{A, L, R};
    use crate::model::developer_reload::{qualifies, Queue};
    assert_eq!(
        row("Reload developer config").help,
        "On: hold L+R+A on Player 1 in Training to reload dev.json.\nThe reads are queued and done off the game's thread."
    );
    assert!(qualifies(0, true, L | R | A) && !qualifies(0, false, L | R | A), "in Training only");
    assert!(!qualifies(1, true, L | R | A) && !qualifies(0, true, L | R), "Player 1, the whole chord");
    // The hook's side counts a request and reads nothing: the read waits for the worker.
    let queue = Queue::<4>::new();
    queue.set_enabled(true);
    assert!(queue.request());
    let counts = queue.counts(false);
    assert_eq!((counts.requested, counts.pending, counts.completed), (1, 1, 0));
}
}