"""Original negative controls for the UI preview/perception false-failure audit."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("ui_glyph_audit", ROOT/"eden/ui_glyph_audit.py")
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class GlyphAuditTests(unittest.TestCase):
    def check(self, frames):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            baseline = self.frame()
            baseline.save(root/"baseline.gif")
            frames[0].save(root/"capture.gif", save_all=True, append_images=frames[1:],
                           duration=100, loop=0)
            return audit.audit(root/"baseline.gif", [root/"capture.gif"], {"label": (4, 4, 12, 12)})

    @staticmethod
    def frame():
        image = Image.new("RGB", (20, 20), (0, 0, 0))
        image.paste((255, 255, 255), (5, 5, 10, 10))
        return image

    def test_background_changes_do_not_report_missing_labels(self):
        changed = self.frame()
        changed.paste((255, 0, 0), (15, 15, 20, 20))
        self.assertTrue(self.check([self.frame(), changed])["all_compared_regions_identical"])

    def test_an_erased_label_is_detected(self):
        self.assertFalse(self.check([Image.new("RGB", (20, 20))])["all_compared_regions_identical"])

    def test_a_missing_middle_frame_is_not_hidden_by_recovery(self):
        result = self.check([self.frame(), Image.new("RGB", (20, 20)), self.frame()])
        self.assertEqual(result["total_frames"], 3)
        self.assertEqual([f["regions_equal_baseline"]["label"] for f in result["files"][0]["frames"]],
                         [True, False, True])
        self.assertFalse(result["all_compared_regions_identical"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
