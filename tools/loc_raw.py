#!/usr/bin/python3
"""Count non-blank, non-comment Rust lines, split into product / test / diag-only / host-only.

Rule (TASK-rc10-code-reduction.md): *.rs outside target/, tests/, third_party/, vendor/.
A line is classified by the innermost cfg-gated item (or cfg-gated `mod x;` file) that contains it:
  test : #[cfg(test)] (any predicate that requires `test`), and files only reachable from such a mod
  diag : predicate requires a diag-family feature (never in the shipped NRO)
  host : predicate requires not(target_os = "switch") (never in the NRO; host shim for tests)
  prod : everything else
Usage: python3 -I tools/loc_raw.py <root> [--json out.json] [--depth N] [--top N]   (docs/CODE-REDUCTION-rc10.md; tests: tools/test_loc_raw.py)
"""
import json
import os
import re
import sys

DIAG_FEATURES = {
    "diag", "texture-probe", "seam-drill", "console-practice-trace", "console-swap-trace",
    "native-unlink-probe", "hang-trace", "plugin-heap-meter", "named-input-trace",
    "mii-ldn-qualification-probe", "automation-results-native", "simulator",
}
SKIP_DIRS = {"target", "tests", "third_party", "vendor", ".git"}


def strip(src):
    """Return src with comments removed and string/char contents blanked (newlines kept)."""
    out = []
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            i = n if j < 0 else j
        elif c == "/" and i + 1 < n and src[i + 1] == "*":
            depth, i = 1, i + 2
            while i < n and depth:
                if src.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif src.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    if src[i] == "\n":
                        out.append("\n")
                    i += 1
        elif c == '"' or (c == "r" and re.match(r'r#*"', src[i:i + 12]) and not (i and (src[i - 1].isalnum() or src[i - 1] == "_"))) \
                or (c == "b" and re.match(r'b(r#*)?"', src[i:i + 12]) and not (i and (src[i - 1].isalnum() or src[i - 1] == "_"))):
            m = re.match(r'b?(r(#*))?"', src[i:i + 14])
            raw, hashes = m.group(1) is not None, m.group(2) or ""
            i += m.end()
            out.append('"')
            if raw:
                end = src.find('"' + hashes, i)
                end = n if end < 0 else end
                out.append("".join(ch if ch == "\n" else "s" for ch in src[i:end]))
                i = min(n, end + 1 + len(hashes))
            else:
                while i < n and src[i] != '"':
                    if src[i] == "\\":
                        if i + 1 < n and src[i + 1] == "\n":
                            out.append("\n")
                        else:
                            out.append("s")
                        i += 2
                        continue
                    out.append("\n" if src[i] == "\n" else "s")
                    i += 1
                i += 1
            out.append('"')
        elif c == "'":
            m = re.match(r"'(\\.[^']*|[^'\\])'", src[i:i + 12])
            if m:
                out.append("'c'")
                i += m.end()
            else:
                out.append(c)
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def classify_pred(pred):
    """pred: text inside cfg(...), with string contents intact."""
    p = re.sub(r"\s+", "", pred)
    # remove not(...) groups to see what is positively required
    def drop_not(s):
        while True:
            k = s.find("not(")
            if k < 0:
                return s, []
            depth, j = 1, k + 4
            while j < len(s) and depth:
                depth += s[j] == "("
                depth -= s[j] == ")"
                j += 1
            inner = s[k + 4:j - 1]
            rest, more = drop_not(s[:k] + s[j:])
            return rest, [inner] + more
    pos, negs = drop_not(p)
    if pos.startswith("any("):
        # any(): gated in only if every alternative is non-product; keep simple: test if all alts mention test/diag
        alts = [a for a in re.split(r",(?![^()]*\))", pos[4:-1]) if a]
        kinds = {classify_pred(a) for a in alts}
        for neg in negs:
            kinds.add("host" if 'target_os="switch"' in neg else "prod")
        if kinds <= {"test"}:
            return "test"
        if kinds <= {"test", "diag"}:
            return "diag"
        if kinds <= {"test", "diag", "host"}:
            return "host"
        return "prod"
    if re.search(r"(^|[(,])test([),]|$)", pos):
        return "test"
    feats = set(re.findall(r'feature="([^"]+)"', pos))
    if feats & DIAG_FEATURES:
        return "diag"
    if any('target_os="switch"' in neg for neg in negs):
        return "host"
    return "prod"


ATTR = re.compile(r"#(!?)\[")


def analyse(path, src):
    """Return (line_kinds list for code lines, gated child mods {name: kind}, inner file kind or None, path attrs)."""
    code = strip(src)
    lines = code.split("\n")
    kinds = [None] * len(lines)
    # offsets
    starts, off = [], 0
    for ln in lines:
        starts.append(off)
        off += len(ln) + 1
    import bisect

    def line_of(pos):
        return bisect.bisect_right(starts, pos) - 1

    orig = src  # for reading string contents of attributes we need the original; map by re-scan below
    file_kind = None
    mods = {}
    spans = []
    i = 0
    n = len(code)
    for m in ATTR.finditer(code):
        a0 = m.start()
        # find end of attribute
        depth, j = 1, m.end()
        while j < n and depth:
            depth += code[j] == "["
            depth -= code[j] == "]"
            j += 1
        body = code[m.end():j - 1].strip()
        if not body.startswith("cfg(") and not body.startswith("cfg ("):
            continue
        # recover original predicate text (strings were blanked): take same lines from original source
        l0, l1 = line_of(a0), line_of(j - 1)
        otext = "\n".join(src.split("\n")[l0:l1 + 1])
        om = re.search(r"cfg\s*\((.*)\)\s*\]", otext, re.S)
        pred = om.group(1) if om else body[4:-1]
        kind = classify_pred(pred)
        if kind == "prod":
            continue
        if m.group(1) == "!":
            file_kind = kind
            continue
        # item extent: skip following attributes, then to ';' or matching '}' at depth 0
        k = j
        while True:
            mm = re.match(r"\s*#\[", code[k:])
            if not mm:
                break
            d, k = 1, k + mm.end()
            while k < n and d:
                d += code[k] == "["
                d -= code[k] == "]"
                k += 1
        head_start = k
        d = 0
        end = None
        while k < n:
            ch = code[k]
            if ch in "([":
                d += 1
            elif ch in ")]":
                d -= 1
                if d < 0:  # attribute on an expression/field/arg at end of enclosing group
                    end = k - 1
                    break
            elif ch == "{":
                # match braces
                bd, k = 1, k + 1
                while k < n and bd:
                    bd += code[k] == "{"
                    bd -= code[k] == "}"
                    k += 1
                # a struct-literal/expression may continue; treat item as ended unless followed by ; or else-ish
                rest = re.match(r"\s*(;|,)", code[k:])
                if rest and d == 0:
                    k += rest.end()
                end = k - 1
                break
            elif ch == "}" and d == 0:
                end = k - 1
                break
            elif ch in ";," and d == 0:
                end = k
                break
            k += 1
        if end is None:
            end = n - 1
        head = code[head_start:end + 1]
        mm = re.match(r"\s*(pub(\([^)]*\))?\s+)?mod\s+([A-Za-z_0-9]+)\s*;", head)
        if mm:
            mods[mm.group(3)] = kind
        spans.append((a0, end, kind))
    order = {"test": 3, "diag": 2, "host": 1}
    for a, b, kind in spans:
        for ln in range(line_of(a), line_of(b) + 1):
            if kinds[ln] is None or order[kind] > order[kinds[ln]]:
                kinds[ln] = kind
    counts = {"prod": 0, "test": 0, "diag": 0, "host": 0}
    for ln, text in enumerate(lines):
        if text.strip():
            counts[kinds[ln] or "prod"] += 1
    # all mod declarations with optional #[path]
    decls = []
    for mm in re.finditer(r'(#\[path\s*=\s*"s*"\]\s*)?(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_0-9]+)\s*;', code):
        p = None
        if mm.group(1):
            l0 = line_of(mm.start())
            om = re.search(r'#\[path\s*=\s*"([^"]+)"\]', "\n".join(src.split("\n")[l0:l0 + 3]))
            p = om.group(1) if om else None
        decls.append((mm.group(2), p))
    return counts, mods, file_kind, decls


def main():
    root = os.path.abspath(sys.argv[1])
    args = sys.argv[2:]
    depth = int(args[args.index("--depth") + 1]) if "--depth" in args else 3
    top = int(args[args.index("--top") + 1]) if "--top" in args else 40
    files = {}
    for dp, dn, fn in os.walk(root):
        dn[:] = sorted(d for d in dn if d not in SKIP_DIRS)
        for f in sorted(fn):
            if f.endswith(".rs"):
                p = os.path.join(dp, f)
                with open(p, encoding="utf-8", errors="replace") as fh:
                    files[p] = analyse(p, fh.read())
    # propagate gated mods to files
    forced = {}

    def mod_dir(p):
        b = os.path.basename(p)
        return os.path.dirname(p) if b in ("mod.rs", "lib.rs", "main.rs") else os.path.join(os.path.dirname(p), b[:-3])

    def force(p, kind, seen):
        if p in seen or p not in files:
            return
        seen.add(p)
        order = {"test": 3, "diag": 2, "host": 1}
        if p not in forced or order[kind] > order[forced[p]]:
            forced[p] = kind
        for name, pth in files[p][3]:
            for c in children(p, name, pth):
                force(c, kind, seen)

    def children(p, name, pth):
        if pth:
            base = os.path.dirname(p)
            return [os.path.normpath(os.path.join(base, pth))]
        d = mod_dir(p)
        return [os.path.join(d, name + ".rs"), os.path.join(d, name, "mod.rs")]

    for p, (counts, mods, fk, decls) in files.items():
        if fk:
            force(p, fk, set())
        for name, pth in decls:
            if name in mods:
                for c in children(p, name, pth):
                    force(c, mods[name], set())
    rows = []
    for p, (counts, mods, fk, decls) in files.items():
        c = dict(counts)
        if p in forced:
            k = forced[p]
            order = {"prod": 0, "host": 1, "diag": 2, "test": 3}
            moved = {x: 0 for x in c}
            for x, v in c.items():
                tgt = x if order[x] >= order[k] else k
                moved[tgt] += v
            c = moved
        rows.append((os.path.relpath(p, root), c))
    tot = {"prod": 0, "test": 0, "diag": 0, "host": 0}
    groups = {}
    for rel, c in rows:
        parts = rel.split(os.sep)
        key = os.sep.join(parts[:depth]) if len(parts) > depth else rel
        g = groups.setdefault(key, {"prod": 0, "test": 0, "diag": 0, "host": 0, "files": 0})
        g["files"] += 1
        for x in tot:
            tot[x] += c[x]
            g[x] += c[x]
    print(f"root {root}\nfiles {len(rows)}  prod {tot['prod']}  test {tot['test']}  diag {tot['diag']}  host {tot['host']}  all {sum(tot.values())}")
    print(f"\n{'group':60} {'files':>5} {'prod':>7} {'diag':>6} {'host':>6} {'test':>7}")
    for key, g in sorted(groups.items(), key=lambda kv: -(kv[1]['prod'] + kv[1]['diag'] + kv[1]['host']))[:top]:
        print(f"{key:60} {g['files']:5} {g['prod']:7} {g['diag']:6} {g['host']:6} {g['test']:7}")
    if "--json" in args:
        with open(args[args.index("--json") + 1], "w") as fh:
            json.dump({"total": tot, "files": {rel: c for rel, c in rows}}, fh, indent=0)


if __name__ == "__main__":
    main()
