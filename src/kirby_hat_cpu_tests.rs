//! B4-3 (Surface lane `artifacts/phase23/LANE-SURFACE-REPORT.md` "B4-3 root cause"; Ally lane `LANE-ALLY-REPORT.md`
//! "Round 4" §3): the CPU Kirby hat request stuck forever because `kirby_hats::on_frame` ran only with Player 1's
//! body. It now runs once per Kirby with that Kirby's own body (`state_kirby_opff_hook`, the fork's call site);
//! the role match is `hat_role`. The call-site wiring itself is checked in `tests/kirby_hat_cpu.rs`.

use crate::engine::kirby::kirby_hat_cycle::{hat_role, ShortcutLatch};

/// Stand-ins for two module accessors: equality is all `hat_role` reads.
const P1_KIRBY: usize = 0x1000;
const CPU_KIRBY: usize = 0x2000;

/// The right-click chord requests role 1 (CPU) when the CPU's body is a Kirby, and the CPU Kirby's own frame
/// callback then serves that role: the request, then the role-1 match that lets the pending request age and start.
/// P1's Kirby still serves role 0.
#[test]
fn a_cpu_kirby_body_serves_the_cpu_request_and_p1_still_serves_its_own() {
    let roles = [Some(P1_KIRBY), Some(CPU_KIRBY)];
    let mut latch = ShortcutLatch::new();
    let (request, suppress) = latch.sample(true, true, false, true, roles.map(|r| r.is_some()));
    assert_eq!(request, Some(1), "Special + right stick click requests the CPU's hat");
    assert!(suppress);
    assert_eq!(hat_role(CPU_KIRBY, roles), Some(1), "the CPU Kirby's own callback serves role 1");
    let mut latch = ShortcutLatch::new();
    assert_eq!(latch.sample(true, true, true, false, [true, true]).0, Some(0), "Special + left click requests P1's");
    assert_eq!(hat_role(P1_KIRBY, roles), Some(0), "P1's Kirby still serves role 0");
    // A body that is neither role's Kirby (a helper, or a role that is not a Kirby) serves no request.
    assert_eq!(hat_role(0x3000, roles), None);
    assert_eq!(hat_role(CPU_KIRBY, [Some(P1_KIRBY), None]), None, "the CPU is not a Kirby");
    // The defect, as wiring: one callback per frame with Player 1's body never reached role 1; one per Kirby does.
    let served = |bodies: &[usize]| bodies.iter().filter_map(|b| hat_role(*b, roles)).collect::<Vec<_>>();
    assert_eq!(served(&[P1_KIRBY]), [0]);
    assert_eq!(served(&[P1_KIRBY, CPU_KIRBY]), [0, 1]);
}
