#!/usr/bin/python3
"""Write exact-text carry-manifest edits from the derived base and wanted files.

UTF-8 input keeps line endings. SequenceMatcher grouped opcodes with three lines
of context give one edit per group, processed last to first. When an old block
is not unique in the current text, grow its context upward in both old and new
until it is unique or reaches the top. Replay each replacement in order and
require the result to equal the wanted text before writing the manifest.
Check the supplied base against the manifest's destination hash. Only carry
writes a carried file or updates its recorded hash.
"""
import argparse
import difflib
import hashlib
import json
import os
import sys


DEFAULT_MANIFEST = "docs/provenance/pkg4-carry-manifest.json"
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def derive(base: str, wanted: str, reason: str, test_reason=None, tests_marker=None) -> list[dict]:
    if (test_reason is None) != (tests_marker is None):
        raise SystemExit("--test-reason and --tests-marker must be given together")
    if base == wanted:
        return []
    a, b = base.splitlines(keepends=True), wanted.splitlines(keepends=True)
    tests_from = None
    if tests_marker is not None:
        tests_from = next((i for i, line in enumerate(a) if line.startswith(tests_marker)), None)
        if tests_from is None:
            raise SystemExit(f"tests marker not found in base: {tests_marker}")
    edits, text = [], base
    groups = list(difflib.SequenceMatcher(None, a, b, autojunk=False).get_grouped_opcodes(3))
    for group in reversed(groups):
        i1, i2, j1, j2 = group[0][1], group[-1][2], group[0][3], group[-1][4]
        before = 0
        while True:
            old = "".join(a[i1 - before:i2])
            new = "".join(a[i1 - before:i1]) + "".join(b[j1:j2])
            if text.count(old) == 1:
                break
            if i1 - before == 0:
                raise SystemExit(f"no unique context for the block at base line {i1 + 1}")
            before += 1
        edits.append({"reason": test_reason if tests_from is not None and i1 >= tests_from else reason,
                      "old": old, "new": new})
        text = text.replace(old, new)
    if text != wanted:
        raise SystemExit("the replayed edits do not give the wanted text")
    return edits


def rebuild(args):
    import carry_pkg4
    import carry_pkg6

    tool = carry_pkg6 if "pkg6" in os.path.basename(args.manifest) else carry_pkg4
    path = os.path.join(args.root, args.manifest)
    with open(path, encoding="utf-8") as f:
        manifest = json.load(f)
    entry = next((entry for entry in manifest["files"] if entry["dest"] == args.dest), None)
    if entry is None:
        raise SystemExit(f"no manifest entry for {args.dest}")
    if entry.get("removed"):
        raise SystemExit(f"cannot rebuild removed entry {args.dest}")
    stage_dir = os.path.join(tool.STAGES, args.stage, "stage")
    source_path = tool.source_path(entry, stage_dir) if tool is carry_pkg4 else os.path.join(stage_dir, entry["src"])
    with open(source_path, "rb") as f:
        source = f.read()
    if tool.sha(source) != entry.get("src_sha256"):
        raise SystemExit(f"source hash does not match the manifest for {entry['src']}")
    with open(os.path.join(args.root, args.dest), "rb") as f:
        wanted = f.read()
    if tool.sha(wanted) != entry.get("dest_sha256"):
        raise SystemExit(f"destination hash does not match the manifest for {args.dest}")
    dest_by_src = {item["src"]: item["dest"] for item in manifest["files"]}
    base = tool.transform({**entry, "edits": []}, manifest["modules"], source, dest_by_src)
    current, kept, dropped = base, [], []
    for edit in entry.get("edits", []):
        try:
            updated = tool.apply_edits({"src": entry["src"], "edits": [edit]}, current)
        except SystemExit:
            dropped.append(edit)
        else:
            kept.append(edit)
            current = updated
    reasons = list(dict.fromkeys(edit["reason"] for edit in dropped))
    reason = f"{args.reason}; replaces {len(dropped)} edit(s) that no longer applied in order: {' | '.join(reasons)}"
    derived = derive(current.decode("utf-8"), wanted.decode("utf-8"), reason)
    edits = kept + derived
    if tool.apply_edits({"src": entry["src"], "edits": edits}, base) != wanted:
        raise SystemExit(f"rebuilt edits do not reproduce {args.dest}")
    print(f"{args.dest}: kept {len(kept)}, dropped {len(dropped)}, derived {len(derived)}")
    if args.dry_run:
        print("dry run: no files written")
        return
    entry["edits"] = edits
    with open(path, "w", encoding="utf-8", newline="") as f:
        json.dump(manifest, f, indent=1)
        f.write("\n")
    print(f"rebuilt {args.dest}; now run: python3 -I tools/carry_{'pkg6' if tool is carry_pkg6 else 'pkg4'}.py carry, ledger, verify")


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("dest")
    ap.add_argument("base", nargs="?")
    ap.add_argument("--reason", required=True)
    ap.add_argument("--test-reason")
    ap.add_argument("--tests-marker")
    ap.add_argument("--manifest", default=DEFAULT_MANIFEST)
    ap.add_argument("--root", default=".")
    ap.add_argument("--wanted")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--rebuild", action="store_true")
    ap.add_argument("--stage")
    args = ap.parse_args(argv)
    if args.rebuild:
        if not args.stage:
            ap.error("--rebuild requires --stage")
        if args.base or args.wanted or args.test_reason or args.tests_marker:
            ap.error("--rebuild uses the recorded destination, not a base or alternate wanted file")
        rebuild(args)
        return
    if not args.base:
        ap.error("base is required without --rebuild")
    if args.stage:
        ap.error("--stage requires --rebuild")
    with open(args.base, encoding="utf-8", newline="") as f:
        base = f.read()
    with open(args.wanted or os.path.join(args.root, args.dest), encoding="utf-8", newline="") as f:
        wanted = f.read()
    edits = derive(base, wanted, args.reason, args.test_reason, args.tests_marker)
    if not edits:
        print("no change")
        return
    path = os.path.join(args.root, args.manifest)
    with open(path, encoding="utf-8") as f:
        manifest = json.load(f)
    entry = next((entry for entry in manifest["files"] if entry["dest"] == args.dest), None)
    if entry is None:
        raise SystemExit(f"no manifest entry for {args.dest}")
    if hashlib.sha256(base.encode("utf-8")).hexdigest() != entry.get("dest_sha256"):
        raise SystemExit(f"base does not match the derived destination hash for {args.dest}; run carry and use its output as the base")
    if not args.dry_run:
        entry.setdefault("edits", []).extend(edits)
        with open(path, "w", encoding="utf-8", newline="") as f:
            json.dump(manifest, f, indent=1)
            f.write("\n")
    for edit in edits:
        print(f"{len(edit['old'].splitlines()):>3} -> {len(edit['new'].splitlines()):>3} lines  {edit['reason'][:60]}")
    if args.dry_run:
        print(f"dry run: {len(edits)} edits derived for {args.dest}; no files written")
        return
    package = "pkg6" if "pkg6" in os.path.basename(args.manifest) else "pkg4"
    print(f"{len(edits)} edits appended to {args.dest}; now run: python3 -I tools/carry_{package}.py carry, ledger, verify")


if __name__ == "__main__":
    main()
