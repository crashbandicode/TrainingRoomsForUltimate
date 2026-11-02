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


if __name__ == "__main__":
    unittest.main(verbosity=2)
