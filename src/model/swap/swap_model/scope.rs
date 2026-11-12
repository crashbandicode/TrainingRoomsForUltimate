//! Scope and read-only contract per callback (WP-E, WORKPLAN §2.6d; ROWS-ANALYSIS row 17).
//!
//! Every per-frame or seam callback declares where it may run (`Scope`) and whether it writes
//! simulated state (`read_only`). The rule (`admitted`): a read-only callback runs wherever it
//! declares; a mutating one runs only in Training or a waiting room, unless its row names a
//! reviewed exception (`Exception`) that says why a match is safe. The Switch dispatcher
//! (`src/swap_native/scope_gate.rs`) applies the rule; this table is the review made mechanical.
//! Hooks are covered by `native_seams::Seam::{scopes, read_only}` (one row per site); this table
//! holds the callbacks that run inside a hook body before its own gate, and the external
//! `cpu_pilot` entry points (declared, not changed). Since M4b (U14) it also declares the WP3, WP4,
//! WP5 and WP-P callbacks and the U5 gate writer: the start of mutating work is gated
//! (`scope_gate::admitted` at the callback), while teardown, release and forward continuation
//! (`Exception::MustComplete`) and the writer are declared and never gated.
//!
//! FACTs (read in the baseline stage, `src/training/mod.rs` `once_per_frame_per_fighter`): four
//! calls run on every fighter frame in every mode **before** `if !is_training_mode() { return }`:
//! `lobby::update`, `character_roster::sources::on_frame`, `shield::param_installer`,
//! `character_items::on_frame`. Their reviews are the rows below.

use crate::engine::seams::native_seams::Scopes;

/// Where the game is now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Native Training (melee 0x20, mode 0x04040000).
    Training,
    /// A Quickplay or local-wireless waiting room (melee 0x0d).
    LobbyWaiting,
    /// Any offline match or other mode.
    Match,
    /// An online or local-wireless match (checked RNG, lockstep peers).
    Online,
}

impl Scope {
    pub const fn bit(self) -> Scopes {
        match self {
            Scope::Training => Scopes::TRAINING,
            Scope::LobbyWaiting => Scopes::LOBBY_WAITING,
            Scope::Match => Scopes::MATCH,
            Scope::Online => Scopes::ONLINE,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Scope::Training => "Training",
            Scope::LobbyWaiting => "LobbyWaiting",
            Scope::Match => "Match",
            Scope::Online => "Online",
        }
    }
}

/// Why a mutating callback may run in a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exception {
    None,
    /// Writes simulated state back to the value native code put there (covered by replay A/B run A).
    RestoresNative(&'static str),
    /// A crash guard on a null store; draws no checked random number (covered by run A).
    CrashGuard(&'static str),
    /// Not ours to change (`cpu_pilot`, WORKPLAN §2.9): declared as it is, qualified by its owner.
    External(&'static str),
    /// Releases, tears down or hands back to native only what the plugin itself started (its detached
    /// bodies and their pad routes, the map keys its ledger holds, its INSTALL lease, a request native
    /// has already begun). Native frees or settles that state next (finalize, scene exit, a Reload), and
    /// the scene words can already name the next scene there, so it must complete in every scope:
    /// declared with every scope and never gated (U14).
    MustComplete(&'static str),
}

/// One callback's declared contract and the review behind it.
#[derive(Clone, Copy, Debug)]
pub struct CallbackContract {
    pub name: &'static str,
    /// The hook it runs in (`native_seams` label), or "" for an entry point called from our code.
    pub seam: &'static str,
    pub scopes: Scopes,
    pub read_only: bool,
    pub per_frame: bool,
    pub exception: Exception,
    /// The review: what it does outside Training, with the source lines read.
    pub review: &'static str,
}

const EVERYWHERE: Scopes = Scopes::ALL;
const PRACTICE: Scopes = Scopes(Scopes::TRAINING.0 | Scopes::LOBBY_WAITING.0);

pub const CALLBACKS: &[CallbackContract] = &[
    CallbackContract {
        name: "native_body_bank::scene_header",
        seam: "body_bank.scene_header",
        scopes: Scopes::TRAINING,
        read_only: false,
        per_frame: false,
        exception: Exception::None,
        review: "P-HOT B H1 (LANE-HOT-SLOTS-REPORT.md §12): in native Training only, extends a visible role's resource \
                 header with its start-up window and publishes a pending bank; outside Training it returns before any \
                 write (entry_window::extendable needs native Training; src/runtime/roster/native_body_bank.rs)",
    },
    CallbackContract {
        name: "lobby::update",
        seam: "training.vtable_on_frame_slot",
        scopes: EVERYWHERE,
        read_only: true,
        per_frame: true,
        exception: Exception::None,
        review: "reads the scene tuple (game_mode::current: three native reads); on a change it tears down \
                 the plugin's own bookkeeping (banks departed, leases cleared, macros suspended) — no simulated \
                 state, no RNG (EVIDENCE §4.2; src/training/lobby.rs:112-156)",
    },
    CallbackContract {
        name: "character_roster::sources::on_frame",
        seam: "training.vtable_on_frame_slot",
        scopes: EVERYWHERE,
        read_only: true,
        per_frame: true,
        exception: Exception::None,
        review: "returns at once unless the plugin holds source leases (HAS_CONSUMERS), which scene_end clears \
                 before a match; otherwise P1's own body only: reads whether each lease's consumer lives and \
                 releases the plugin's lease book entry (src/character_sources.rs:1064-1097)",
    },
    CallbackContract {
        name: "shield::param_installer",
        seam: "training.vtable_on_frame_slot",
        scopes: EVERYWHERE,
        read_only: false,
        per_frame: true,
        exception: Exception::RestoresNative(
            "outside Training it writes COMMON_PARAMS.shield_damage_mul back to the value cached at its first \
             call (the native value); Infinite Shield sets 0.0 only in Training (src/training/shield.rs:167-193; \
             EVIDENCE §4.4 item 1; replay A/B run A)",
        ),
        review: "writes a simulated parameter on every fighter frame in every mode; the written value equals \
                 the native one outside Training",
    },
    CallbackContract {
        name: "character_items::on_frame",
        seam: "training.vtable_on_frame_slot",
        scopes: EVERYWHERE,
        read_only: true,
        per_frame: true,
        exception: Exception::None,
        review: "returns at once unless an item request is pending; outside Training it only cancels the \
                 plugin's pending request and its source lease (src/character_items.rs:117-127)",
    },
    CallbackContract {
        name: "cpu_pilot::init",
        seam: "",
        scopes: EVERYWHERE,
        read_only: true,
        per_frame: false,
        exception: Exception::External(
            "boot: reads the saved setting and request file (WORKPLAN §2.9); looks up the standalone \
             ssbu-cpu-pilot plugin's ssbu_cpu_pilot_state_v1 export and installs nothing when it is present; \
             checks the cpu_pilot seam rows before its one hook",
        ),
        review: "external boundary; CPU-as-P1 merge (ssbu-cpu-pilot-core 0162eed) added the read-only \
                 export lookup and the seam guard, no simulated state",
    },
    CallbackContract {
        name: "cpu_pilot::on_frame",
        seam: "training.vtable_on_frame_slot",
        scopes: Scopes(Scopes::TRAINING.0 | Scopes::MATCH.0 | Scopes::ONLINE.0),
        read_only: false,
        per_frame: true,
        exception: Exception::External(
            "P1 CPU Control stays allowed online (operator decision); runs native AI think with the shared \
             AI RNG state swapped for a private copy (EVIDENCE §4.4 item 2; since the CPU-as-P1 merge through \
             ssbu_cpu_pilot_core::native::think, the same reads, writes and call); qualified by its extracting \
             agent; this plan runs R-D4 only",
        ),
        review: "external boundary, declared without change",
    },
    CallbackContract {
        name: "cpu_pilot::input",
        seam: "needle.fim",
        scopes: Scopes(Scopes::TRAINING.0 | Scopes::MATCH.0 | Scopes::ONLINE.0),
        read_only: false,
        per_frame: true,
        exception: Exception::External("replaces P1's mapped inputs with the AI decision (EVIDENCE §4.4 item 2)"),
        review: "external boundary, declared without change",
    },
    CallbackContract {
        name: "cpu_pilot::menu_settings_changed",
        seam: "",
        scopes: PRACTICE,
        read_only: true,
        per_frame: false,
        exception: Exception::External("menu apply only"),
        review: "external boundary, declared without change",
    },
    // ---- M4 declarations (U14): WP3, WP4, WP5, WP-P and the U5 gate writer. Gated rows carry one
    // `if !scope_gate::admitted("<name>")` at the start of the operation; `MustComplete` rows and the
    // writer are declared and deliberately not gated (a refusal there would leave native the plugin's
    // half-released state). ----
    CallbackContract {
        name: "body_graph::execute",
        seam: "body_bank.finalize_entry",
        scopes: EVERYWHERE,
        read_only: false,
        per_frame: false,
        exception: Exception::MustComplete(
            "WP3 teardown executor: destroys the bank's own detached bodies, contexts and dispatchers and clears \
             their pad routes (`DestroyStep::ReleasePadRoute`, D-PTA1c) before native frees the window; runs from \
             the `finalize_entry` replace hook, from `before_reload` (mod Reload), at a native Reload's loader 0 \
             and in the swap rollback (`retire_window`/`retire_groups`/`perform_swap`, src/native_body_bank.rs)",
        ),
        review: "writes native state only to undo what the bank built; skipping it leaves bodies whose window \
                 native is about to free (dangling routes, the D-HANG1/DEFECT #2 class)",
    },
    CallbackContract {
        name: "r_handle::release",
        seam: "body_bank.finalize_entry",
        scopes: EVERYWHERE,
        read_only: false,
        per_frame: false,
        exception: Exception::MustComplete(
            "WP4: returns the map keys the role's ledger holds (`0x17e2fe0` orphan keys, then `0x17e8610`) on every \
             bank teardown (`release_group_keys` in `retire_groups`/`group_lifecycle`, the post-build object \
             release); WORKAROUND-DECISIONS row 15: a key kept past a teardown refuses the next Reload at loader 7",
        ),
        review: "native release calls for keys this plugin acquired; never a key native holds for itself",
    },
    CallbackContract {
        name: "native_body_bank::group_prefetch",
        seam: "roster.native_service",
        scopes: PRACTICE,
        read_only: false,
        per_frame: true,
        exception: Exception::None,
        review: "WP4: a planned group's prefetch acquires native map keys through the ledger (`r_handle::acquire`, \
                 `0x17e64d0`) under INSTALL; outside Training and waiting rooms it does not start and the group \
                 stays Planned, the same path as a not-ready resource owner (`group_lifecycle`, src/native_body_bank.rs)",
    },
    CallbackContract {
        name: "mii_ordinary_cpu::prequeue",
        seam: "mii_ordinary_gate.mii_ordinary_gate_entry",
        scopes: PRACTICE,
        read_only: false,
        per_frame: false,
        exception: Exception::None,
        review: "the CPU Mii publication at the `0x6510f4` gate writes the destination record, E/D scalars or the \
                 same-class write set (src/mii_ordinary_cpu_native.rs `publish`/`publish_same_class`); a refusal is the \
                 ordinary prequeue refusal and the request continues natively (row 7)",
    },
    CallbackContract {
        name: "mii_ordinary_player::prequeue",
        seam: "mii_ordinary_gate.mii_ordinary_gate_entry",
        scopes: PRACTICE,
        read_only: false,
        per_frame: false,
        exception: Exception::None,
        review: "the Player Mii publication at the `0x6510f4` gate writes the record and E/D scalars or the same-class \
                 write set (src/mii_ordinary_native.rs `publish`/`publish_same_class`); a refusal is the ordinary \
                 prequeue refusal and the request continues natively (row 7)",
    },
    CallbackContract {
        name: "mii_ordinary_cpu::continue_natively",
        seam: "mii_ordinary_gate.mii_ordinary_gate_entry",
        scopes: EVERYWHERE,
        read_only: false,
        per_frame: false,
        exception: Exception::MustComplete(
            "WP5b row 7 forward continuation: after states 1-3 destroyed the window, the refused request hands the \
             staged descriptor to native's state-6 tail instead of stranding the entry at loader 6 (D-PTA1b)",
        ),
        review: "changes only the transaction's state; native installs what it already began",
    },
    CallbackContract {
        name: "mii_ordinary_player::continue_natively",
        seam: "mii_ordinary_gate.mii_ordinary_gate_entry",
        scopes: EVERYWHERE,
        read_only: false,
        per_frame: false,
        exception: Exception::MustComplete(
            "WP5c row 7 forward continuation (Player side of D-PTA1b): the refused request continues natively",
        ),
        review: "changes only the transaction's state; native installs what it already began",
    },
    CallbackContract {
        name: "mii_ordinary_cpu::settle_outcome",
        seam: "roster.native_service",
        scopes: EVERYWHERE,
        read_only: false,
        per_frame: false,
        exception: Exception::MustComplete(
            "WP5b single release site: ends the CPU transaction (`[Txn] outcome=…`) and releases its INSTALL lease \
             at the loader settle, the bank settle and the exact/bank-switch destructors",
        ),
        review: "a lease kept past a scene change blocks every later install",
    },
    CallbackContract {
        name: "mii_ordinary_player::settle_outcome",
        seam: "roster.native_service",
        scopes: EVERYWHERE,
        read_only: false,
        per_frame: false,
        exception: Exception::MustComplete(
            "WP5c single release site: ends the Player transaction and releases its INSTALL lease at the loader \
             settle, the bank settle and the exact/bank-switch destructors",
        ),
        review: "a lease kept past a scene change blocks every later install",
    },
    CallbackContract {
        name: "mii_ordinary_gate::install_r68_on_skip",
        seam: "mii_ordinary_gate.mii_ordinary_gate_entry",
        scopes: EVERYWHERE,
        read_only: false,
        per_frame: false,
        exception: Exception::RestoresNative(
            "U5 / W-M2-4: on the configure-skip path (`O+0x4c` bit0 at `0x6510f4`) writes the `O+0x68` record index \
             and `O+0x6c..0x7c` tokens native configure (`0x651334-0x65137c`) writes when bit0 is clear, for a tracked \
             Mii payload only (`identity_view::configured_mii_words`); not gated: a refusal would leave the stale \
             index for the bind `0x651444 -> 0x67e7d0` to retire another owner's record",
        ),
        review: "the configure-equivalent words, nothing else (receipt `ExpectedMutation::ConfigureSkipped`); \
                 proof rows R-R68a/b",
    },
    CallbackContract {
        name: "presentation::state_load_started",
        seam: "training.vtable_on_frame_slot",
        scopes: PRACTICE,
        read_only: true,
        per_frame: false,
        exception: Exception::None,
        review: "WP-P: value-only read of the role's HUD card (bust/bind) when a save-state load starts \
                 (src/swap_native/presentation.rs); writes only its own atomic",
    },
    CallbackContract {
        name: "presentation::state_load_completed",
        seam: "training.vtable_on_frame_slot",
        scopes: PRACTICE,
        read_only: true,
        per_frame: false,
        exception: Exception::None,
        review: "WP-P: value-only read of the card when the load completes, compared with the start, logged",
    },
    CallbackContract {
        name: "presentation::shadow_readback",
        seam: "roster.native_service",
        scopes: PRACTICE,
        read_only: true,
        per_frame: false,
        exception: Exception::None,
        review: "WP-P: value-only card readback after a publication or native reset, logged `[Hud] readback`",
    },
];

/// The callback contract by name.
#[cfg(test)]
pub fn contract(name: &str) -> Option<&'static CallbackContract> {
    CALLBACKS.iter().find(|c| c.name == name)
}

/// What the Switch dispatcher reads of one row: the name, the declared scopes, the read-only flag
/// and whether a reviewed exception excuses it. The review and the exception's reason stay in the
/// authored `CALLBACKS` for the host tests; `GATES` is built from them at compile time, so that text
/// never reaches the NRO (the U17 rule for row text).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gate {
    pub name: &'static str,
    pub scopes: Scopes,
    pub read_only: bool,
    pub excused: bool,
}

impl Gate {
    const EMPTY: Gate = Gate { name: "", scopes: Scopes::NONE, read_only: true, excused: false };

    /// Whether this callback may run in `now`: it must declare `now`; a mutating callback outside
    /// Training and waiting rooms also needs a reviewed exception.
    pub const fn admitted(&self, now: Scope) -> bool {
        self.scopes.contains(now.bit())
            && (self.read_only || matches!(now, Scope::Training | Scope::LobbyWaiting) || self.excused)
    }

    /// Whether it is admitted whatever the scene: every scope declared, and read-only or excused.
    /// The dispatcher answers these without reading the scene (a per-frame row costs no native read).
    pub const fn admitted_everywhere(&self) -> bool {
        self.scopes.contains(Scopes::ALL) && (self.read_only || self.excused)
    }
}

impl CallbackContract {
    /// The run-time view of this row (`GATES` holds one per row, in `CALLBACKS` order).
    pub const fn gate(&self) -> Gate {
        Gate {
            name: self.name,
            scopes: self.scopes,
            read_only: self.read_only,
            excused: !matches!(self.exception, Exception::None),
        }
    }
}

/// Rows in `CALLBACKS` (the dispatcher's refusal-logged bit set has one bit per row).
pub const GATE_COUNT: usize = CALLBACKS.len();

const fn project(rows: &[CallbackContract]) -> [Gate; GATE_COUNT] {
    let mut out = [Gate::EMPTY; GATE_COUNT];
    let mut i = 0;
    while i < rows.len() {
        out[i] = rows[i].gate();
        i += 1;
    }
    out
}

/// The dispatcher's table: one `Gate` per `CALLBACKS` row, same order.
pub static GATES: [Gate; GATE_COUNT] = project(CALLBACKS);

/// Whether `c` may run in `now` (the rule of [`Gate::admitted`]).
#[cfg(test)]
pub fn admitted(c: &CallbackContract, now: Scope) -> bool {
    c.gate().admitted(now)
}

/// Whether `c` is admitted whatever the scene (the rule of [`Gate::admitted_everywhere`]).
#[cfg(test)]
pub const fn admitted_everywhere(c: &CallbackContract) -> bool {
    c.gate().admitted_everywhere()
}

/// Scope from the three native scene reads (`game_mode::current`) and the native Training flag.
/// FACT: Training = (initialized, 0x20, 0x04040000); waiting rooms use melee 0x0d (Quickplay
/// 0x0701xxxx, local wireless 0x02010000); a Quickplay match is melee 0x0e, an LDN match melee
/// 0x01 with mode 0x02010000 (EVIDENCE §4.1).
pub fn scope_of(native_training: bool, melee: u8, mode: u32) -> Scope {
    if native_training {
        Scope::Training
    } else if melee == 0x0d {
        Scope::LobbyWaiting
    } else if melee == 0x0e || mode & 0xffff_0000 == 0x0701_0000 || mode == 0x0201_0000 {
        Scope::Online
    } else {
        Scope::Match
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    const ALL_SCOPES: [Scope; 4] = [Scope::Training, Scope::LobbyWaiting, Scope::Match, Scope::Online];

    fn admitted_everywhere_is_exactly_admitted_in_all_four_scopes() {
        // The dispatcher's no-read fast path must never admit what `admitted` would refuse.
        for c in CALLBACKS {
            assert_eq!(admitted_everywhere(c), ALL_SCOPES.iter().all(|now| admitted(c, *now)), "{}", c.name);
        }
        assert!(admitted_everywhere(contract("shield::param_installer").unwrap()));
        assert!(!admitted_everywhere(contract("cpu_pilot::on_frame").unwrap()), "declared scopes are respected");
        assert!(!admitted_everywhere(contract("mii_ordinary_cpu::prequeue").unwrap()));
    }

    fn scope_from_the_scene_tuple() {
        assert_eq!(scope_of(true, 0x20, 0x0404_0000), Scope::Training);
        assert_eq!(scope_of(false, 0x0d, 0x0701_0101), Scope::LobbyWaiting);
        assert_eq!(scope_of(false, 0x0d, 0x0201_0000), Scope::LobbyWaiting);
        assert_eq!(scope_of(false, 0x0e, 0x0701_0101), Scope::Online);
        assert_eq!(scope_of(false, 0x01, 0x0201_0000), Scope::Online);
        assert_eq!(scope_of(false, 0x01, 0x0101_0000), Scope::Match);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("admitted_everywhere_is_exactly_admitted_in_all_four_scopes", admitted_everywhere_is_exactly_admitted_in_all_four_scopes as fn()),
            ("scope_from_the_scene_tuple", scope_from_the_scene_tuple as fn()),
        ]);
    }
    }
}
