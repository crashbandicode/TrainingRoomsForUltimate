#!/usr/bin/env python3
"""rc.10 S1 (docs/CODE-REDUCTION-rc10.md): guard-table rows as data.

A guard table is an array of struct literals (`Seam { .. }`, `Site { .. }`, `CommonSite { .. }`). Its rows live in
`data/tables/<file stem>.<TABLE>.tsv`; `build.rs` turns each file back into the array (`$OUT_DIR/tables/<name>.rs`), and the
source keeps `include!(concat!(env!("OUT_DIR"), "/tables/<name>.rs"))` where the literal was.

    python3 -I tools/rows_tsv.py extract <file.rs>...     move every table of the files into data/tables/ (edits the files)
    python3 -I tools/rows_tsv.py table <file.rs> <out.tsv> [TABLE]   only write the TSV of one table (a generator's output)

TSV: line 1 is `#! <Struct>` then the field names, tab-separated. A row is one line, a cell per field. A cell is the
field's Rust value as written (an offset, an enum path, a quoted string), except `guard`: its 64 bytes as 128 hex
digits. A line starting with `# ` is a comment and stays with the row below it.
"""
import os
import re
import sys

ROW_TYPES = ('Seam', 'Site', 'CommonSite')
HEAD = re.compile(r'pub(?:\([a-z]+\))?\s+(?:static|const)\s+([A-Z_0-9]+)\s*:\s*(?:&\s*\[\s*(\w+)\s*\]|\[\s*(\w+)\s*;[^\]]*\])\s*=\s*(&?)\s*\[')


def mask_of(text):
    """True where a character is code (not in a comment or a string)."""
    mask = [True] * len(text)
    i, n = 0, len(text)
    while i < n:
        if text.startswith('//', i):
            j = text.find('\n', i)
            j = n if j < 0 else j
            for k in range(i, j):
                mask[k] = False
            i = j
        elif text[i] == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == '\\' else 1
            for k in range(i + 1, j):
                mask[k] = False
            i = j + 1
        else:
            i += 1
    return mask


def close_of(text, mask, at):
    """Index of the bracket that closes the one at `at`."""
    pairs = {'[': ']', '{': '}', '(': ')'}
    stack = []
    for i in range(at, len(text)):
        if not mask[i]:
            continue
        c = text[i]
        if c in pairs:
            stack.append(pairs[c])
        elif c in pairs.values():
            assert stack.pop() == c, (i, c)
            if not stack:
                return i
    raise ValueError('unbalanced')


def split_top(body, mask, sep):
    """Split at top-level `sep` characters."""
    out, depth, start = [], 0, 0
    for i, c in enumerate(body):
        if not mask[i]:
            continue
        if c in '[{(':
            depth += 1
        elif c in ']})':
            depth -= 1
        elif c == sep and depth == 0:
            out.append((start, i))
            start = i + 1
    out.append((start, len(body)))
    return out


def squeeze(value, vmask):
    """One line: whitespace runs outside strings become one space."""
    out = []
    for c, m in zip(value, vmask):
        if m and c.isspace():
            if out and out[-1] != ' ':
                out.append(' ')
        else:
            out.append(c)
    return ''.join(out).strip()


def parse_table(text, mask, lo, hi):
    """Rows of the array body text[lo:hi]: (struct, field names, list of (comments, cells))."""
    rows, comments, struct, names = [], [], None, None
    i = lo
    while i < hi:
        if text[i].isspace() or (text[i] == ',' and mask[i]):
            i += 1
        elif text.startswith('//', i):
            j = text.find('\n', i)
            comments.append(text[i + 2:j].strip())
            i = j
        else:
            m = re.compile(r'(\w+)\s*\{').match(text, i)
            assert m and m.group(1) in ROW_TYPES, text[i:i + 60]
            struct = struct or m.group(1)
            assert struct == m.group(1)
            brace = m.end() - 1
            end = close_of(text, mask, brace)
            body, bmask = text[brace + 1:end], mask[brace + 1:end]
            fields = []
            for a, b in split_top(body, bmask, ','):
                piece, pmask = body[a:b], bmask[a:b]
                if not piece.strip():
                    continue
                # a comment inside a row goes above the row
                while True:
                    cm = re.match(r'\s*//([^\n]*)\n', piece)
                    if not cm:
                        break
                    comments.append(cm.group(1).strip())
                    piece, pmask = piece[cm.end():], pmask[cm.end():]
                colon = next(k for k, c in enumerate(piece) if c == ':' and pmask[k])
                name = piece[:colon].strip()
                value, vmask = piece[colon + 1:], pmask[colon + 1:]
                if name == 'guard':
                    hexes = re.findall(r'0x([0-9a-fA-F]{2})\b', value)
                    assert len(hexes) == 64 and value.strip().startswith('['), value[:80]
                    cell = ''.join(hexes).lower()
                else:
                    cell = squeeze(value, vmask)
                assert '\t' not in cell and '\n' not in cell, cell
                fields.append((name, cell))
            if names is None:
                names = [n for n, _ in fields]
            assert names == [n for n, _ in fields], (names, fields)
            rows.append((comments, [c for _, c in fields]))
            comments = []
            i = end + 1
    assert not comments, comments
    return struct, names, rows


def tables_of(text):
    mask = mask_of(text)
    for m in HEAD.finditer(text):
        if not mask[m.start()] or (m.group(2) or m.group(3)) not in ROW_TYPES:
            continue
        open_at = m.end() - 1
        close_at = close_of(text, mask, open_at)
        if not text[open_at + 1:close_at].strip():
            continue
        start = m.start(4) if m.group(4) else open_at
        yield m.group(1), m.group(4), start, close_at, parse_table(text, mask, open_at + 1, close_at)


def tsv(struct, names, rows):
    lines = ['#! ' + struct + '\t' + '\t'.join(names)]
    for comments, cells in rows:
        lines += ['# ' + c for c in comments]
        lines.append('\t'.join(cells))
    return '\n'.join(lines) + '\n'


def main():
    cmd = sys.argv[1]
    if cmd == 'table':
        text = open(sys.argv[2], encoding='utf-8').read()
        found = [t for t in tables_of(text) if len(sys.argv) < 5 or t[0] == sys.argv[4]]
        assert len(found) == 1, [t[0] for t in found]
        open(sys.argv[3], 'w', encoding='utf-8').write(tsv(*found[0][4]))
        print('wrote', sys.argv[3], len(found[0][4][2]), 'rows')
        return
    repo = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    out_dir = os.path.join(repo, 'data', 'tables')
    os.makedirs(out_dir, exist_ok=True)
    total = 0
    for path in sys.argv[2:]:
        text = open(path, encoding='utf-8').read()
        stem = os.path.basename(path)[:-3]
        found = list(tables_of(text))
        for name, amp, start, close_at, (struct, names, rows) in reversed(found):
            key = f'{stem}.{name}'
            assert not os.path.exists(os.path.join(out_dir, key + '.tsv')), key
            open(os.path.join(out_dir, key + '.tsv'), 'w', encoding='utf-8').write(tsv(struct, names, rows))
            text = text[:start] + amp + f'include!(concat!(env!("OUT_DIR"), "/tables/{key}.rs"))' + text[close_at + 1:]
            total += len(rows)
            print(f'{key}: {len(rows)} rows of {struct}')
        open(path, 'w', encoding='utf-8').write(text)
    print('rows moved', total)


if __name__ == '__main__':
    main()
