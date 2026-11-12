#!/usr/bin/env python3
"""Host tests of tools/loc_raw.py (the raw line counter of docs/CODE-REDUCTION-rc10.md): python3 -I tools/test_loc_raw.py"""
import os
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import loc_raw  # noqa: E402


def counts(src):
    return loc_raw.analyse('x.rs', src)[0]


def check(name, got, want):
    if got != want:
        print(f'FAIL {name}: {got} != {want}')
        sys.exit(1)


# comments, blank lines and string contents are not code; a `//` inside a string is not a comment
check('comments', counts('// a\n\n/* b\n c */\nfn f() {\n    let s = "// not a comment";\n}\n'),
      {'prod': 3, 'test': 0, 'diag': 0, 'host': 0})
# a test-gated item is test code, with its attribute line
check('test item', counts('fn a() {}\n#[cfg(test)]\nfn b() {\n    a();\n}\n'), {'prod': 1, 'test': 4, 'diag': 0, 'host': 0})
check('test mod', counts('fn a() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n'),
      {'prod': 1, 'test': 5, 'diag': 0, 'host': 0})
# a diag-family feature is diag-only; a shipped feature is product
check('diag', counts('#[cfg(feature = "diag")]\nfn d() {}\n#[cfg(feature = "deluxe-sdk")]\nfn p() {}\n'),
      {'prod': 2, 'test': 0, 'diag': 2, 'host': 0})
check('host', counts('#[cfg(not(target_os = "switch"))]\nfn h() {}\n'), {'prod': 0, 'test': 0, 'diag': 0, 'host': 2})
# predicates
check('all(test, x)', loc_raw.classify_pred('all(test, feature = "x")'), 'test')
check('any(test, switch)', loc_raw.classify_pred('any(test, target_os = "switch")'), 'prod')
check('any(test, simulator)', loc_raw.classify_pred('any(test, feature = "simulator")'), 'diag')
check('not(test)', loc_raw.classify_pred('not(test)'), 'prod')

# a file reached only through a test-gated `mod name;` is test code
with tempfile.TemporaryDirectory() as root:
    os.makedirs(os.path.join(root, 'src'))
    open(os.path.join(root, 'src', 'lib.rs'), 'w').write('mod a;\n#[cfg(test)]\nmod a_tests;\n')
    open(os.path.join(root, 'src', 'a.rs'), 'w').write('pub fn a() {}\n')
    open(os.path.join(root, 'src', 'a_tests.rs'), 'w').write('fn t() {}\nfn u() {}\n')
    out = os.path.join(root, 'out.json')
    sys.argv = ['loc_raw.py', root, '--json', out, '--top', '0']
    stdout = sys.stdout
    sys.stdout = open(os.devnull, 'w')
    try:
        loc_raw.main()
    finally:
        sys.stdout = stdout
    import json
    total = json.load(open(out))['total']
    check('gated file', total, {'prod': 2, 'test': 4, 'diag': 0, 'host': 0})
print('ok')
