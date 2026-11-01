#!/usr/bin/env python3
"""File-level provenance of the fork stage against upstream (merge base).

Usage: python3 -I provenance_scan.py <stage_root> <upstream_root> <out_json> <out_md> [exclude_prefix ...]

For every file under the stage (as `rg --files` would list it: .gitignore respected,
so no target/), classify against the same path in the upstream tree:
  - ours-new          : path absent upstream
  - upstream-identical: byte-identical
  - mixed             : both exist, text differs; report retained-upstream vs fork-only line
                        counts and the fork-only line ranges
  - binary-differs    : both exist, binary content differs
Upstream files absent in the stage are listed as dropped-by-fork.
A new file whose basename matches an upstream basename with a high similarity ratio is
flagged as possibly moved so a human looks at it.
"""
import difflib
import hashlib
import json
import os
import subprocess
import sys


def rg_files(root):
    out = subprocess.run(["rg", "--files", root], capture_output=True, text=True, check=True).stdout
    rel = []
    for line in out.splitlines():
        if not line:
            continue
        rel.append(os.path.relpath(line, root))
    return sorted(rel)


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def is_text(data):
    if b"\0" in data[:8192]:
        return False
    try:
        data.decode("utf-8")
        return True
    except UnicodeDecodeError:
        return False


def ranges(indices):
    """Collapse sorted 1-based line indices into 'a-b' ranges."""
    out = []
    start = prev = None
    for i in indices:
        if start is None:
            start = prev = i
        elif i == prev + 1:
            prev = i
        else:
            out.append((start, prev))
            start = prev = i
    if start is not None:
        out.append((start, prev))
    return out


def fmt_ranges(rs, limit=12):
    parts = ["%d" % a if a == b else "%d-%d" % (a, b) for a, b in rs]
    if len(parts) > limit:
        return ", ".join(parts[:limit]) + ", … (+%d more)" % (len(parts) - limit)
    return ", ".join(parts)


def compare_text(up_lines, st_lines):
    sm = difflib.SequenceMatcher(None, up_lines, st_lines, autojunk=False)
    retained = 0
    fork_only = []  # stage line indices (1-based) not matched to upstream
    upstream_removed = 0
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == "equal":
            retained += i2 - i1
        else:
            if tag in ("replace", "delete"):
                upstream_removed += i2 - i1
            if tag in ("replace", "insert"):
                fork_only.extend(range(j1 + 1, j2 + 1))
    return retained, upstream_removed, fork_only


def main():
    stage, upstream, out_json, out_md = sys.argv[1:5]
    excludes = tuple(sys.argv[5:])
    stage_files = [p for p in rg_files(stage) if not p.startswith(excludes)]
    upstream_files = [p for p in rg_files(upstream) if not p.startswith(excludes)]
    up_set = set(upstream_files)
    st_set = set(stage_files)

    up_basenames = {}
    for p in upstream_files:
        up_basenames.setdefault(os.path.basename(p), []).append(p)

    rows = []
    for rel in stage_files:
        sp = os.path.join(stage, rel)
        with open(sp, "rb") as f:
            sdata = f.read()
        row = {"path": rel, "stage_bytes": len(sdata), "stage_sha256": sha256(sp)[:16]}
        if rel in up_set:
            upath = os.path.join(upstream, rel)
            with open(upath, "rb") as f:
                udata = f.read()
            row["upstream_sha256"] = sha256(upath)[:16]
            if sdata == udata:
                row["origin"] = "upstream-identical"
                row["stage_lines"] = sdata.count(b"\n")
            elif is_text(sdata) and is_text(udata):
                ul = udata.decode("utf-8").splitlines()
                sl = sdata.decode("utf-8").splitlines()
                retained, removed, fork_only = compare_text(ul, sl)
                row["origin"] = "mixed"
                row["upstream_lines"] = len(ul)
                row["stage_lines"] = len(sl)
                row["retained_upstream_lines"] = retained
                row["upstream_lines_removed"] = removed
                row["fork_only_lines"] = len(fork_only)
                row["fork_only_ranges"] = fmt_ranges(ranges(fork_only))
                # Full list for machine readers (the table above truncates after 12 ranges).
                row["fork_only_ranges_full"] = [[a, b] for a, b in ranges(fork_only)]
                row["retained_fraction_of_stage"] = round(retained / max(1, len(sl)), 3)
            else:
                row["origin"] = "binary-differs"
        else:
            row["origin"] = "ours-new"
            if is_text(sdata):
                row["stage_lines"] = sdata.decode("utf-8", "replace").count("\n")
            # possible move: same basename upstream with high similarity
            cands = up_basenames.get(os.path.basename(rel), [])
            best = None
            for c in cands:
                with open(os.path.join(upstream, c), "rb") as f:
                    cdata = f.read()
                if is_text(sdata) and is_text(cdata):
                    r = difflib.SequenceMatcher(None, cdata.decode("utf-8").splitlines(),
                                                sdata.decode("utf-8").splitlines(), autojunk=False).ratio()
                    if best is None or r > best[1]:
                        best = (c, r)
            if best and best[1] >= 0.3:
                row["possible_move_from"] = best[0]
                row["possible_move_ratio"] = round(best[1], 3)
        rows.append(row)

    dropped = [p for p in upstream_files if p not in st_set]

    summary = {}
    for r in rows:
        summary[r["origin"]] = summary.get(r["origin"], 0) + 1
    summary["dropped-by-fork"] = len(dropped)

    result = {"stage": stage, "upstream": upstream, "summary": summary, "files": rows, "dropped_by_fork": dropped}
    with open(out_json, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=1)

    with open(out_md, "w", encoding="utf-8") as f:
        f.write("| Path | Origin | Stage lines | Upstream lines | Retained upstream | Fork-only lines | Fork-only ranges (stage line numbers) |\n")
        f.write("| --- | --- | ---: | ---: | ---: | ---: | --- |\n")
        for r in rows:
            f.write("| `%s` | %s | %s | %s | %s | %s | %s |\n" % (
                r["path"], r["origin"] + (" (moved from `%s`? %.2f)" % (r["possible_move_from"], r["possible_move_ratio"]) if "possible_move_from" in r else ""),
                r.get("stage_lines", ""), r.get("upstream_lines", ""), r.get("retained_upstream_lines", ""),
                r.get("fork_only_lines", ""), r.get("fork_only_ranges", "")))
        f.write("\nDropped by fork (present upstream, absent in stage):\n\n")
        for p in dropped:
            f.write("- `%s`\n" % p)
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
