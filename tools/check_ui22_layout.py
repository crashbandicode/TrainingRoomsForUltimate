"""Verify the UI22 delta against prior recipes and emit full traversal fixtures.

Consumes private operator-owned stock archives; never commits their bytes.
Usage: check_ui22_layout.py STOCK_DIR PRIOR_ASSETS GENERATED_DIR
"""
import hashlib
import json
import struct
import sys
from pathlib import Path


def sections(data):
    assert data[:4] == b"FLYT"
    at = struct.unpack_from("<H", data, 6)[0]
    out = []
    for _ in range(struct.unpack_from("<H", data, 16)[0]):
        size = struct.unpack_from("<I", data, at + 4)[0]
        assert size >= 8 and at + size <= len(data)
        out.append(data[at:at + size])
        at += size
    assert at == len(data)
    return out


def sarc(data):
    assert data[:4] == b"SARC" and data[6:8] == b"\xff\xfe"
    head = struct.unpack_from("<H", data, 4)[0]
    size, offset = struct.unpack_from("<II", data, 8)
    assert size == len(data) and data[head:head + 4] == b"SFAT"
    count = struct.unpack_from("<H", data, head + 6)[0]
    names = head + 12 + count * 16 + 8
    out = {}
    for i in range(count):
        _, attr, start, end = struct.unpack_from("<IIII", data, head + 12 + i * 16)
        at = names + (attr & 0xffffff) * 4
        name = data[at:data.index(0, at)].decode()
        assert attr >> 24 == 1 and name not in out and start <= end <= len(data) - offset
        out[name] = data[offset + start:offset + end]
    return out


def fnv(data):
    value = 0xcbf29ce484222325
    for byte in data:
        value = ((value ^ byte) * 0x100000001b3) & 0xffffffffffffffff
    return value


def apply(stock, recipe):
    assert recipe[:4] == b"TRA1"
    sl, sf, ol, of = struct.unpack_from("<IQIQ", recipe, 4)
    assert sl == len(stock) and sf == fnv(stock)
    out = bytearray()
    at = 28
    while at < len(recipe):
        op, size = struct.unpack_from("<BI", recipe, at)
        at += 5
        if op == 0:
            start = struct.unpack_from("<I", recipe, at)[0]
            at += 4
            assert start + size <= len(stock)
            out.extend(stock[start:start + size])
        else:
            assert op == 2 and at + size <= len(recipe)
            out.extend(recipe[at:at + size])
            at += size
    assert len(out) == ol and fnv(out) == of
    return bytes(out)


def topology(blob):
    stack, rows, records = [], [], {}
    last = None
    for section in sections(blob):
        tag = section[:4]
        if tag == b"pas1":
            assert last is not None
            stack.append(last)
        elif tag == b"pae1":
            stack.pop()
        elif tag in (b"pan1", b"pic1", b"txt1", b"prt1", b"wnd1", b"bnd1"):
            name = section[12:36].split(b"\0")[0].decode()
            assert name not in records
            parent = stack[-1] if stack else None
            rows.append((parent, name))
            records[name] = section
            last = name
    assert not stack
    return rows, records


def check(old, new):
    oldrows, oldpanes = topology(old)
    rows, panes = topology(new)
    added = {f"{prefix}{i:02}" for prefix in ("ILFB_", "ILSB_") for i in range(15)}
    assert set(panes) - set(oldpanes) == added
    assert all(panes[name] == data for name, data in oldpanes.items())
    assert [(p, n) for p, n in rows if n not in added] == oldrows
    # No material/font/texture or other non-hierarchy section is changed.
    pane_tags = {b"pan1", b"pic1", b"txt1", b"prt1", b"wnd1", b"bnd1", b"pas1", b"pae1"}
    assert [s for s in sections(old) if s[:4] not in pane_tags] == [s for s in sections(new) if s[:4] not in pane_tags]
    for parent, name in rows:
        if name not in added:
            continue
        text = name.replace("ILFB_", "ILF_").replace("ILSB_", "ILS_")
        siblings = [n for p, n in rows if p == parent]
        assert siblings[siblings.index(name) + 1] == text
        assert parent == "TrModInputLog" + name[-2:]
        p, t = panes[name], panes[text]
        assert p[:4] == b"pic1" and p[8] & 2 and not p[8] & 1 and p[10] == 255
        assert p[9] == t[9], "same pane origin as the existing text"
        assert p[44:84] == t[44:84], "transform and exact text footprint"
    assert len(rows) < 4096
    return rows, len(oldpanes), added


def main():
    stockdir, prior, generated = map(Path, sys.argv[1:])
    reports = {}
    for kind in ("training", "waiting"):
        stock = (stockdir / f"info_{kind}.arc").read_bytes()
        oldraw = apply(stock, (prior / f"info_{kind}.tra1").read_bytes())
        newraw = (generated / kind / "target.arc").read_bytes()
        recipe = (generated / kind / f"info_{kind}.tra1").read_bytes()
        assert apply(stock, recipe) == newraw
        old, new = sarc(oldraw), sarc(newraw)
        member = f"blyt/info_{kind}.bflyt"
        assert old.keys() == new.keys()
        assert all(old[name] == data for name, data in new.items() if name != member)
        rows, preserved, added = check(old[member], new[member])
        # Negative control: changing an existing text's alpha must fail.
        damaged = bytearray(new[member])
        needle = b"ILF_00\0"
        at = damaged.index(needle) - 2
        damaged[at] ^= 1
        try:
            check(old[member], damaged)
        except AssertionError:
            pass
        else:
            raise AssertionError("existing-pane mutation escaped preservation oracle")
        indices = {name: i for i, (_, name) in enumerate(rows)}
        (generated / f"{kind}.tsv").write_text("".join(f"{indices[parent] if parent else 0}\t{name}\n" for parent, name in rows))
        assert len(newraw) <= 9 * 1024 * 1024
        reports[kind] = {"old_bytes": len(oldraw), "new_bytes": len(newraw), "delta": len(newraw) - len(oldraw), "recipe_bytes": len(recipe), "panes": len(rows), "preserved_panes": preserved, "added_panes": len(added), "target_sha256": hashlib.sha256(newraw).hexdigest(), "recipe_sha256": hashlib.sha256(recipe).hexdigest(), "nonlayout_members_preserved": len(old) - 1}
    reports["boot_buffers"] = {"count": 2, "each_bytes": max(reports[k]["new_bytes"] for k in ("training", "waiting")), "total_delta": 2 * (max(reports[k]["new_bytes"] for k in ("training", "waiting")) - max(reports[k]["old_bytes"] for k in ("training", "waiting")))}
    (generated / "preservation.json").write_text(json.dumps(reports, indent=2) + "\n")
    print(json.dumps(reports))


if __name__ == "__main__":
    main()
