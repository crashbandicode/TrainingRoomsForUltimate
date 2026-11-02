#!/usr/bin/env python3
"""Package 4 carry tool (S4-4): manifest-driven copy of the fork's named-macro / replay / clip layer with a deterministic
path rewrite. Same mechanism as tools/carry_pkg6.py (one manifest, recorded edits, a re-sync test), kept as its own file so the
two streams do not edit one tool.

    python3 -I tools/carry_pkg4.py carry   [--stage m5q] [--update] [--manifest FILE]
    python3 -I tools/carry_pkg4.py verify  [--stage m5q] [--dest-only]
    python3 -I tools/carry_pkg4.py ledger

The manifest (docs/provenance/pkg4-carry-manifest.json) lists, for every carried file, the source path, the destination, the
crate the file belongs to in the fork (`macros`, `tui`, `root`, `smd` for the vendored smash-match-data crate), an optional list
of exact-text `edits` (each carries its reason and is a CARRY-OVER ledger row), and the sha256 of the source and of the written
destination. The `root` of an entry is `stage` (the frozen fork stage, default) or `smd` (the standalone `smash-match-data-design`
crate, the folder above the stages). The manifest also holds the module map the rewrite uses:
`modules[crate][fork module] = clean-room module path`; the key `*` is the default prefix for any other `crate::x` of that crate.

* `carry` reads each source file (only paths listed in the manifest; every one is an `ours-new` file of
  docs/provenance/provenance-files.json, or the standalone crate), checks its sha256 against the manifest (a changed file needs
  `--update`, which also refreshes the manifest and prints the changed files for the ledger), applies the rewrite and the edits,
  writes the destination and records its sha256.
* `verify` re-derives every destination from the source and compares bytes (or, with `--dest-only`, compares the destination's
  sha256 with the manifest: no stage needed, used by the host test). A hand-edited carried file fails.

Nothing under the stage is ever written.
"""
import argparse, hashlib, json, os, posixpath, re, sys

STAGES = "/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/stages"
DEFAULT_MANIFEST = "docs/provenance/pkg4-carry-manifest.json"
LEDGER = "docs/provenance/pkg4-carry-ledger.md"
EXTERNAL = {"training_mod_consts": "consts", "training_mod_mii": "mii", "training_mod_tui": "tui", "training_mod_macros": "macros",
            "training_mod_sync": "sync"}
HEADS = "crate|" + "|".join(EXTERNAL)


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def split_top(inner: str):
    parts, depth, cur = [], 0, ""
    for ch in inner:
        if ch == "{": depth += 1
        if ch == "}": depth -= 1
        if ch == "," and depth == 0:
            parts.append(cur); cur = ""
        else:
            cur += ch
    if cur.strip():
        parts.append(cur)
    return [p.strip() for p in parts if p.strip()]


def rewrite_path(head_crate: str, member: str, modules: dict):
    """`member` is 'name' or 'name::rest' under a crate; returns the clean-room path or None when unmapped."""
    segs = member.split("::")
    table = modules.get(head_crate, {})
    for k in range(len(segs), 0, -1):  # longest mapped prefix wins
        dest = table.get("::".join(segs[:k]))
        if dest is not None:
            rest = "::".join(segs[k:])
            return dest + ("::" + rest if rest else "")
    default = table.get("*")
    if default is not None and segs[0] not in ("engine", "model", "runtime"):  # already a clean-room path (the second pass sees the first pass output)
        return default + "::" + member
    return None


def rewrite_source(text: str, file_crate: str, modules: dict) -> str:
    # the storage root moved (decision S1, model::storage): every fork path under sd:/ultimate/TrainingModpack is under TrainingRooms
    text = text.replace("sd:/ultimate/TrainingModpack", "sd:/ultimate/TrainingRooms")

    def group(m):
        head = m.group(1)
        crate = file_crate if head == "crate" else EXTERNAL.get(head)
        if crate is None:
            return m.group(0)
        out = []
        for mem in split_top(m.group(2)):
            new = rewrite_path(crate, mem.split(" as ")[0].strip(), modules)
            alias = " as " + mem.split(" as ")[1].strip() if " as " in mem else ""
            out.append(f"{head}::{mem}" if new is None else new + alias)
        return "use {" + ", ".join(out) + "};"

    text = re.sub(r"\buse\s+(" + HEADS + r")::\{((?:[^{}]|\{[^{}]*\})*)\}\s*;", group, text)

    def plain(m):
        head, member = m.group(1), m.group(2)
        crate = file_crate if head == "crate" else EXTERNAL[head]
        new = rewrite_path(crate, member, modules)
        return m.group(0) if new is None else new

    return re.sub(r"\b(" + HEADS + r")::([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)", plain, text)


def rewrite_includes(text: str, entry: dict, dest_by_src: dict) -> str:
    def inc(m):
        rel = m.group(2)
        target = posixpath.normpath(posixpath.join(posixpath.dirname(entry["src"]), rel))
        if target not in dest_by_src:
            raise SystemExit(f"{entry['src']}: {m.group(1)}(\"{rel}\") names {target}, which is not in the manifest")
        new = posixpath.relpath(dest_by_src[target], posixpath.dirname(entry["dest"]))
        return f'{m.group(1)}("{new}")'
    return re.sub(r"\b(include_str!|include_bytes!|include!)\(\s*\"([^\"]+)\"\s*\)", inc, text)


def apply_edits(entry: dict, out: bytes) -> bytes:
    for e in entry.get("edits", []):
        s = out.decode("utf-8")
        if "delete_from" in e:
            # delete [delete_from, delete_to): `delete_from` occurs once; `delete_to` is the first occurrence after it
            if s.count(e["delete_from"]) != 1:
                raise SystemExit(f"{entry['src']}: delete_from `{e['delete_from'][:60]}` matches {s.count(e['delete_from'])} times")
            a = s.index(e["delete_from"])
            b = s.rindex(e["delete_to_last"]) if "delete_to_last" in e else s.find(e["delete_to"], a + len(e["delete_from"]))
            if b < 0 or b < a:
                raise SystemExit(f"{entry['src']}: delete_to for `{e['delete_from'][:60]}` not found")
            out = (s[:a] + s[b:]).encode("utf-8")
        elif "append" in e:
            out = (s + e["append"]).encode("utf-8")
        else:
            n = s.count(e["old"])
            if n != e.get("count", 1):
                raise SystemExit(f"{entry['src']}: edit `{e['old'][:60]}` matches {n} times, expected {e.get('count', 1)}")
            out = s.replace(e["old"], e["new"]).encode("utf-8")
    return out


def transform(entry: dict, modules: dict, src_bytes: bytes, dest_by_src: dict) -> bytes:
    if not entry["src"].endswith(".rs") or entry["crate"] == "smd":
        out = src_bytes
    else:
        text = src_bytes.decode("utf-8")
        text = rewrite_source(text, entry["crate"], modules)
        text = rewrite_includes(text, entry, dest_by_src)
        if entry.get("prepend"):
            lines = text.split("\n")
            k = 0
            while k < len(lines) and (lines[k].startswith("//!") or lines[k].strip() == ""):
                k += 1
            text = "\n".join(lines[:k]) + ("\n" if k else "") + entry["prepend"] + "\n".join(lines[k:])
        out = text.encode("utf-8")
    return apply_edits(entry, out)


def source_path(entry: dict, stage_dir: str) -> str:
    if entry.get("root", "stage") == "smd":
        return os.path.normpath(os.path.join(STAGES, "..", "smash-match-data-design", entry["src"]))
    return os.path.join(stage_dir, entry["src"])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=["carry", "verify", "ledger"])
    ap.add_argument("--stage", default=None)
    ap.add_argument("--update", action="store_true")
    ap.add_argument("--dest-only", action="store_true")
    ap.add_argument("--manifest", default=DEFAULT_MANIFEST)
    ap.add_argument("--root", default=".")
    a = ap.parse_args()
    mpath = os.path.join(a.root, a.manifest)
    manifest = json.load(open(mpath))
    stage_name = a.stage or manifest["stage"]
    stage_dir = os.path.join(STAGES, stage_name, "stage")
    modules = manifest["modules"]
    dest_by_src = {e["src"]: e["dest"] for e in manifest["files"]}
    if a.cmd == "ledger":
        write_ledger(a.root, manifest)
        return
    bad = 0
    changed = []
    for entry in manifest["files"]:
        dest = os.path.join(a.root, entry["dest"])
        if a.cmd == "verify" and a.dest_only:
            if not os.path.exists(dest):
                print("MISSING", entry["dest"]); bad += 1; continue
            if sha(open(dest, "rb").read()) != entry["dest_sha256"]:
                print("EDITED ", entry["dest"]); bad += 1
            continue
        data = open(source_path(entry, stage_dir), "rb").read()
        src_hash = sha(data)
        if src_hash != entry["src_sha256"]:
            if a.cmd == "carry" and a.update:
                changed.append((entry["src"], entry["src_sha256"][:12], src_hash[:12]))
                entry["src_sha256"] = src_hash
            else:
                print("SOURCE CHANGED", entry["src"], entry["src_sha256"][:12], "->", src_hash[:12]); bad += 1; continue
        out = transform(entry, modules, data, dest_by_src)
        if a.cmd == "carry":
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            open(dest, "wb").write(out)
            entry["dest_sha256"] = sha(out)
        elif not os.path.exists(dest) or open(dest, "rb").read() != out:
            print("DIFFERS", entry["dest"]); bad += 1
    if a.cmd == "carry":
        manifest["stage"] = stage_name
        json.dump(manifest, open(mpath, "w"), indent=1)
        open(mpath, "a").write("\n")
        for c in changed:
            print("re-synced", *c)
        print(f"carried {len(manifest['files'])} files from stage {stage_name}")
    if bad:
        print(f"{bad} problem(s)")
        sys.exit(1)
    if a.cmd == "verify":
        print(f"verified {len(manifest['files'])} files" + (" (dest only)" if a.dest_only else f" against stage {stage_name}"))


def write_ledger(root, manifest):
    """The carried-file table of CARRY-OVER.md (stage, path, sha256, clean-room changes), generated from the manifest."""
    path = os.path.join(root, LEDGER)
    with open(path, "w") as f:
        f.write(f"# Package 4 carried files (stage {manifest['stage']})\n\n")
        f.write("Generated by `python3 -I tools/carry_pkg4.py ledger` from `pkg4-carry-manifest.json`; do not edit. Every file is a copy of\n")
        f.write("the source file with the path rewrite of the tool (`crate::x` / `training_mod_*::x` to the clean-room module path, listed in the\n")
        f.write("manifest's `modules`) and, where a row says so, the exact-text edits below it. `src sha256` is the source file, `here sha256` the\n")
        f.write("file in this repository. Rows with root `smd` come from the standalone `smash-match-data-design` crate (git `faac04e`).\n\n")
        f.write("| Here | Fork path | src sha256 | here sha256 | edits (clean-room changes, each with its reason) |\n| --- | --- | --- | --- | --- |\n")
        for e in manifest["files"]:
            edits = "; ".join(x["reason"] for x in e.get("edits", [])) or ("copied unchanged" if e["crate"] == "smd" else "path rewrite only")
            f.write(f"| `{e['dest']}` | `{e['src']}` | `{e['src_sha256'][:16]}` | `{e['dest_sha256'][:16]}` | {edits} |\n")


main()
