#!/usr/bin/env python3
"""rc.7 boot crash on the operator's Switch (2026-11-11 02:01Z): the Switch kernel zeroes x8-x17 on SVC return (Eden
does not), and the compiler had kept a word address in x10 across an inline `svc #0x34` (hos-lock's WaitForAddress
without `clobber_abi("C")`), then `ldaxr wzr, [x10]` read address 0. Eden can never show this class, so this is the
proof on the built ELF: after every `svc`, no path inside the function reads one of x8-x17 (or w8-w17) before writing
it.

    python3 tools/svc_clobber_check.py <libtraining_rooms.so> [--list]

From each `svc`, every path through the function is followed (fall-through, and the targets of `b`, `b.cond`,
`cbz`/`cbnz`, `tbz`/`tbnz` inside the function) with the set of registers the kernel zeroed and nothing has written yet.
A path ends at `ret`/`br`/an unconditional branch out of the function, and at a call (`bl`/`blr`: x8-x17 are the
callee's to clobber, so the compiler never relies on them across one). A read of a register still in the set is a
violation. Exit 0: none; 1: violations (listed); 2: the ELF could not be read."""
import re
import subprocess
import sys

OBJDUMP = "/usr/bin/llvm-objdump-21"
ZEROED = frozenset(range(8, 18))
FUNC = re.compile(r"^([0-9a-f]+) <(.+)>:$")
INSN = re.compile(r"^\s*([0-9a-f]+):\s+(\S+)(?:\s+(.*?))?\s*$")
REG = re.compile(r"\b[xw]([0-9]|[12][0-9]|30)\b")
TARGET = re.compile(r"^(?:0x)?([0-9a-f]+)(?:\s|$)")

STORE_PREFIX = ("st",)  # st, stp, str*, stur*, stlr*, stxr*, stlxr*, stnp, stg ...
COMPARE = {"cmp", "cmn", "tst", "ccmp", "ccmn", "fcmp", "fcmpe", "fccmp", "fccmpe"}
NO_REG_WRITE = {"prfm", "prfum", "dmb", "dsb", "isb", "nop", "hint", "yield", "wfe", "wfi", "sev", "sevl", "clrex",
                "msr", "brk", "hlt", "udf", "sys", "dc", "ic", "tlbi", "bti", "pacibsp", "autibsp", "paciasp",
                "autiasp", "xpaclri", "csdb", "esb", "psb", "ssbb", "pssbb", "sb", "dgh"}
READS_DEST = {"movk", "bfi", "bfxil", "bfm", "bfc"}
CONDITIONAL = {"cbz", "cbnz", "tbz", "tbnz"}


def split_operands(args):
    out, depth, cur = [], 0, ""
    for ch in args:
        if ch in "[{":
            depth += 1
        elif ch in "]}":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


def regs(text):
    return {int(m.group(1)) for m in REG.finditer(text)}


def effects(op, args):
    """(reads, writes) as sets of general register numbers (x/w), for one instruction."""
    args = re.sub(r"<[^>]*>", "", (args or "").split("//")[0]).strip()
    ops = split_operands(args) if args else []
    base = op.split(".")[0]
    if base in NO_REG_WRITE or not ops:
        return regs(args), set()
    if base in COMPARE or base in ("b", "bl", "br", "blr", "ret", "braa", "brab", "blraa", "blrab", "retaa", "retab") \
            or base in CONDITIONAL or op.startswith("b."):
        return regs(args), set()
    writeback = any(o.endswith("!") for o in ops) or (len(ops) >= 2 and ops[-1].startswith("#") and "[" in ops[-2])
    mem = [o for o in ops if o.startswith("[")]
    base_regs = set().union(*(regs(o) for o in mem)) if mem else set()
    if base.startswith(STORE_PREFIX):
        # Exclusive stores write their status register (the first operand): st(l)xr(b|h)?, st(l)xp.
        if re.match(r"^stl?x[rp][bh]?$", base):
            return regs(", ".join(ops[1:])), regs(ops[0]) | (base_regs if writeback else set())
        return regs(args), base_regs if writeback else set()
    if base.startswith(("cas", "swp", "ld")) and re.match(r"^(cas|swp|ld(add|clr|eor|set|smax|smin|umax|umin))", base):
        # Atomics: cas Rs, Rt, [Rn] reads all, writes Rs; swp/ldadd Rs, Rt, [Rn] reads Rs, Rn, writes Rt.
        if base.startswith("cas"):
            return regs(args), regs(ops[0])
        return regs(ops[0]) | base_regs, regs(ops[1]) if len(ops) > 1 else set()
    if base in ("ldp", "ldnp", "ldpsw", "ldxp", "ldaxp", "ldiapp"):
        writes = regs(ops[0]) | (regs(ops[1]) if len(ops) > 1 else set())
        return base_regs, writes | (base_regs if writeback else set())
    if base.startswith("ld"):
        # ldr/ldur/ldar/ldaxr/ldxr/ldrb… Rt, [Rn{, Rm}]{!} or [Rn], #imm: reads the address registers, writes Rt.
        return base_regs, regs(ops[0]) | (base_regs if writeback else set())
    dest = regs(ops[0])
    reads = set().union(*(regs(o) for o in ops[1:])) if len(ops) > 1 else set()
    if base in READS_DEST:
        reads |= dest
    return reads, dest


def parse(elf):
    text = subprocess.run([OBJDUMP, "-d", "--no-show-raw-insn", "--demangle", elf], capture_output=True, text=True,
                          check=True).stdout
    funcs, cur = [], None
    for line in text.splitlines():
        m = FUNC.match(line)
        if m:
            cur = {"name": m.group(2), "insns": []}
            funcs.append(cur)
            continue
        m = INSN.match(line)
        if m and cur is not None:
            cur["insns"].append((int(m.group(1), 16), m.group(2), m.group(3) or ""))
    return funcs


def successors(i, insns, index):
    addr, op, args = insns[i]
    base = op.split(".")[0]
    nxt = [i + 1] if i + 1 < len(insns) else []
    if base in ("ret", "br", "eret", "retaa", "retab", "braa", "brab") or base in ("bl", "blr", "blraa", "blrab") \
            or base in ("brk", "udf", "hlt"):
        return []
    target = None
    last = split_operands(args)[-1] if args else ""
    t = TARGET.match(last.strip())
    if t and (base == "b" or op.startswith("b.") or base in CONDITIONAL):
        target = index.get(int(t.group(1), 16))
    if base == "b":
        return [target] if target is not None else []
    if op.startswith("b.") or base in CONDITIONAL:
        return nxt + ([target] if target is not None else [])
    return nxt


def check(elf):
    funcs = parse(elf)
    sites, violations = [], []
    for f in funcs:
        insns = f["insns"]
        index = {a: i for i, (a, _, _) in enumerate(insns)}
        for i, (addr, op, args) in enumerate(insns):
            if op != "svc":
                continue
            sites.append((f["name"], addr, args))
            seen, stack, steps = set(), [(i + 1, ZEROED)], 0
            while stack and steps < 20000:
                j, live = stack.pop()
                steps += 1
                if j >= len(insns) or (j, live) in seen or not live:
                    continue
                seen.add((j, live))
                a, o, ar = insns[j]
                if o == "svc":
                    continue
                reads, writes = effects(o, ar)
                bad = reads & live
                if bad:
                    violations.append((f["name"], addr, a, o, ar, sorted(bad)))
                    continue
                live = frozenset(live - writes)
                for k in successors(j, insns, index):
                    stack.append((k, live))
    return sites, violations


ALLOW = __file__.rsplit("/", 1)[0] + "/svc_clobber_allow.txt"


def normal(op, args):
    return " ".join((op + " " + re.sub(r"<[^>]*>", "", args).split("//")[0]).split())


def allowed():
    """Reviewed reads (`function | instruction | register | reason`): reads the analysis cannot tell from a dead value.
    Every svc asm declares clobber_abi("C") (tests/svc_clobber_abi.rs), so the compiler never keeps a value live in
    x8-x17 across one; a read it still reports lies past a join or a loop where the value is dead on the svc's path.
    Each entry names the exact instruction, so a changed function reports again and needs a new review."""
    out = {}
    try:
        lines = open(ALLOW).read().splitlines()
    except OSError:
        return out
    for line in lines:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        parts = [p.strip() for p in line.split("|")]
        if len(parts) >= 4 and parts[3]:
            out[(parts[0], " ".join(parts[1].split()), parts[2])] = parts[3]
    return out


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    try:
        sites, violations = check(sys.argv[1])
    except (OSError, subprocess.CalledProcessError) as error:
        print("cannot read %s: %s" % (sys.argv[1], error))
        return 2
    allow = allowed()
    reviewed, open_ = [], []
    for v in violations:
        name, svc, at, op, args, bad = v
        keys = [(name, normal(op, args), "x%d" % r) for r in bad]
        (reviewed if all(k in allow for k in keys) else open_).append(v)
    print("svc sites: %d; reads of x8-x17 after an svc before a write: %d (reviewed in %s: %d; open: %d)"
          % (len(sites), len(violations), ALLOW.rsplit("/", 1)[1], len(reviewed), len(open_)))
    if "--list" in sys.argv:
        for name, addr, args in sites:
            print("  svc %s at %#x in %s" % (args, addr, name[:140]))
    for name, svc, at, op, args, bad in reviewed:
        print("REVIEWED %s: svc at %#x, then %#x %s reads %s: %s" % (name[:140], svc, at, normal(op, args),
              ["x%d" % r for r in bad], allow[(name, normal(op, args), "x%d" % bad[0])][:120]))
    for name, svc, at, op, args, bad in open_:
        print("VIOLATION %s: svc at %#x, then %#x %s reads %s" % (name[:140], svc, at, normal(op, args), ["x%d" % r for r in bad]))
    return 1 if open_ else 0


if __name__ == "__main__":
    sys.exit(main())
