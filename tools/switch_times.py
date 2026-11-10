#!/usr/bin/env python3
"""rc.7 target-first: each fighter switch's time to playable, from the diag log (`practice-diagnostics*.log`).

    python3 -I tools/switch_times.py <log> [<log> ...] [--owner 0|1] [--json out.json]

A switch starts at the role's `[CharacterRoster] request handed ... role=Player|Cpu` line. It is playable at the first
ready line after it: `[BodyBank] ready owner=N` (a bank window or a group) or the adapter's own `ready` / `bank ready`
line (a canonical Reload has no bank). The time is the difference of the two lines' millisecond ticks. The path comes
from the lines in between:
- `target-first`: the Reload built the picked fighter alone;
- `window N`: a Reload built an N-body window;
- `canonical`: a Reload with no bank;
- `in-window`: a warm member change;
- `group`: a warm exchange into a built group.

The background builds that completed before the role's next request are listed with each switch
(`[GroupSwap] background built owner=N ... ok=true`). Logs are merged by their `#seq` numbers, so the `.previous`
copy can be passed too. Numbers are copied from the lines; nothing is estimated.
"""
import json
import re
import sys

LINE = re.compile(r"^(\d+) #(\d+) (.*)$")
REQUEST = re.compile(r"\[CharacterRoster\] request handed to the ordinary Mii owner role=(Player|Cpu) selection=(\S+)")
ADAPTER = {0: "MiiOrdinaryPlayer", 1: "MiiOrdinaryCpu"}


def read_file(path):
    out = []
    with open(path, errors="replace") as handle:
        for raw in handle:
            m = LINE.match(raw.rstrip("\n"))
            if m:
                out.append((int(m.group(1)), int(m.group(2)), m.group(3)))
    return out


def sessions(paths):
    """The logs' lines per boot session, each deduplicated by seq and in seq order. A file continues a session when its
    lines share or continue its sequence (the rotated `.previous` copy of the same boot); a file whose sequence restarts
    below a session's lines and overlaps them is another boot (sequences start at #1 on every boot)."""
    groups = []
    for lines in (read_file(p) for p in paths):
        if not lines:
            continue
        ticks = {seq: tick for tick, seq, _ in lines}
        home = next((g for g in groups if all(g["lines"].get(s, (t,))[0] == t for s, t in ticks.items())), None)
        if home is None:
            home = {"lines": {}}
            groups.append(home)
        for line in lines:
            home["lines"].setdefault(line[1], line)
    return [[g["lines"][k] for k in sorted(g["lines"])] for g in groups]


def read_lines(paths):
    """(tick_ms, seq, text) of the logs' one boot session; several sessions are an error here (use `sessions`)."""
    found = sessions(paths)
    if len(found) > 1:
        raise ValueError("the logs hold %d boot sessions" % len(found))
    return found[0] if found else []


def switches_by_session(paths, owner=0):
    """[`switches`] of each boot session, in the order the paths first name them, each row with its session index."""
    rows = []
    for index, lines in enumerate(sessions(paths)):
        for row in switches(lines, owner):
            row["session"] = index
            rows.append(row)
    return rows


def owner_of(role):
    return 0 if role == "Player" else 1


def switches(lines, owner=0):
    """One dict per switch of `owner`, in log order."""
    ready = re.compile(r"\[BodyBank\] ready owner=%d index=\d+ kind=0x([0-9a-f]+)|\[%s\] (?:bank )?ready ticket=.* target=0x([0-9a-f]+)"
                       % (owner, ADAPTER[owner]))
    built = re.compile(r"\[GroupSwap\] background built owner=%d group=(\d+) ok=true .*?kinds=\[([0-9a-f, ]*)\]" % owner)
    out, current = [], None
    for tick, seq, text in lines:
        m = REQUEST.search(text)
        if m and owner_of(m.group(1)) == owner:
            if current is not None:
                out.append(current)
            current = {"request_ms": tick, "seq": seq, "selection": m.group(2), "path": None, "kind": None,
                       "ready_ms": None, "ms": None, "lines": [], "background": []}
            continue
        if current is None:
            continue
        if current["ms"] is None:
            current["lines"].append(text)
            r = ready.search(text)
            if r:
                current["ready_ms"] = tick
                current["ms"] = tick - current["request_ms"]
                current["kind"] = int(r.group(1) or r.group(2), 16)
                current["path"] = path_of(current["lines"], owner)
            continue
        b = built.search(text)
        if b:
            current["background"].append({"ms": tick, "group": int(b.group(1)),
                                          "kinds": [int(k, 16) for k in b.group(2).replace(" ", "").split(",") if k]})
    if current is not None:
        out.append(current)
    for s in out:
        del s["lines"]
    return out


def path_of(texts, owner):
    joined = "\n".join(texts)
    if "[BodyBank] target first owner=%d " % owner in joined:
        return "target-first"
    m = re.search(r"\[BodyBank\] build owner=%d .*count: (\d+)" % owner, joined)
    if m:
        return "window %s" % m.group(1)
    if "[GroupSwap] swap owner=%d " % owner in joined:
        return "group"
    if "[BodyBank] reuse owner=%d " % owner in joined:
        return "in-window"
    if "bank_build=false" in joined or "[ProcessMemory] window owner=%d " % owner in joined:
        return "canonical"
    return "other"


def table(rows):
    lines = ["| # | session | request (ms) | target | path | to playable (ms) | background built before the next switch |",
             "| --- | --- | --- | --- | --- | --- | --- |"]
    for i, s in enumerate(rows):
        bg = "; ".join("group %d %s at +%d ms" % (b["group"], "/".join("%#x" % k for k in b["kinds"]), b["ms"] - s["ready_ms"])
                       for b in s["background"]) or "-"
        kind = "-" if s["kind"] is None else "%#x" % s["kind"]
        ms = "not ready" if s["ms"] is None else str(s["ms"])
        lines.append("| %d | %d | %d | %s | %s | %s | %s |" % (i + 1, s.get("session", 0), s["request_ms"], kind,
                                                              s["path"] or "-", ms, bg))
    return "\n".join(lines)


def main(argv):
    owner, out_json, paths = 0, None, []
    i = 0
    while i < len(argv):
        if argv[i] == "--owner":
            owner = int(argv[i + 1]); i += 2
        elif argv[i] == "--json":
            out_json = argv[i + 1]; i += 2
        else:
            paths.append(argv[i]); i += 1
    if not paths:
        print(__doc__)
        return 2
    rows = switches_by_session(paths, owner)
    print(table(rows))
    if out_json:
        with open(out_json, "w") as handle:
            json.dump(rows, handle, indent=1)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
