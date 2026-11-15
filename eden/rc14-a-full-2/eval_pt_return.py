#!/usr/bin/env python3
"""a-full-2: apply afull2.py's `pt_return_verdict` (the corrected a2-pt check, rows a069cf32+) to a saved diag log, offline.
Usage: eval_pt_return.py <afull2.py> <practice-diagnostics log> [<log> ...]
The kept group comes from the log's `window kept across reload owner=0 group=N kinds=[24, 25, 26]` line; `since_keep` is
every line from it on, `back` every line from the first later `request owner=0 kind=0x24 group=N` on."""
import ast
import json
import re
import sys
from pathlib import Path

source = Path(sys.argv[1]).read_text()
tree = ast.parse(source)
func = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'pt_return_verdict')
namespace = {'re': re}
exec(compile(ast.Module(body=[func], type_ignores=[]), sys.argv[1], 'exec'), namespace)
lines = []
for path in sys.argv[2:]:
    lines += Path(path).read_text(errors='replace').splitlines()
keep = next((i for i, t in enumerate(lines)
             if re.search(r'\[GroupSwap\] window kept across reload owner=0 group=\d+ kinds=\[24, 25, 26\]', t)), None)
if keep is None:
    sys.exit('no kept PT window in the log')
group = int(re.search(r'group=(\d+)', lines[keep]).group(1))
pick = next((i for i in range(keep, len(lines))
             if re.search(r'\[GroupSwap\] request owner=0 kind=0x2[456] group=%d ' % group, lines[i])), len(lines))
verdict = namespace['pt_return_verdict']((group, [0x24, 0x25, 0x26]), lines[keep:], lines[pick:])
print(json.dumps({'kept_line': lines[keep][:160], 'pick_line': lines[pick][:160] if pick < len(lines) else None,
                  **verdict, 'result': 'PASS' if not verdict['pt_problems'] else 'FAIL'}, indent=1))
