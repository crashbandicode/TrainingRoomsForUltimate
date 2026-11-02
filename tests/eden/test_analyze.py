"""Host tests of eden/analyze_probe.py and eden/analyze_step.py on synthetic shots (Pillow only).

Run: /usr/bin/python3 -I tests/eden/test_analyze.py
Each dataset is a folder of 1280x674 grey PNGs plus the JSON the probe scripts write. Steve's region
(x 100-420, y 120-480) is flat grey at rest and gets a bright block from the onset frame on; a small
deterministic noise below the threshold is added to every shot. A planted 5-frame shift of the
onset must be measured as 5, no shift as 0, and a flicker that falls back below the threshold must
not be taken for the onset.
"""
import importlib.util
import json
import random
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / "eden" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


tr_eden = load("tr_eden")
probe = load("analyze_probe")
hook_stats = load("hook_stats")
step = load("analyze_step")

W, H = 1280, 674
BOX = (100, 120, 420, 480)


def shot(path, moved, rng, noise=3):
    """A rest frame (or one with the block) plus noise: every pixel of the region moves by up to `noise`."""
    value = 200 if moved else 90
    img = Image.new("L", (W, H), 40)
    region = Image.new("L", (BOX[2] - BOX[0], BOX[3] - BOX[1]), value)
    px = region.load()
    for _ in range(4000):
        x, y = rng.randrange(region.width), rng.randrange(region.height)
        px[x, y] = max(0, min(255, value + rng.randint(-noise, noise) * 4))
    img.paste(region, BOX[:2])
    img.save(path)


def dataset(root, name, frames, onset, flicker=(), seed=7):
    """probe.py's output shape: <name>.json rows and <name>/<shot> pngs, onset = first frame with the block."""
    rng = random.Random(seed)
    folder = Path(root) / name
    folder.mkdir(parents=True)
    shot(folder / "base.png", False, rng)
    rows = []
    for f in frames:
        moved = (f >= onset or f in flicker) if onset is not None else f in flicker
        if f in flicker and f < onset:
            moved = True
        shot(folder / f"f{f}.png", moved, rng)
        rows.append({"frame_paused": f, "shot": f"f{f}.png", "base": "base.png"})
    (Path(root) / f"{name}.json").write_text(json.dumps(rows))
    return Path(root)


FRAMES = list(range(56, 96))


class AnalyzeTests(unittest.TestCase):
    def check(self, module, onset_in, flicker=(), thr=6.5):
        with tempfile.TemporaryDirectory() as tmp:
            d = dataset(tmp, "jump", FRAMES, onset_in, flicker)
            return module.find_onset(module.table_for(d, "jump"), thr)

    def test_the_onset_is_found(self):
        for module in (probe, step):
            self.assertEqual(self.check(module, 69), 69)
            self.assertEqual(self.check(module, 74), 74)

    def test_a_planted_five_frame_shift_is_detected(self):
        for module in (probe, step):
            self.assertEqual(self.check(module, 74) - self.check(module, 69), 5)

    def test_no_shift_is_not_detected(self):
        for module in (probe, step):
            self.assertEqual(self.check(module, 74) - self.check(module, 74), 0)

    def test_no_movement_has_no_onset(self):
        for module in (probe, step):
            self.assertIsNone(self.check(module, None))

    def test_a_flicker_that_falls_back_is_not_the_onset(self):
        # Frames 60 and 61 light up and go dark again; the real onset is 74.
        for module in (probe, step):
            self.assertEqual(self.check(module, 74, flicker=(60, 61)), 74)

    def test_the_noise_floor_stays_below_the_threshold(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = dataset(tmp, "jump", FRAMES, 74)
            table = probe.table_for(d, "jump")
            onset = probe.find_onset(table, 6.5)
            self.assertLess(probe.max_noise(table, onset), 6.5)

    def test_the_command_line_prints_what_the_doc_cites(self):
        import contextlib
        import io

        with tempfile.TemporaryDirectory() as tmp:
            dataset(tmp, "jump", FRAMES, 69)
            for module, key in ((probe, "onset_frame=69"), (step, "onset_frame=69")):
                out = io.StringIO()
                old = sys.argv
                sys.argv = ["x", tmp, "jump", "6.5"]
                try:
                    with contextlib.redirect_stdout(out):
                        module.main()
                finally:
                    sys.argv = old
                self.assertIn(key, out.getvalue())


class ScanTests(unittest.TestCase):
    """`tr_eden.py scan` produces the "N lines, M matching" the evidence doc cites."""

    def test_a_clean_log_has_no_match(self):
        log = ["[ 1.0] Debug Service: ok", "HW.Memory read ok @ 0x80000000", "Frontend: running"]
        self.assertEqual(tr_eden.scan_lines(log), [])

    def test_planted_lines_are_found_in_any_case(self):
        log = ["a", "Core: PANIC at x", "b", "HW.Memory Unmapped Read32 @ 0x0000000000000008", "c", "Process Aborted", "CRASH"]
        self.assertEqual([n for n, _ in tr_eden.scan_lines(log)], [2, 4, 6, 7])

class HarnessConfigTests(unittest.TestCase):
    """A stream that shares the Yoga deploys `tr_eden.py` with its own config beside it."""

    def test_default_labels_are_package_ones_and_the_r1fix_config_has_its_own(self):
        import re
        self.assertEqual(tr_eden.label_ok("tr101"), "tr101")
        with self.assertRaises(AssertionError):
            tr_eden.label_ok("r1-01")
        cfg = json.loads((ROOT / "eden" / "tr_eden.r1fix.config.json").read_text())
        self.assertTrue(re.fullmatch(cfg["label_re"], "r1-01") and not re.fullmatch(cfg["label_re"], "tr101"))
        self.assertNotIn(cfg["display"], (":61", ":93", ":95"))
        self.assertTrue(cfg["root"].endswith("trainingrooms-eden-r1") and cfg["profile"] != "a")


class HookStatsGateTests(unittest.TestCase):
    """`eden/hook_stats.py`: a lane passes only with nonzero hook coverage and a zero allocation delta."""

    GOOD = ("100 #1 [TrainingRooms] hook_stats allocations=0 delta=0 hook_calls=412 "
            "hooks=input_caller_a_apply:100,reset_check:3\n"
            "200 #2 [TrainingRooms] hook_stats allocations=0 delta=0 hook_calls=60 "
            "hooks=input_caller_a_apply:130,reset_check:3,hazard_rule:30\n")

    def test_a_lane_with_coverage_and_no_allocation_passes(self):
        ok, reasons = hook_stats.gate(self.GOOD, ["input_caller_a_apply", "hazard_rule"])
        self.assertTrue(ok, reasons)

    def test_zero_coverage_fails(self):
        ok, reasons = hook_stats.gate(self.GOOD, ["pause_enter"])
        self.assertFalse(ok)
        self.assertTrue(any("pause_enter" in r for r in reasons))

    def test_no_line_at_all_fails(self):
        ok, reasons = hook_stats.gate("100 #1 [TrainingRooms] something else\n", [])
        self.assertFalse(ok)
        self.assertIn("no hook_stats line", reasons[0])

    def test_an_allocation_delta_fails_even_if_it_was_earlier(self):
        bad = ("1 #1 [TrainingRooms] hook_stats allocations=2 delta=2 hook_calls=5 hooks=reset_check:5\n"
               "2 #2 [TrainingRooms] hook_stats allocations=2 delta=0 hook_calls=5 hooks=reset_check:10\n")
        ok, reasons = hook_stats.gate(bad, ["reset_check"])
        self.assertFalse(ok)
        self.assertTrue(any("delta 2" in r for r in reasons) and any("total 2" in r for r in reasons))

    def test_the_line_the_plugin_prints_is_parsed(self):
        row = hook_stats.parse("[TrainingRooms] hook_stats allocations=0 delta=0 hook_calls=3 hooks=none")[0]
        self.assertEqual((row["delta"], row["hooks"]), (0, {}))
        row = hook_stats.parse(self.GOOD)[1]
        self.assertEqual(row["hooks"]["hazard_rule"], 30)

class LayoutArchiveGateTests(unittest.TestCase):
    GOOD = ("100 #1 [LayoutArchive] handed out info_training bytes=500 stock=400 base=vanilla load_allocations=0\n"
            "5200 #2 [LayoutArchive] released info_training bytes=500 after=5100ms\n")

    def test_vanilla_fallback_and_stock_waiting_retire_without_allocating(self):
        text = self.GOOD + self.GOOD.replace("info_training", "info_waiting").replace("base=vanilla", "base=stock").replace("5100ms", "5000ms")
        self.assertEqual(hook_stats.gate_layout_archives(text, ["info_training", "info_waiting"]), (True, []))

    def test_load_allocation_is_not_hidden_by_clean_steady_state_counters(self):
        text = self.GOOD.replace("load_allocations=0", "load_allocations=1") + HookStatsGateTests.GOOD
        ok, reasons = hook_stats.gate_layout_archives(text, ["info_training"])
        self.assertFalse(ok)
        self.assertTrue(any("load allocations 1" in reason for reason in reasons))

    def test_early_missing_or_unmatched_retirement_fails(self):
        for text in (self.GOOD.replace("5100ms", "4999ms"),
                     self.GOOD.splitlines()[0], self.GOOD.splitlines()[1],
                     self.GOOD.replace("released info_training bytes=500", "released info_training bytes=501")):
            with self.subTest(text=text):
                self.assertFalse(hook_stats.gate_layout_archives(text, ["info_training"])[0])

    def test_refusal_missing_layout_and_truncated_records_fail(self):
        for text, required in (("", []),
                               (self.GOOD + "[LayoutArchive] no free archive buffer (layout_arc=skipped)", ["info_training"]),
                               (self.GOOD, ["info_waiting"]),
                               ("[LayoutArchive] handed out info_training bytes=500", ["info_training"]),
                               (self.GOOD + "[LayoutArchive] released info_training", ["info_training"])):
            with self.subTest(text=text, required=required):
                self.assertFalse(hook_stats.gate_layout_archives(text, required)[0])


if __name__ == "__main__":
    unittest.main(verbosity=2)
