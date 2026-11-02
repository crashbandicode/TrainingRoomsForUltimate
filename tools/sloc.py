"""Implementation code lines in Rust: no blank lines, no comments, no tests.

Provenance: the operator's counter /home/intpa/smash-training-lobby/tools/sloc3.py (2026-11-02, sha256
b569131d2f25ec901f7f71ff1ed920a6fc9147161f28b5c8e82a6ba6780de6b4), copied 2026-11-02 by the slim stream.
The counting rules (strip_comments, drop_cfg_test, count) are unchanged, so totals stay comparable with the
operator's fork count. Changes: vendored code under third_party/ is not ours and is excluded from the total
(reported on its own line); a `tree` mode counts the working tree's tracked and untracked-not-ignored files
(what tools/size_gate.py gates); depth 0 lists single files; the module is importable (main() only as a script).

Excluded: files under tests/, benches/, examples/, third_party/; items gated by #[cfg(test)] (their brace block);
// and /* */ comments (doc comments included); blank lines.
usage: python3 -I sloc.py git <repo> <rev> [depth]  |  python3 -I sloc.py dir <path> [depth]
       python3 -I sloc.py tree <repo> [depth]"""
import collections
import pathlib
import re
import subprocess
import sys

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


def drop_cfg_test(text):
    """Remove #[cfg(test)]-gated items: from the attribute to the end of the item's brace block or ';'."""
    out = []
    pos = 0
    for m in CFG_TEST.finditer(text):
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


def count(text):
    body = drop_cfg_test(strip_comments(text))
    return sum(1 for line in body.splitlines() if line.strip())


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


def totals(items, depth):
    """(per-group counts of our code, vendored total)."""
    ours, vendored = collections.Counter(), 0
    for name, text in items:
        if in_dirs(name, VENDORED_DIRS):
            vendored += count(text)
        else:
            ours[group(name, depth)] += count(text)
    return ours, vendored


def main(argv):
    mode = argv[1]
    if mode == 'git':
        items, rest = items_git(argv[2], argv[3]), argv[4:]
    elif mode == 'dir':
        items, rest = items_dir(argv[2]), argv[3:]
    else:
        items, rest = items_tree(argv[2]), argv[3:]
    ours, vendored = totals(items, int(rest[0]) if rest else 1)
    for g, n in sorted(ours.items(), key=lambda kv: -kv[1]):
        print(f'{g:40s} {n:8d}')
    print(f'{"TOTAL (implementation, no tests)":40s} {sum(ours.values()):8d}')
    print(f'{"third_party/ (vendored, not counted)":40s} {vendored:8d}')


if __name__ == '__main__':
    main(sys.argv)
