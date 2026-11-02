"""Census of the seam table's consumers (lead ruling 2026-11-02, override of D6 with proof).

A row has a consumer when any Rust source outside the row files (rows_*.rs, images.rs) and outside test code
(native_seams/tests.rs, tests/) contains: its label as a string literal; its offset as a hex literal (a hook
attribute or a call at that address); for a `needle.<x>` row, the constant `SITE_<X>`; or another row names it
(`Sharing::OwnHook { hook }`, `EntryOrBody { owner }`). Test-only references are listed apart.
usage: python3 -I seam_consumers.py <repo> [--groups g1,g2] [--json out]"""
import json, pathlib, re, sys

root = pathlib.Path(sys.argv[1])
seams = root / 'src/model/seams/native_seams'
ROW = re.compile(r'(Seam|ImageSeam) \{ label: "([^"]+)", group: "([^"]+)",(.*?)(?=\n\s*(?:Seam|ImageSeam) \{ label:|\n\];)', re.S)
rows = []
for f in sorted(list(seams.glob('rows_*.rs')) + [seams / 'images.rs']):
    text = f.read_text()
    for m in ROW.finditer(text):
        body = m.group(4)
        off = re.search(r'offset: (0x[0-9a-fA-F_]+)', body)
        rows.append({'file': f.name, 'label': m.group(2), 'group': m.group(3),
                     'offset': int(off.group(1).replace('_', ''), 16) if off else None,
                     'names': re.findall(r'(?:hook|owner): "([^"]+)"', body)})
row_files = {str(seams / r['file']) for r in rows}
code, tests = {}, {}
for p in root.rglob('*.rs'):
    if 'target' in p.parts or str(p) in row_files:
        continue
    rel = p.relative_to(root).as_posix()
    bucket = tests if rel.startswith('tests/') or rel.endswith('native_seams/tests.rs') or rel.endswith('_tests.rs') else code
    bucket[rel] = p.read_text(errors='replace')
named_by_rows = {n for r in rows for n in r['names']}

def offset_pattern(off):
    h = f'{off:x}'
    # 0x1234abc, 0x123_4abc, 0x1_234_abc ... : any underscores between the digits
    return re.compile(r'0x' + r'_?'.join(h) + r'\b', re.I)

def find(texts, label, off):
    hits = []
    lit = f'"{label}"'
    needle = 'SITE_' + label.split('.', 1)[1].upper() if label.startswith('needle.') else None
    pat = offset_pattern(off) if off else None
    for rel, t in texts.items():
        if lit in t:
            hits.append(f'{rel}: label')
        elif needle and re.search(r'\b' + needle + r'\b', t):
            hits.append(f'{rel}: {needle}')
        elif pat and pat.search(t):
            hits.append(f'{rel}: offset {off:#x}')
    return hits

out = {}
for r in rows:
    c = find(code, r['label'], r['offset'])
    if r['label'] in named_by_rows:
        c.append('another row names it')
    r['consumers'] = c
    r['test_refs'] = find(tests, r['label'], None)
    g = out.setdefault(r['group'], {'rows': 0, 'consumed': 0, 'files': set(), 'unconsumed': [], 'evidence': set()})
    g['rows'] += 1
    g['files'].add(r['file'])
    if c:
        g['consumed'] += 1
        g['evidence'].update(c)
    else:
        g['unconsumed'].append(r['label'])
for name, g in sorted(out.items(), key=lambda kv: (kv[1]['consumed'] > 0, kv[0])):
    print(f"{name:28s} rows={g['rows']:3d} consumed={g['consumed']:3d} files={','.join(sorted(g['files']))}"
          + (f"  e.g. {sorted(g['evidence'])[:2]}" if g['evidence'] else ''))
if '--json' in sys.argv:
    json.dump(rows, open(sys.argv[sys.argv.index('--json') + 1], 'w'), indent=1)
