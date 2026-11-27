#!/usr/bin/env python3
"""Verify or carry the 38 approved binary item assets and their source receipt, plus rc.13's Wario bike icon and rc.14's
egg target, its cracked frame and the placing cursor (drawn by tools/draw_egg_target.py; verified with its --check).

The Rust carry tools rewrite text; this identity-only binary carry checks the
same provenance boundary before reading any stage asset. Never writes a stage.
Usage: carry_ui_items.py [--stage /path/to/m5q/stage] [--carry]
Without --stage, verify the destination hashes and generated ledger only.
"""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", type=Path)
    parser.add_argument("--carry", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    manifest = json.loads((root / "docs/provenance/ui-item-assets.json").read_text())
    provenance = {r["path"]: r for r in json.loads((root / "docs/provenance/provenance-files.json").read_text())["files"]}
    if args.carry and args.stage is None:
        parser.error("--carry requires --stage")
    rows = manifest["files"]
    # 39 stage files, plus rc.13's Wario bike icon (an Eden capture, not a stage file; its sources.json entry appended)
    # and rc.14's egg target, its cracked frame and the placing cursor (drawn by a script here)
    assert len(rows) == 43
    ledger = ["# UI item asset carry ledger", "", "Stage m5q; identity copies, no image edits. All sources are `ours-new`.",
              "rc.13 adds `icon-38.png` (Wario: Bike), an Eden capture cut out by background subtraction, and appends its",
              "entry to `sources.json` (the stage file's digest is kept as `stage_sha256`).",
              "rc.14 adds `assets/egg-target/target.png` (Eggs-ercise's target), `cracked.png` (its break frame) and",
              "`cursor.png` (the placing cursor), drawn by `tools/draw_egg_target.py`.", "",
              "| Source | Destination | SHA256 | Transform |", "| --- | --- | --- | --- |"]
    for row in rows:
        source, destination = row["source"], row["destination"]
        target = root / destination
        if row["transform"] != "identity":
            assert row["origin"].startswith(("rc.13, ours-new", "rc.14, ours-new")), destination
            if "drawn_by" in row:
                subprocess.run([sys.executable, "-I", str(root / row["drawn_by"]), "--check"], check=True)
            if source is not None:
                proof = provenance[source]
                assert proof["origin"] == "ours-new" and row["stage_sha256"].startswith(proof["stage_sha256"])
                if args.stage is not None:
                    assert hashlib.sha256((args.stage / source).read_bytes()).hexdigest() == row["stage_sha256"], source
            assert hashlib.sha256(target.read_bytes()).hexdigest() == row["sha256"], destination
            label = source or row.get("drawn_by") or "rc.13 Eden capture"
            ledger.append(f"| `{label}` | `{destination}` | `{row['sha256']}` | {row['transform']} |")
            continue
        proof = provenance[source]
        assert proof["origin"] == "ours-new"
        assert row["sha256"].startswith(proof["stage_sha256"])
        if args.stage is not None:
            data = (args.stage / source).read_bytes()
            assert hashlib.sha256(data).hexdigest() == row["sha256"], source
            if args.carry:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
        assert hashlib.sha256(target.read_bytes()).hexdigest() == row["sha256"], destination
        ledger.append(f"| `{source}` | `{destination}` | `{row['sha256']}` | identity |")
    content = "\n".join(ledger) + "\n"
    target = root / "tools/provenance-data/ui-item-assets-ledger.txt"
    if args.carry:
        target.write_text(content)
    else:
        assert target.read_text() == content, "stale asset ledger"
    print("43 UI asset files and ledger verified")


if __name__ == "__main__":
    main()
