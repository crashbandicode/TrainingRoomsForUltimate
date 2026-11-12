#!/usr/bin/env python3
"""rc.9 test cut (docs/TEST-CUT-LEDGER-rc9.md): remove or wrap test-gated modules of a Rust source text.

Used by tools/carry_pkg4.py and tools/carry_pkg6.py for the manifest edits `cut_tests`, `wrap_tests` and `wrap_file`, so a
re-sync from the fork stage re-derives the carried files without the cut unit tests:

* `cut(text, names)`      removes each `#[cfg(test)] mod <name> { ... }` / `mod <name>;` item (with its attributes).
* `wrap(text, names, m)`  wraps the body of each inline test module in `<m>! { ... }` (the lib-only unit-test macro).
* `wrap_file(text, m)`    wraps a whole out-of-line test file, after its leading `//!` lines and inner attributes.

Only test-gated `mod` items are touched; comments, strings and char literals are skipped by a small lexer.

    python3 -I tools/test_cut.py selftest
"""
import re
import sys

def lex_spans(src):
    """Return a bytearray mask: 1 where the char is code (not comment / string / char literal)."""
    n = len(src)
    mask = bytearray(b'\x01') * n
    i = 0
    while i < n:
        c = src[i]
        if src.startswith('//', i):
            j = src.find('\n', i)
            j = n if j < 0 else j
            for k in range(i, j):
                mask[k] = 0
            i = j
        elif src.startswith('/*', i):
            depth = 1
            j = i + 2
            while j < n and depth:
                if src.startswith('/*', j):
                    depth += 1
                    j += 2
                elif src.startswith('*/', j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            for k in range(i, j):
                mask[k] = 0
            i = j
        elif c == '"' or (c in 'rb' and re.match(r'(?:b?r#*"|b")', src[i:i + 12]) and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == '_'))):
            m = re.match(r'(b?)(r?)(#*)"', src[i:i + 12])
            raw = m.group(2) == 'r'
            hashes = m.group(3)
            j = i + m.end()
            if raw:
                end = '"' + hashes
                e = src.find(end, j)
                j = n if e < 0 else e + len(end)
            else:
                while j < n and src[j] != '"':
                    j += 2 if src[j] == '\\' else 1
                j += 1
            for k in range(i, min(j, n)):
                mask[k] = 0
            i = j
        elif c == "'":
            # char literal or lifetime
            m = re.match(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|.)|[^\\'])'", src[i:i + 14])
            if m:
                for k in range(i, i + m.end()):
                    mask[k] = 0
                i += m.end()
            else:
                i += 1
        else:
            i += 1
    return mask


ATTR = re.compile(r'#\[')


def find_test_mods(src):
    """Yield dicts for each test-gated module item: start (of first attr), end, name, inline, path_attr, body span."""
    mask = lex_spans(src)
    out = []
    for m in re.finditer(r'#\[cfg\((?:test|all\(test\b[^\]]*)\)\]', src):
        if not mask[m.start()]:
            continue
        # attribute run: go back over preceding attributes / doc comments on directly preceding lines
        start = m.start()
        while True:
            line_start = src.rfind('\n', 0, start) + 1
            if src[line_start:start].strip():
                break
            prev_end = line_start - 1
            if prev_end < 0:
                break
            prev_start = src.rfind('\n', 0, prev_end) + 1
            prev = src[prev_start:prev_end].strip()
            if prev.startswith('#[') and prev.endswith(']') or prev.startswith('///'):
                start = prev_start + (len(src[prev_start:prev_end]) - len(src[prev_start:prev_end].lstrip()))
            else:
                break
        # forward: more attributes, then `mod name`
        pos = m.end()
        path_attr = None
        while True:
            ws = re.match(r'\s*', src[pos:])
            pos += ws.end()
            if src.startswith('#[', pos):
                depth = 0
                j = pos
                while j < len(src):
                    if mask[j]:
                        if src[j] == '[':
                            depth += 1
                        elif src[j] == ']':
                            depth -= 1
                            if depth == 0:
                                break
                    j += 1
                attr = src[pos:j + 1]
                pm = re.match(r'#\[path\s*=\s*"([^"]+)"\]', attr)
                if pm:
                    path_attr = pm.group(1)
                pos = j + 1
            elif src.startswith('//', pos):
                pos = src.find('\n', pos)
            else:
                break
        mm = re.match(r'(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*([;{])', src[pos:])
        if not mm:
            continue
        name = mm.group(1)
        if mm.group(2) == ';':
            end = pos + mm.end()
            out.append(dict(start=start, end=end, name=name, inline=False, path=path_attr, body=None))
        else:
            open_at = pos + mm.end() - 1
            depth = 0
            j = open_at
            while j < len(src):
                if mask[j]:
                    if src[j] == '{':
                        depth += 1
                    elif src[j] == '}':
                        depth -= 1
                        if depth == 0:
                            break
                j += 1
            out.append(dict(start=start, end=j + 1, name=name, inline=True, path=path_attr, body=(open_at + 1, j)))
    # drop nested (a test mod inside another test mod)
    out.sort(key=lambda d: d['start'])
    top = []
    for d in out:
        if top and d['start'] < top[-1]['end']:
            continue
        top.append(d)
    return top


def cut(text, names):
    mods = [d for d in find_test_mods(text) if d['name'] in names]
    found = {d['name'] for d in mods}
    missing = [n for n in names if n not in found]
    if missing:
        raise SystemExit(f'cut_tests: no test-gated module named {missing}')
    for d in reversed(mods):
        s, e = d['start'], d['end']
        while e < len(text) and text[e] in ' \t':
            e += 1
        if e < len(text) and text[e] == '\n':
            e += 1
        ls = text.rfind('\n', 0, s) + 1
        if not text[ls:s].strip():
            s = ls
        text = text[:s] + text[e:]
    text = re.sub(r'\n{3,}\Z', '\n', text)
    if not text.endswith('\n'):
        text += '\n'
    return text


def wrap(text, names, macro):
    for name in names:
        d = next((x for x in find_test_mods(text) if x['name'] == name and x['inline']), None)
        if d is None:
            raise SystemExit(f'wrap_tests: no inline test-gated module named {name}')
        a, b = d['body']
        # leading inner attributes and inner doc comments stay outside the macro
        while True:
            m = re.match(r'[ \t]*\n|[ \t]*(?://!|#!\[)[^\n]*\n', text[a:b])
            if not m or (not m.group(0).strip() and not re.match(r'(?:[ \t]*\n)*[ \t]*(?://!|#!\[)', text[a:b])):
                break
            a += m.end()
        lead = '' if text[a - 1] == '\n' and text[d['body'][0]:a].strip() else '\n'
        text = text[:a] + f'{lead}    {macro}! {{' + ('\n' if not lead else '') + text[a:b].rstrip() + '\n    }\n' + text[b:]
    return text


def wrap_file(text, macro):
    lines = text.split('\n')
    k = 0
    while k < len(lines) and (lines[k].startswith('//!') or lines[k].startswith('#![') or not lines[k].strip()):
        k += 1
    head, rest = '\n'.join(lines[:k]), '\n'.join(lines[k:])
    return (head + '\n' if head else '') + f'{macro}! {{\n' + rest.rstrip() + '\n}\n'


TEST_FN = re.compile(r'((?:\s*(?:#\[[^\]\n]*\]|//[^\n]*))*)\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+(\w+)\s*\(\s*\)\s*(->[^{;]*)?\{')
TABLE_FN = 'case_table'


def _scopes(text, mask):
    """Inline module bodies of `text` as (body_start, body_end), innermost last, plus the root (0, len)."""
    out = [(0, len(text))]
    for m in re.finditer(r'\bmod\s+\w+\s*\{', text):
        if not mask[m.start()]:
            continue
        open_at = m.end() - 1
        depth = 0
        j = open_at
        while j < len(text):
            if mask[j]:
                if text[j] == '{':
                    depth += 1
                elif text[j] == '}':
                    depth -= 1
                    if depth == 0:
                        break
            j += 1
        out.append((open_at + 1, j))
    return out


def tabulate_region(text, keep=()):
    """Turn the `#[test]` functions of each module scope in `text` into cases of one `#[test] fn case_table()` per scope.
    A case runs on its own thread (`crate::test_table::run`), as the harness ran it, and asserts what it asserted. Left as
    they are: a scope with fewer than two tests, a test under `cfg`, `ignore` or `should_panic`, a name in `keep` (a case a lint
    only accepts inside a `#[test]`), a test that re-executes the binary (`current_exe`), and one this scanner cannot read."""
    mask = lex_spans(text)
    scopes = _scopes(text, mask)
    by_scope = {}
    edits = []  # (start, end, replacement)
    for m in re.finditer(r'#\[test\]', text):
        if not mask[m.start()]:
            continue
        f = TEST_FN.match(text, m.end())
        if not f:
            continue
        # attributes on the lines directly above `#[test]`
        pre = ''
        ls = text.rfind('\n', 0, m.start()) + 1
        if text[ls:m.start()].strip():
            continue
        k = ls
        while k > 0:
            ps = text.rfind('\n', 0, k - 1) + 1
            line = text[ps:k - 1].strip()
            if line.startswith('#[') and line.endswith(']'):
                pre = line + pre
                k = ps
            else:
                break
        attrs = pre + f.group(1)
        if 'cfg' in attrs or 'ignore' in attrs:
            continue
        name = f.group(2)
        if name in keep:
            continue
        # a test that runs this binary again with `--exact <its name>` must stay a test of that name
        depth, j = 0, f.end() - 1
        while j < len(text):
            if mask[j]:
                if text[j] == '{':
                    depth += 1
                elif text[j] == '}':
                    depth -= 1
                    if depth == 0:
                        break
            j += 1
        if 'current_exe' in text[f.end():j]:
            continue
        if 'should_panic' in attrs:
            continue
        sp = expect = None
        call = f'{name} as fn()' if not f.group(3) else f'(|| {{ {name}().unwrap(); }}) as fn()'
        scope = max((s for s in scopes if s[0] <= m.start() < s[1]), key=lambda s: s[0])
        by_scope.setdefault(scope, []).append((m, f, sp, name, call, expect, k))
    for scope, tests in by_scope.items():
        if len(tests) < 2 or re.search(r'\bfn\s+' + TABLE_FN + r'\b', text[scope[0]:scope[1]]):
            continue
        rows = []
        for m, f, sp, name, call, expect, attr_start in tests:
            # drop the `#[test]` line
            ls = text.rfind('\n', 0, m.start()) + 1
            le = text.find('\n', m.end()) + 1
            if text[m.end():le].strip() == '':
                edits.append((ls, le, ''))
            else:
                edits.append((m.start(), m.end(), ''))
            if sp:
                am = re.search(r'#\[should_panic(?:\(\s*expected\s*=\s*"(?:[^"\\]|\\.)*"\s*\))?\]', text[attr_start:f.end()])
                a, b = attr_start + am.start(), attr_start + am.end()
                las = text.rfind('\n', 0, a) + 1
                lae = text.find('\n', b) + 1
                if text[las:a].strip() == '' and text[b:lae].strip() == '':
                    edits.append((las, lae, ''))
                else:
                    edits.append((a, b, ''))
            rows.append(f'("{name}", {call}),')
        ls = text.rfind('\n', 0, tests[0][0].start()) + 1
        ind = re.match(r'[ \t]*', text[ls:]).group(0)
        table = (f'\n{ind}/// rc.9: this module\'s cases in one table. Each case runs on its own thread, as the test harness ran it.\n'
                 f'{ind}#[test]\n{ind}fn {TABLE_FN}() {{\n{ind}    crate::test_table::run(&[\n'
                 + ''.join(f'{ind}        {r}\n' for r in rows) + f'{ind}    ]);\n{ind}}}\n')
        end = scope[1]
        if end == len(text):
            edits.append((end, end, ('' if text.endswith('\n') else '\n') + table))
        else:
            # before the scope's closing brace, on its own lines
            ce = text.rfind('\n', 0, end) + 1
            if text[ce:end].strip() == '':
                edits.append((ce, ce, table.lstrip('\n') if text[:ce].endswith('\n\n') else table))
            else:
                edits.append((end, end, table))
    for s, e, rep in sorted(edits, key=lambda x: (x[0], x[1]), reverse=True):
        text = text[:s] + rep + text[e:]
    return text


def table_tests(text, keep=()):
    """`tabulate_region` on the body of every top-level inline test-gated module."""
    for d in sorted((x for x in find_test_mods(text) if x['inline']), key=lambda x: x['start'], reverse=True):
        a, b = d['body']
        text = text[:a] + tabulate_region(text[a:b], keep) + text[b:]
    return text


def apply_edit(e, text):
    """One manifest edit of the test cut, or None when `e` is another kind of edit."""
    if 'cut_tests' in e:
        return cut(text, e['cut_tests'])
    if 'wrap_tests' in e:
        return wrap(text, e['wrap_tests'], e['macro'])
    if 'wrap_file' in e:
        return wrap_file(text, e['wrap_file'])
    if 'table_tests' in e:
        return table_tests(text, e.get('keep', ()))
    if 'table_file' in e:
        return tabulate_region(text, e.get('keep', ()))
    if 'insert_before_line' in e:
        lines = text.split('\n')
        at = e['insert_before_line'] - 1
        return '\n'.join(lines[:at]) + ('\n' if at else '') + e['text'] + '\n'.join(lines[at:])
    return None


def selftest():
    src = ('fn a() { let s = "#[cfg(test)] mod x {"; let c = \'{\'; }\n\n/// doc\n#[cfg(test)]\nmod tests {\n    use super::*;\n'
           '    #[test]\n    fn t() { assert_eq!("}", "}"); /* } */ }\n}\n\n#[cfg(test)]\n#[path = "b_tests.rs"]\nmod more;\n\nfn z() {}\n')
    names = [d['name'] for d in find_test_mods(src)]
    assert names == ['tests', 'more'], names
    out = cut(src, ['tests', 'more'])
    assert out == 'fn a() { let s = "#[cfg(test)] mod x {"; let c = \'{\'; }\n\n\n\nfn z() {}\n', repr(out)
    w = wrap(src, ['tests'], 'm')
    assert 'mod tests {\n    m! {\n    use super::*;' in w and w.count('m! {') == 1, w
    assert find_test_mods(w)[0]['name'] == 'tests'
    assert wrap_file('//! doc\n\nuse a;\n', 'm') == '//! doc\n\nm! {\nuse a;\n}\n'
    print('test_cut selftest ok')


if __name__ == '__main__' and sys.argv[1:] == ['selftest']:
    selftest()