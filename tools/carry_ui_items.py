#!/usr/bin/env python3
"""Verify or carry the 38 approved binary item assets and their source receipt.

The Rust carry tools rewrite text; this identity-only binary carry checks the
same provenance boundary before reading any stage asset. Never writes a stage.
Usage: carry_ui_items.py [--stage /path/to/m5q/stage] [--carry]
Without --stage, verify the destination hashes and generated ledger only.
"""
import argparse
import hashlib
import json
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
    assert len(rows) == 39
    ledger = ["# UI item asset carry ledger", "", "Stage m5q; identity copies, no image edits. All sources are `ours-new`.", "",
              "| Source | Destination | SHA256 | Transform |", "| --- | --- | --- | --- |"]
    for row in rows:
        source, destination = row["source"], row["destination"]
        proof = provenance[source]
        assert proof["origin"] == "ours-new" and row["transform"] == "identity"
        assert row["sha256"].startswith(proof["stage_sha256"])
        target = root / destination
        if args.stage is not None:
            data = (args.stage / source).read_bytes()
            assert hashlib.sha256(data).hexdigest() == row["sha256"], source
            if args.carry:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
        assert hashlib.sha256(target.read_bytes()).hexdigest() == row["sha256"], destination
        ledger.append(f"| `{source}` | `{destination}` | `{row['sha256']}` | identity |")
    content = "\n".join(ledger) + "\n"
    target = root / "docs/provenance/ui-item-assets-ledger.md"
    if args.carry:
        target.write_text(content)
    else:
        assert target.read_text() == content, "stale asset ledger"
    print("39 UI item asset files and ledger verified")


if __name__ == "__main__":
    main()
