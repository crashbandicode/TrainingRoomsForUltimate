"""Recheck the specific ui16/ui17 missing-label report against original GIF pixels.

Usage: python3 -I eden/ui_glyph_audit.py [eden/runs]
The baseline's complete labels were visually checked. Equality covers only the
listed regions and captured frames; it is not an assertion about unseen frames.
"""
import hashlib
import json
from pathlib import Path
import sys

from PIL import Image

REGIONS = {
    "tabs": (40, 45, 1235, 85),
    "groups": (35, 140, 300, 210),
    "hud_label": (340, 140, 400, 165),
    "row_labels": (340, 178, 640, 310),
    "delay_label": (340, 320, 470, 345),
    "footer": (45, 595, 1000, 685),
}
CAPTURES = {
    "ui-17": ["display-baseline", "defaults-x-two", "defaults-y-row",
              "defaults-zr-page", "defaults-y-idle-ten"],
    "ui-16": ["defaults-y-tenframes", "defaults-zr-tenframes",
              "defaults-save-only-x", "defaults-y-row-reset", "defaults-zr-page-reset"],
}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def audit(baseline_path, paths, regions):
    with Image.open(baseline_path) as image:
        baseline = image.convert("RGB")
    expected = {name: baseline.crop(box).tobytes() for name, box in regions.items()}
    rows = []
    for path in paths:
        frames = []
        with Image.open(path) as image:
            for index in range(image.n_frames):
                image.seek(index)
                rgb = image.convert("RGB")
                equal = {name: rgb.crop(box).tobytes() == expected[name]
                         for name, box in regions.items()}
                frames.append({"frame": index, "regions_equal_baseline": equal})
        rows.append({"file": str(path), "sha256": digest(path), "frames": frames})
    return {"baseline": str(baseline_path), "baseline_sha256": digest(baseline_path),
            "regions": regions, "files": rows,
            "total_frames": sum(len(row["frames"]) for row in rows),
            "all_compared_regions_identical": all(all(f["regions_equal_baseline"].values())
                                                  for row in rows for f in row["frames"])}


def main():
    runs = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).parent/"runs"
    paths = [runs/run/"final/run/shots"/f"{name}.gif"
             for run, names in CAPTURES.items() for name in names]
    result = audit(runs/"ui-17/final/run/shots/display-baseline.gif", paths, REGIONS)
    print(json.dumps(result, indent=2))
    return 0 if result["all_compared_regions_identical"] else 1


if __name__ == "__main__":
    sys.exit(main())
