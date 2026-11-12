#!/usr/bin/env python3
"""rc.9 coverage (docs/COVERAGE-rc9.md): product-code line coverage of the host test builds.

    cargo llvm-cov --no-fail-fast --no-report                 # run the instrumented tests (own, fresh CARGO_TARGET_DIR)
    python3 -I tools/coverage.py export <target dir> <out.lcov>
    python3 -I tools/coverage.py report <out.lcov> . [--files N] [--tsv out.tsv]

`export` merges the raw profiles and exports lcov with the toolchain's own llvm-profdata / llvm-cov (rustup component
`llvm-tools`), not through `cargo llvm-cov report`: that report drops every path containing `/tests/`, and so the lines of
`src/` files reached through a by-path test crate (`tests/../src/model/x.rs`). Use a fresh target directory: binaries of
an earlier run would be exported with zero counts.

`report` counts product lines only. Test code is reported apart: lines inside test-gated modules (`#[cfg(test)] mod`),
test-only files (the targets of such modules and their children), `src/test_table.rs` and everything under `tests/`.
Files outside `src/` and `tests/` (the vendored crates, the layout builder) are listed separately and not counted. A file
that llvm-cov lists under two spellings (`src/engine/../model/x.rs`, `src/model/x.rs`) is merged per line (most hits).
"""
import collections
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import test_cut  # noqa: E402


def export(target, out):
    sysroot = subprocess.run(['rustc', '--print', 'sysroot'], capture_output=True, text=True, check=True).stdout.strip()
    host = subprocess.run(['rustc', '-vV'], capture_output=True, text=True, check=True).stdout.split('host: ')[1].split('\n')[0]
    tools = os.path.join(sysroot, 'lib', 'rustlib', host, 'bin')
    cov = os.path.join(target, 'llvm-cov-target')
    raw = sorted(os.path.join(cov, f) for f in os.listdir(cov) if f.endswith('.profraw'))
    listing = out + '.profraw.txt'
    open(listing, 'w').write('\n'.join(raw) + '\n')
    subprocess.run([os.path.join(tools, 'llvm-profdata'), 'merge', '-sparse', '-f', listing, '-o', out + '.profdata'], check=True)
    deps = os.path.join(cov, 'debug', 'deps')
    objs = sorted(os.path.join(deps, f) for f in os.listdir(deps) if '.' not in f and os.access(os.path.join(deps, f), os.X_OK) and os.path.isfile(os.path.join(deps, f)))
    cmd = [os.path.join(tools, 'llvm-cov'), 'export', '-format=lcov', '-instr-profile=' + out + '.profdata',
           r'-ignore-filename-regex=(/\.cargo/|/rustc/|/toolchain/tools/|/registry/)', objs[0]] + ['-object=' + o for o in objs[1:]]
    with open(out, 'w') as f:
        subprocess.run(cmd, stdout=f, check=True)
    print(f'profraw={len(raw)} objects={len(objs)} lcov={os.path.getsize(out)} bytes')


if len(sys.argv) >= 4 and sys.argv[1] == 'export':
    export(sys.argv[2], sys.argv[3])
    sys.exit(0)
if len(sys.argv) < 4 or sys.argv[1] != 'report':
    sys.exit(__doc__)
lcov, tree = sys.argv[2], os.path.abspath(sys.argv[3])

def resolve(decl_file, d):
    b = os.path.dirname(decl_file)
    stem = os.path.splitext(os.path.basename(decl_file))[0]
    if d['path']:
        p = os.path.normpath(os.path.join(b, d['path']))
        return p if os.path.exists(p) else None
    for c in (os.path.join(b, d['name'] + '.rs'), os.path.join(b, d['name'], 'mod.rs'),
              os.path.join(b, stem, d['name'] + '.rs'), os.path.join(b, stem, d['name'], 'mod.rs')):
        if os.path.exists(c):
            return c
    return None


test_files = {'src/test_table.rs'}
test_spans = {}
queue = []
for dp, _, fns in os.walk(os.path.join(tree, 'src')):
    for fn in fns:
        if not fn.endswith('.rs'):
            continue
        p = os.path.join(dp, fn)
        text = open(p, encoding='utf-8').read()
        spans = []
        for d in test_cut.find_test_mods(text):
            if d['inline']:
                spans.append((text.count('\n', 0, d['start']) + 1, text.count('\n', 0, d['end']) + 1))
            else:
                t = resolve(p, d)
                if t:
                    queue.append(t)
        if spans:
            test_spans[os.path.relpath(p, tree)] = spans
while queue:
    p = queue.pop()
    rel = os.path.relpath(p, tree)
    if rel in test_files:
        continue
    test_files.add(rel)
    text = open(p, encoding='utf-8').read()
    for m in re.finditer(r'(?:#\[path\s*=\s*"([^"]+)"\]\s*)?(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;', text):
        c = resolve(p, {'path': m.group(1), 'name': m.group(2)})
        if c:
            queue.append(c)

lines = collections.defaultdict(dict)
cur = None
for raw in open(lcov, encoding='utf-8', errors='replace'):
    if raw.startswith('SF:'):
        p = os.path.normpath(raw[3:].strip())
        cur = os.path.relpath(p, tree) if p.startswith(tree + '/') else None
    elif raw.startswith('DA:') and cur:
        ln, hits = raw[3:].strip().split(',')[:2]
        ln, hits = int(ln), int(hits)
        lines[cur][ln] = max(lines[cur].get(ln, 0), hits)

prod = {}
vend = collections.defaultdict(lambda: [0, 0])
t_tot = t_cov = 0
missed = {}
for rel, da in lines.items():
    if not rel.startswith(('src/', 'tests/')):
        g = '/'.join(rel.split('/')[:2])
        vend[g][0] += len(da)
        vend[g][1] += sum(1 for h in da.values() if h)
        continue
    if rel.startswith('tests/') or rel in test_files:
        t_tot += len(da)
        t_cov += sum(1 for h in da.values() if h)
        continue
    spans = test_spans.get(rel, [])
    c = v = 0
    miss = []
    for ln, h in da.items():
        if any(a <= ln <= b for a, b in spans):
            t_tot += 1
            t_cov += 1 if h else 0
            continue
        c += 1
        if h:
            v += 1
        else:
            miss.append(ln)
    if c:
        prod[rel] = (c, v)
        missed[rel] = sorted(miss)
tot = sum(c for c, _ in prod.values())
cov = sum(v for _, v in prod.values())
print(f'PRODUCT files={len(prod)} lines={tot} covered={cov} missed={tot - cov} pct={100 * cov / tot:.2f}')
print(f'test code (not counted): lines={t_tot} covered={t_cov}')
for g, (c, v) in sorted(vend.items()):
    print(f'vendored/other (not counted): {g} lines={c} covered={v} pct={100 * v / c:.2f}')


def group(rel):
    parts = rel.split('/')
    if len(parts) >= 4:
        return '/'.join(parts[:3])
    if len(parts) == 3:
        return parts[0] + '/' + parts[1] + '/*.rs'
    return rel if len(parts) == 2 and parts[0] == 'src' else '/'.join(parts[:2])


groups = collections.defaultdict(lambda: [0, 0])
for rel, (c, v) in prod.items():
    g = group(rel)
    groups[g][0] += c
    groups[g][1] += v
for g, (c, v) in sorted(groups.items(), key=lambda kv: -(kv[1][0] - kv[1][1])):
    print(f'{c - v:6d} missed  {100 * v / c:6.2f}%  {c:6d} lines  {g}')
if '--files' in sys.argv:
    n = int(sys.argv[sys.argv.index('--files') + 1])
    print('--- files by missed lines')
    for rel, (c, v) in sorted(prod.items(), key=lambda kv: -(kv[1][0] - kv[1][1]))[:n]:
        print(f'{c - v:6d} missed  {100 * v / c:6.2f}%  {c:6d}  {rel}')
if '--tsv' in sys.argv:
    with open(sys.argv[sys.argv.index('--tsv') + 1], 'w') as o:
        for rel, (c, v) in sorted(prod.items()):
            o.write(f'{rel}\t{c}\t{v}\n')
if '--missed' in sys.argv:
    rel = sys.argv[sys.argv.index('--missed') + 1]
    m = missed.get(rel, [])
    out = []
    for ln in m:
        if out and out[-1][1] == ln - 1:
            out[-1][1] = ln
        else:
            out.append([ln, ln])
    print(rel, 'missed ranges:', ' '.join(f'{a}-{b}' if a != b else str(a) for a, b in out))
