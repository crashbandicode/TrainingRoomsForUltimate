#!/usr/bin/python3
"""Static stack analysis of the plugin's ELF (the `.so` cargo-skyline turns into the NRO), stream/stack 2026-11-02.

Per function: the bytes its prologue moves SP down by (the straight-line prologue simulated, including LLVM's inline
stack-probe loop `sub x9, sp, #…; … mov sp, x9`). Call graph: direct `bl`, and tail `b` to a function start. Per entry: the
deepest path, frames summed. Indirect calls (`blr`, vtables, `fmt` trait objects) and imports (SDK) are not followed, so
every number is a lower bound; the measured high-water marks of the diag build cover the rest (`model::thread_stacks`).

    python3 -I tools/stack_depth.py <elf> frames [--min N]       functions with frames of at least N bytes (default 4096)
    python3 -I tools/stack_depth.py <elf> entry <regex> [--path]  deepest path from each matching function
    python3 -I tools/stack_depth.py <elf> table [<thread_stacks.rs>]
        each row of model::thread_stacks against the ELF: thread_start's own frame plus the deepest path of the row's
        spawn shim; exit 1 when a row's static_bound is below that or its stack is below the required size.

The ELF is `<target>/aarch64-skyline-switch/release/libtraining_rooms.so` of a diag build.
"""
import re
import subprocess
import sys

OBJDUMP = "/usr/bin/llvm-objdump-21"

FUNC_RE = re.compile(r"^([0-9a-f]+) <(.+)>:$")
INSN_RE = re.compile(r"^\s*([0-9a-f]+):\s+(\S+)\s*(.*)$")
IMM_RE = re.compile(r"#(-?0x[0-9a-f]+|-?\d+)")
SDK_RESERVE = 16 * 1024


def imm(text):
    m = IMM_RE.search(text)
    if not m:
        return None
    v = int(m.group(1), 0)
    if "lsl #12" in text:
        v <<= 12
    return v


def parse(text):
    """objdump -d text -> {address: {"addr", "name", "insns": [(address, op, args)]}}."""
    funcs = {}
    cur = None
    for line in text.splitlines():
        m = FUNC_RE.match(line)
        if m:
            cur = {"addr": int(m.group(1), 16), "name": m.group(2), "insns": []}
            funcs[cur["addr"]] = cur
            continue
        if cur is None:
            continue
        m = INSN_RE.match(line)
        if m:
            cur["insns"].append((int(m.group(1), 16), m.group(2), m.group(3)))
    return funcs


def frame_of(insns):
    """Bytes the prologue moves SP down by: the deepest point of the straight-line prologue."""
    sp = 0
    x9 = None
    deepest = 0
    for _, op, args in insns[:96]:
        a = args.replace(" ", "")
        if op == "sub" and a.startswith("sp,sp,#"):
            sp += imm(args) or 0
        elif op == "add" and a.startswith("sp,sp,#"):
            sp -= imm(args) or 0
        elif op in ("stp", "str") and "]!" in a and ",[sp,#-" in a:
            sp += -imm(args.split("[sp")[1])
        elif op == "sub" and a.startswith("x9,sp,#"):
            x9 = sp + (imm(args) or 0)
        elif op == "sub" and a.startswith("x9,x9,#") and x9 is not None:
            x9 += imm(args) or 0
        elif op == "mov" and a == "sp,x9" and x9 is not None:
            sp = x9
        elif op in ("bl", "blr", "ret", "br"):
            break
        deepest = max(deepest, sp)
    return max(deepest, sp)


def target(args):
    m = re.match(r"(?:0x)?([0-9a-f]+)\s*<(.+)>", args.strip())
    if not m:
        return None
    return int(m.group(1), 16)


def link(funcs):
    starts = set(funcs)
    for addr, f in funcs.items():
        f["frame"] = frame_of(f["insns"])
        calls, indirect = set(), 0
        for _, op, args in f["insns"]:
            if op in ("bl", "b"):
                t = target(args)
                if t in starts and t != addr and not funcs[t]["name"].endswith("@plt"):
                    calls.add(t)
            elif op == "blr":
                indirect += 1
        f["calls"], f["indirect"] = calls, indirect
    return funcs


def analyse(elf):
    out = subprocess.run([OBJDUMP, "-d", "--no-show-raw-insn", "--demangle", elf], capture_output=True, text=True, check=True).stdout
    return link(parse(out))


def deepest(funcs, root):
    """(bytes, path) of the deepest direct-call path from `root`; recursion is cut at the repeated function."""
    memo, onstack = {}, set()
    sys.setrecursionlimit(100000)

    def go(a):
        if a in memo:
            return memo[a]
        if a in onstack:
            return 0, []
        onstack.add(a)
        best, path = 0, []
        for c in funcs[a]["calls"]:
            d, p = go(c)
            if d > best:
                best, path = d, p
        onstack.discard(a)
        res = (funcs[a]["frame"] + best, [a] + path)
        memo[a] = res
        return res

    return go(root)


ROW_RE = re.compile(r'row\(ThreadId::(\w+),\s*"([^"]+)",\s*(\d+)\s*\*\s*KIB,\s*([\d_]+),\s*([\d_]+),\s*"[^"]*",\s*"([^"]+)"')


def table_rows(text):
    return [(m.group(1), m.group(2), int(m.group(3)) * 1024, int(m.group(4).replace("_", "")), int(m.group(5).replace("_", "")), m.group(6))
            for m in ROW_RE.finditer(text)]


def required(static_bound, measured):
    need = max(static_bound, measured)
    total = need + need // 4 + SDK_RESERVE
    return -(-total // 4096) * 4096


def check_table(funcs, rows):
    """Lines of the report and the number of failures."""
    start = next((f for f in funcs.values() if f["name"].endswith("::thread_start")), None)
    base = start["frame"] if start else 0
    shims = [f for f in funcs.values() if "spawn_unchecked<" in f["name"] and "call_once::{shim" in f["name"]]
    lines, bad = [f"thread_start frame {base} B"], 0
    for ident, name, size, static_bound, measured, symbol in rows:
        hits = [f for f in shims if symbol in f["name"]]
        if not hits:
            lines.append(f"{name:20s} size={size:7d} no shim in this ELF (feature off)")
            continue
        depth = max(base + deepest(funcs, f["addr"])[0] for f in hits)
        need = required(max(depth, static_bound), measured)
        ok = static_bound >= depth and size >= need
        bad += not ok
        lines.append(f"{name:20s} size={size:7d} static={depth:7d} table_static={static_bound:7d} measured={measured:7d} "
                     f"required={need:7d} {'ok' if ok else 'FAIL'}")
    return lines, bad


def main():
    elf, cmd = sys.argv[1], sys.argv[2]
    funcs = analyse(elf)
    if cmd == "frames":
        n = int(sys.argv[sys.argv.index("--min") + 1]) if "--min" in sys.argv else 4096
        for f in sorted(funcs.values(), key=lambda f: -f["frame"]):
            if f["frame"] < n:
                break
            print(f"{f['frame']:7d} 0x{f['addr']:08x} {f['name'][:200]}")
    elif cmd == "entry":
        rx = re.compile(sys.argv[3])
        for f in funcs.values():
            if rx.search(f["name"]):
                total, path = deepest(funcs, f["addr"])
                print(f"== {total:7d} B  {f['name'][:220]}")
                if "--path" in sys.argv:
                    for a in path:
                        g = funcs[a]
                        print(f"   {g['frame']:7d}  blr={g['indirect']:<3d} {g['name'][:200]}")
    elif cmd == "table":
        path = sys.argv[3] if len(sys.argv) > 3 else "src/model/thread_stacks.rs"
        lines, bad = check_table(funcs, table_rows(open(path).read()))
        print("\n".join(lines))
        sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
