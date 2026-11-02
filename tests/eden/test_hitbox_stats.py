"""Host tests of eden/hitbox_stats.py on synthetic lines: the two line shapes (persistent design and the respawn proof build),
the weighted average, the maxima and an empty log."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "eden"))
import hitbox_stats as h  # noqa: E402

PERSISTENT = "\n".join([
    "1 #1 [Hitbox] stats passes=600 with_work=600 want_hit=0 want_hurt=12600 kept=12535 spawned=65 killed=0 ended=23 failed=0 dropped=0 held_max=42 held_now=42 sleeps=600 restores=600 commands=0 pass_ticks_avg=700 pass_ticks_max=245114 heap_allocations=31",
    "2 #2 [TrainingRooms] hook_stats allocations=31 delta=0 hook_calls=1 hooks=hitbox_frame:1",
    "3 #3 [Hitbox] stats passes=600 with_work=600 want_hit=36 want_hurt=12600 kept=12529 spawned=107 killed=12 ended=95 failed=0 dropped=0 held_max=54 held_now=42 sleeps=598 restores=600 commands=0 pass_ticks_avg=300 pass_ticks_max=19984 heap_allocations=31",
])
PROOF = "1 #1 [Hitbox] stats passes=600 requests=14700 refused=9657 with_calls=600 spheres=0 hurtbox_spheres=14700 max_hurtbox_spheres=30 sleeps=600 restores=600 commands=0 max_spheres=0 pass_ticks_avg=8735 pass_ticks_max=232782 heap_allocations=31"


def test_persistent():
    s = h.summary(PERSISTENT)
    assert s["lines"] == 2 and s["passes"] == 1200
    assert s["held_max"] == 54 and s["failed"] == 0
    assert (s["spawned"], s["killed"], s["ended"]) == (172, 12, 118)
    assert s["ticks_avg"] == 500, s  # weighted by passes
    assert s["ticks_max"] == 245114
    assert s["us_avg"] == round(500 * 1e6 / 19_200_000, 1)


def test_proof_counts_refusals():
    s = h.summary(PROOF)
    assert s["failed"] == 9657 and s["requests"] == 14700
    assert s["ticks_avg"] == 8735


def test_empty():
    assert h.summary("no stats here") == {"lines": 0}


if __name__ == "__main__":
    for name, fn in list(globals().items()):
        if name.startswith("test_"):
            fn()
    print("ok")
