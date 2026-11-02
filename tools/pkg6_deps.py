#!/usr/bin/env python3
"""Package 6 dependency census.

For every ours-new Rust file of the package-6 feature groups (read from the frozen stage), lists the
paths it names outside itself that resolve to a MIXED or upstream-identical file of the scan: those are
the interfaces the clean room has to provide (or the hooks to re-point). Only ours-new files are opened.
Output: docs/provenance/pkg6-deps.json and a printed summary.
"""
import json, os, re, sys, collections

STAGES = "/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/stages"
PKG6 = ["Hot-swap engine","Mii fighters","Cycle preload","CPU-as-P1 handoff","Kirby hats","Practice stages",
        "Lobby practice","Character items","Seam guards","Diagnostics, probes","Input display renderer optimisation"]
CRATES = ["training_mod_consts", "training_mod_tui", "training_mod_mii", "training_mod_macros", "training_mod_sync"]

def expand(path):
    """expand `a::{b, c::{d, e}, f}` into full paths"""
    path = path.strip()
    i = path.find("{")
    if i < 0:
        return [path]
    depth = 0; j = None
    for k in range(i, len(path)):
        if path[k] == "{": depth += 1
        elif path[k] == "}":
            depth -= 1
            if depth == 0: j = k; break
    if j is None: return [path]
    prefix = path[:i]; inner = path[i+1:j]
    parts = []; d = 0; cur = ""
    for ch in inner:
        if ch == "{": d += 1
        if ch == "}": d -= 1
        if ch == "," and d == 0:
            parts.append(cur); cur = ""
        else: cur += ch
    parts.append(cur)
    out = []
    for p in parts:
        p = p.strip()
        if not p: continue
        if p == "self": out.append(prefix.rstrip(":"))
        else:
            for e in expand(prefix + p): out.append(e)
    return out

def crate_of(path):
    for c in CRATES:
        if path.startswith(c + "/"): return c, c + "/src/"
    return "plugin", "src/"

def mod_chain_of_file(path):
    crate, root = crate_of(path)
    rel = path[len(root):]
    if rel.endswith("/mod.rs"): segs = rel[:-7].split("/")
    elif rel.endswith(".rs"): segs = rel[:-3].split("/")
    else: return crate, root, None
    if segs == ["lib"] or segs == ["main"]: segs = []
    return crate, root, segs

def resolve(crate_root, segs, files):
    """longest prefix of segs resolving to a file; returns (file, rest)"""
    for n in range(len(segs), -1, -1):
        sub = segs[:n]
        cands = []
        if n == 0:
            cands = [crate_root + "lib.rs", crate_root + "main.rs"]
        else:
            cands = [crate_root + "/".join(sub) + ".rs", crate_root + "/".join(sub) + "/mod.rs"]
        for c in cands:
            if c in files: return c, segs[n:]
    return None, segs

EDGES = {}
BYFILE = {}

def main():
    root = sys.argv[1] if len(sys.argv) > 1 else "."
    stage_name = sys.argv[2] if len(sys.argv) > 2 else "m5o"
    stage = os.path.join(STAGES, stage_name, "stage")
    files = {f["path"]: f["origin"] for f in json.load(open(os.path.join(root, "docs/provenance/provenance-files.json")))["files"]}
    groups = json.load(open(os.path.join(root, "docs/provenance/feature-groups.json")))
    res = {}
    for gname, g in groups.items():
        if not any(gname.startswith(p) for p in PKG6): continue
        deps = collections.defaultdict(lambda: collections.Counter())
        owners = collections.defaultdict(set)
        for p in g["files"]:
            if not p.endswith(".rs"): continue
            if files[p] != "ours-new": raise SystemExit("not ours-new " + p)
            full = os.path.join(stage, p)
            if not os.path.exists(full): continue
            text = open(full, encoding="utf-8", errors="replace").read()
            crate, croot, myseg = mod_chain_of_file(p)
            if myseg is None: continue
            refs = []
            for m in re.finditer(r"\buse\s+([^;]+);", text):
                for e in expand(m.group(1).replace("\n", " ")):
                    refs.append(re.sub(r"\s+", "", re.sub(r"\s+as\s+\w+\s*$", "", e)))
            for m in re.finditer(r"\b(crate|training_mod_\w+)((?:::[A-Za-z_][A-Za-z0-9_]*)+)", text):
                refs.append(m.group(1) + m.group(2))
            for r in refs:
                segs = r.split("::")
                head = segs[0]
                if head == "crate": c, cr, s = crate, croot, segs[1:]
                elif head == "super":
                    c, cr = crate, croot
                    s = list(myseg[:-1] if p.endswith("/mod.rs") is False else myseg[:-1])
                    if p.endswith("/mod.rs"): s = list(myseg[:-1])
                    k = 0
                    while k < len(segs) and segs[k] == "super":
                        if k > 0: s = s[:-1]
                        k += 1
                    s = s + segs[k:]
                elif head == "self":
                    c, cr = crate, croot
                    s = (list(myseg) if p.endswith("/mod.rs") else list(myseg)) + segs[1:]
                elif head in CRATES:
                    c, cr, s = head, head + "/src/", segs[1:]
                else: continue
                tf, rest = resolve(cr, s, files)
                if tf is None: continue
                # skip references into the file itself or other ours-new files of the same feature set
                if files[tf] == "ours-new":
                    EDGES.setdefault(p, set()).add(tf)
                    continue
                item = "::".join(rest[:1]) if rest else "(module)"
                deps[tf][item] += 1
                BYFILE.setdefault(p, {}).setdefault(tf, collections.Counter())[item] += 1
                owners[tf].add(p)
        res[gname] = {tf: {"origin": files[tf], "items": dict(sorted(c.items(), key=lambda kv: -kv[1])),
                           "used_by_files": len(owners[tf])} for tf, c in sorted(deps.items())}
    json.dump(res, open(os.path.join(root, "docs/provenance/pkg6-deps.json"), "w"), indent=1)
    json.dump({p: {tf: dict(c) for tf, c in d.items()} for p, d in sorted(BYFILE.items())}, open(os.path.join(root, "docs/provenance/pkg6-deps-by-file.json"), "w"), indent=1)
    json.dump({k: sorted(v) for k, v in sorted(EDGES.items())}, open(os.path.join(root, "docs/provenance/pkg6-edges.json"), "w"), indent=1)

    write_layers(root, groups)
    for g, d in res.items():
        print("==", g[:70])
        for tf, v in d.items():
            top = ", ".join(f"{k}({n})" for k, n in list(v["items"].items())[:12])
            print(f"  {tf} [{v['origin']}] files={v['used_by_files']}: {top}")


def write_layers(root, groups):
    """Condense the file graph into strongly connected components and write them by layer: a file may be
    carried once everything in lower layers is in (a cycle is carried as one unit)."""
    fo = {}
    for n, v in groups.items():
        for f in v["files"]:
            fo[f] = n.split(" (")[0].split(",")[0][:14]
    nodes = set(EDGES) | {t for ts in EDGES.values() for t in ts}
    adj = {n: set(EDGES.get(n, ())) - {n} for n in nodes}
    index = {}; low = {}; stack = []; on = set(); sccs = []; counter = [0]
    sys.setrecursionlimit(10000)

    def visit(v):
        index[v] = low[v] = counter[0]; counter[0] += 1; stack.append(v); on.add(v)
        for w in adj[v]:
            if w not in index:
                visit(w); low[v] = min(low[v], low[w])
            elif w in on:
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            comp = []
            while True:
                x = stack.pop(); on.discard(x); comp.append(x)
                if x == v:
                    break
            sccs.append(comp)

    for v in sorted(nodes):
        if v not in index:
            visit(v)
    cid = {x: i for i, s in enumerate(sccs) for x in s}
    dag = collections.defaultdict(set)
    for v in nodes:
        for w in adj[v]:
            if cid[v] != cid[w]:
                dag[cid[v]].add(cid[w])
    level = {}

    def lv(i):
        if i not in level:
            level[i] = 0 if not dag[i] else 1 + max(lv(j) for j in dag[i])
        return level[i]

    for i in range(len(sccs)):
        lv(i)
    layers = collections.defaultdict(list)
    for i, s in enumerate(sccs):
        layers[level[i]].append(s)
    with open(os.path.join(root, "docs/provenance/pkg6-layers.md"), "w") as f:
        f.write("# Package 6 carry layers (generated by `tools/pkg6_deps.py`)\n\n")
        f.write("Edges are `use` / path references between ours-new Rust files of the package-6 feature groups (and the ours-new files of ")
        f.write("packages 3/4 they name: `EXT`). A file is carried after everything in lower layers; a CYCLE is one unit.\n")
        for k in sorted(layers):
            f.write(f"\n## Layer {k}\n\n")
            for s in sorted(layers[k], key=lambda s: sorted(s)[0]):
                tag = "CYCLE: " if len(s) > 1 else ""
                f.write("- " + tag + " | ".join(f"`{x}` [{fo.get(x, 'EXT')}]" for x in sorted(s)) + "\n")


main()