#!/usr/bin/env python3
"""Host tests of tools/rows_tsv.py (guard-table rows as data, docs/CODE-REDUCTION-rc10.md): python3 -I tools/test_rows_tsv.py"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rows_tsv  # noqa: E402

GUARD = ', '.join('0x%02x' % (i * 3 % 256) for i in range(64))
SRC = '''use super::*;
pub const OTHER: [u8; 2] = [1, 2];
#[rustfmt::skip]
pub const ROWS: &[Seam] = &[
    // a comment, with a comma
    Seam { label: "a.b, c", group: "a", offset: 0x10,
        kind: SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid },
        guard: [%s],
        feature: "", scopes: Scopes::ALL, read_only: true, per_frame: false, sharing: Sharing::None,
        note: "x: \\"y\\" // not a comment" },
    Seam { label: "d", group: "a", offset: OFFSET,
        kind: SeamKind::Called,
        guard: [%s],
        feature: "", scopes: Scopes(Scopes::TRAINING.0 | 2), read_only: false, per_frame: false,
        sharing: Sharing::OwnHook { hook: "a.b" }, note: "" },
];
pub static EMPTY: &[Seam] = &[];
''' % (GUARD, GUARD)


def check(name, got, want):
    if got != want:
        print(f'FAIL {name}:\n  {got!r}\n  {want!r}')
        sys.exit(1)


tables = list(rows_tsv.tables_of(SRC))
check('tables', [t[0] for t in tables], ['ROWS'])
name, amp, start, close_at, (struct, names, rows) = tables[0]
check('amp', (amp, SRC[start:start + 2], SRC[close_at]), ('&', '&[', ']'))
check('struct', struct, 'Seam')
check('fields', names, ['label', 'group', 'offset', 'kind', 'guard', 'feature', 'scopes', 'read_only', 'per_frame', 'sharing', 'note'])
check('row count', len(rows), 2)
check('comment', rows[0][0], ['a comment, with a comma'])
hexes = ''.join('%02x' % (i * 3 % 256) for i in range(64))
check('row 0', rows[0][1], ['"a.b, c"', '"a"', '0x10', 'SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid }', hexes,
                            '""', 'Scopes::ALL', 'true', 'false', 'Sharing::None', '"x: \\"y\\" // not a comment"'])
check('row 1', rows[1][1][2:4] + rows[1][1][6:7] + rows[1][1][9:10],
      ['OFFSET', 'SeamKind::Called', 'Scopes(Scopes::TRAINING.0 | 2)', 'Sharing::OwnHook { hook: "a.b" }'])
text = rows_tsv.tsv(struct, names, rows)
lines = text.split('\n')
check('header', lines[0], '#! Seam\t' + '\t'.join(names))
check('comment line', lines[1], '# a comment, with a comma')
check('cells', len(lines[2].split('\t')), len(names))
check('line count', len(lines), 5)
print('ok')
