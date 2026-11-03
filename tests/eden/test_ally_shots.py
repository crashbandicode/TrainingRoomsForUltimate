import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "eden/harness/ally"))
import client

class Shots(unittest.TestCase):
    def test_repeated_labels_and_new_transports_preserve_every_recording(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            recorded = []
            def run(transport, items, timeout=300):
                kind, name, *_ = items[0]
                self.assertEqual(kind, "gshot")
                path = root / (name + ".gif")
                # Eden refuses an existing recording instead of overwriting it.
                with path.open("xb") as stream:
                    Image.new("RGB", (2, 2), (len(recorded) * 80, 0, 0)).save(stream, format="GIF")
                recorded.append(path)
            def get(p, label, rel, dest):
                Path(dest).mkdir(parents=True, exist_ok=True)
                path = root / Path(rel).name
                self.assertEqual(path, recorded[-1])
                return path
            with patch.object(client, "HERE", root / "harness/ally"), patch.object(client, "get", get), patch.object(client.AllyTransport, "run", run):
                p = SimpleNamespace(output="TrainingRooms")
                first = client.AllyTransport(p, "ui-test")
                shots = [first.shot("after-a"), first.shot("after-a"), client.AllyTransport(p, "ui-test").shot("after-a")]
            self.assertEqual(len(set(recorded)), 3)
            self.assertEqual(len(set(shots)), 3)
            colors = []
            for p in shots:
                with Image.open(p) as image:
                    colors.append(image.getpixel((0, 0))[0])
            self.assertEqual(colors, [0, 80, 160])
            self.assertTrue(all(p.exists() for p in recorded))

if __name__ == "__main__":
    unittest.main()
