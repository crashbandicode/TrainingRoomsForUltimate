"""Implementation code lines in Rust: no blank lines, no comments, no tests.

Provenance: the operator's counter /home/intpa/smash-training-lobby/tools/sloc3.py (2026-11-02, sha256
b569131d2f25ec901f7f71ff1ed920a6fc9147161f28b5c8e82a6ba6780de6b4), copied 2026-11-02 by the slim stream.
The counting rules (strip_comments, drop_cfg_test, count) are unchanged, so totals stay comparable with the
operator's fork count. Changes: vendored code under third_party/ is not ours and is excluded from the total
(reported on its own line); a `tree` mode counts the working tree's tracked and untracked-not-ignored files
(what tools/size_gate.py gates); depth 0 lists single files; the module is importable (main() only as a script).
Two measurement modes the lead asked for (2026-11-02), for reports only until the operator rules on the method:
`--test-fixed` also drops `#[cfg(all(test, ...))]` items and files declared only by a test-gated `mod x;` line;
`--normalize` (git mode) counts the revision laid out by rustfmt's defaults in a temporary `git archive` copy.

Excluded: files under tests/, benches/, examples/, third_party/; items gated by #[cfg(test)] (their brace block);
// and /* */ comments (doc comments included); blank lines.
usage: python3 -I sloc.py git <repo> <rev> [depth] [--test-fixed] [--normalize]
       python3 -I sloc.py dir <path> [depth] [--test-fixed]  |  python3 -I sloc.py tree <repo> [depth] [--test-fixed]"""
import collections
import io
import pathlib
import posixpath
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile

TEST_DIRS = ('tests/', 'benches/', 'examples/')
VENDORED_DIRS = ('third_party/',)


def strip_comments(text):
    """Return the text with comments removed (string-literal aware enough for Rust sources)."""
    out = []
    i, n = 0, len(text)
    depth = 0
    in_str = False
    while i < n:
        c = text[i]
        if depth:
            if text.startswith('/*', i):
                depth += 1
                i += 2
            elif text.startswith('*/', i):
                depth -= 1
                i += 2
            else:
                if c == '\n':
                    out.append('\n')
                i += 1
            continue
        if in_str:
            out.append(c)
            if c == '\\' and i + 1 < n:
                out.append(text[i + 1])
                i += 2
                continue
            if c == '"':
                in_str = False
            i += 1
            continue
        if text.startswith('//', i):
            j = text.find('\n', i)
            i = n if j < 0 else j
            continue
        if text.startswith('/*', i):
            depth = 1
            i += 2
            continue
        if c == '"' and not (i > 0 and text[i - 1] == "'" and i + 1 < n and text[i + 1] == "'"):
            in_str = True
        out.append(c)
        i += 1
    return ''.join(out)


CFG_TEST = re.compile(r'#\[cfg\(\s*test\s*\)\]')
# The test-fixed rule (proposed fix, 2026-11-02; not the operator's rule until the operator rules): `cfg(all(test, ...))`
# items are tests too, and a file declared only by such a gated `mod x;` line is test code.
CFG_TEST_ANY = re.compile(r'#\[cfg\(\s*(?:test\s*|all\(\s*test\s*,[^\]]*)\)\]')
TEST_MOD = re.compile(CFG_TEST_ANY.pattern + r'\s*(?:#\[path\s*=\s*"([^"]+)"\]\s*)?(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;')


def drop_cfg_test(text, gate=CFG_TEST):
    """Remove #[cfg(test)]-gated items: from the attribute to the end of the item's brace block or ';'."""
    out = []
    pos = 0
    for m in gate.finditer(text):
        if m.start() < pos:
            continue
        out.append(text[pos:m.start()])
        i = m.end()
        n = len(text)
        while i < n and text[i] not in '{;':
            i += 1
        if i < n and text[i] == ';':
            pos = i + 1
            continue
        depth = 0
        while i < n:
            if text[i] == '{':
                depth += 1
            elif text[i] == '}':
                depth -= 1
                if depth == 0:
                    i += 1
                    break
            i += 1
        pos = i
    out.append(text[pos:])
    return ''.join(out)


def count(text, test_fixed=False):
    body = drop_cfg_test(strip_comments(text), CFG_TEST_ANY if test_fixed else CFG_TEST)
    return sum(1 for line in body.splitlines() if line.strip())


def test_module_files(items):
    """Files whose only declaration is a test-gated `mod x;` line (the test-fixed rule)."""
    names = {name for name, _ in items}
    found = set()
    for name, text in items:
        here = posixpath.dirname(name)
        stem = posixpath.splitext(posixpath.basename(name))[0]
        base = here if stem in ('mod', 'lib', 'main') else posixpath.join(here, stem)
        for m in TEST_MOD.finditer(strip_comments(text)):
            path, module = m.group(1), m.group(2)
            if path:
                candidates = [posixpath.normpath(posixpath.join(here, path))]
            else:
                candidates = [posixpath.join(base, f'{module}.rs'), posixpath.join(base, module, 'mod.rs')]
            found.update(c for c in candidates if c in names)
    return found


def group(path, depth):
    if depth == 0:
        return path
    parts = path.split('/')
    return '/'.join(parts[:depth]) if len(parts) > depth else '/'.join(parts[:-1]) or '(root)'


def in_dirs(name, dirs):
    return any(f'/{d}' in f'/{name}' for d in dirs)


def rust_names(names):
    """Rust sources that are not test files; vendored ones are kept and split off by totals()."""
    return [n for n in names if n.endswith('.rs') and not in_dirs(n, TEST_DIRS)]


def items_git(repo, rev):
    names = subprocess.run(['git', '-C', str(repo), 'ls-tree', '-r', '--name-only', rev],
                           capture_output=True, text=True, check=True).stdout.split('\n')
    return [(name, subprocess.run(['git', '-C', str(repo), 'show', f'{rev}:{name}'], capture_output=True,
                                  check=True).stdout.decode('utf-8', 'replace')) for name in rust_names(names)]


def items_dir(root):
    root = pathlib.Path(root)
    items = []
    for p in root.rglob('*.rs'):
        rel = p.relative_to(root).as_posix()
        if rel.startswith('target/') or '/target/' in rel or in_dirs(rel, TEST_DIRS):
            continue
        items.append((rel, p.read_text('utf-8', 'replace')))
    return items


def items_tree(repo):
    """The working tree as git sees it (tracked plus untracked-not-ignored); a plain directory walk without git."""
    repo = pathlib.Path(repo)
    try:
        names = subprocess.run(['git', '-C', str(repo), 'ls-files', '--cached', '--others', '--exclude-standard'],
                               capture_output=True, text=True, check=True).stdout.split('\n')
    except (OSError, subprocess.CalledProcessError):
        return items_dir(repo)
    return [(n, (repo / n).read_text('utf-8', 'replace')) for n in rust_names(names) if (repo / n).is_file()]


def edition_of(root, rel):
    """The edition of the nearest Cargo.toml above `rel` (its `edition` or its workspace's), else 2021."""
    folder = (root / rel).parent
    while True:
        manifest = folder / 'Cargo.toml'
        if manifest.is_file():
            found = re.search(r'^\s*edition\s*=\s*"(\d{4})"', manifest.read_text('utf-8', 'replace'), re.M)
            if found:
                return found.group(1)
        if folder == root:
            return '2021'
        folder = folder.parent


def items_normalized(repo, rev):
    """`rev` laid out by rustfmt's defaults: `git archive` into a temporary folder, `rustfmt` on every Rust file there
    (by edition, the archive's own toolchain pin), then counted. Nothing in the repository changes."""
    with tempfile.TemporaryDirectory(prefix='sloc-normalize-') as tmp:
        root = pathlib.Path(tmp)
        archive = subprocess.run(['git', '-C', str(repo), 'archive', rev], capture_output=True, check=True).stdout
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(root, filter='data')
        files = [p.relative_to(root).as_posix() for p in root.rglob('*.rs')]
        by_edition = collections.defaultdict(list)
        for rel in files:
            by_edition[edition_of(root, rel)].append(rel)
        rustfmt = shutil.which('rustfmt') or str(pathlib.Path.home() / '.cargo' / 'bin' / 'rustfmt')
        for edition, rels in sorted(by_edition.items()):
            done = subprocess.run([rustfmt, '--edition', edition, *rels], cwd=root, capture_output=True, text=True)
            if done.returncode != 0:
                print(f'sloc: rustfmt (edition {edition}) exit {done.returncode}: {done.stderr.strip()[:400]}', file=sys.stderr)
        names = set(rust_names(files))
        return [(name, text) for name, text in items_dir(root) if name in names]


def totals(items, depth, test_fixed=False):
    """(per-group counts of our code, vendored total)."""
    skip = test_module_files(items) if test_fixed else set()
    ours, vendored = collections.Counter(), 0
    for name, text in items:
        if name in skip:
            continue
        if in_dirs(name, VENDORED_DIRS):
            vendored += count(text, test_fixed)
        else:
            ours[group(name, depth)] += count(text, test_fixed)
    return ours, vendored


def main(argv):
    flags = {a for a in argv if a.startswith('--')}
    argv = [a for a in argv if not a.startswith('--')]
    mode = argv[1]
    if mode == 'git' and '--normalize' in flags:
        items, rest = items_normalized(argv[2], argv[3]), argv[4:]
    elif mode == 'git':
        items, rest = items_git(argv[2], argv[3]), argv[4:]
    elif mode == 'dir':
        items, rest = items_dir(argv[2]), argv[3:]
    else:
        items, rest = items_tree(argv[2]), argv[3:]
    ours, vendored = totals(items, int(rest[0]) if rest else 1, '--test-fixed' in flags)
    for g, n in sorted(ours.items(), key=lambda kv: -kv[1]):
        print(f'{g:40s} {n:8d}')
    print(f'{"TOTAL (implementation, no tests)":40s} {sum(ours.values()):8d}')
    print(f'{"third_party/ (vendored, not counted)":40s} {vendored:8d}')


if __name__ == '__main__':
    main(sys.argv)
