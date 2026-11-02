"""Rustfmt-normalized implementation code lines in Rust: no blank lines, no comments, no tests (the primary's counter, v4).

Provenance: the primary's tool /home/intpa/smash-training-lobby/tools/sloc4.py (2026-11-02, sha256
6dcb208c203b4adcbd8c06c67347726e62c0582bd254723e3a4b6a1dd030fc19), copied 2026-11-02 by the slim stream with its
counting rules unchanged, so totals stay comparable with the fork's count. It replaces the earlier copy of sloc3.py.
Normalized measurement provenance: operator tools/sloc-fmt.ps1 SHA256 ace31db0d654530e3e4faf719dadaef557003510bb4edabed59a5fa42fe77898.
CLI and size_gate format each file via pinned stable rustfmt stdin (edition2021, skip_children=true), then apply
unchanged measure() rules. Scripts/tools are excluded from normalized implementation totals. No source is modified.
Changes: vendored code under third_party/ is excluded and reported on its own line; a `tree` mode counts the working
tree's tracked and untracked-not-ignored files (what tools/size_gate.py gates); depth 0 lists single files; the
counting is a function (`measure`) that size_gate.py imports.

Rules (sloc4): a file reached only through a test-gated `mod name;` (`#[cfg(test)]` or `#[cfg(all(test, ...))]`) is a
test file; items gated by those attributes are dropped; items and files gated by `#[cfg(any(test, feature =
"simulator"))]` are host-only simulator code, reported apart and not counted. Excluded as before: files under tests/,
benches/, examples/; // and /* */ comments; blank lines.
usage: python3 -I sloc.py git <repo> <rev> [depth]  |  python3 -I sloc.py dir <path> [depth]  |  python3 -I sloc.py tree <repo> [depth]"""
import collections
import pathlib
import posixpath
import re
import subprocess
import sys

TEST_DIRS = ('tests/', 'benches/', 'examples/')
VENDORED_DIRS = ('third_party/',)
TEST_ONLY_CFG = re.compile(r'#\[cfg\(\s*(test|all\(\s*test\b[^\]]*)\s*\)\]')
SIM_CFG = re.compile(r'#\[cfg\(\s*any\(\s*test\s*,\s*feature\s*=\s*"simulator"\s*\)\s*\)\]')
MOD_DECL = re.compile(r'((?:#\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;')
PATH_ATTR = re.compile(r'#\[path\s*=\s*"([^"]+)"\]')


def strip_comments(text):
    out, i, n, depth, in_str = [], 0, len(text), 0, False
    while i < n:
        c = text[i]
        if depth:
            if text.startswith('/*', i):
                depth += 1; i += 2
            elif text.startswith('*/', i):
                depth -= 1; i += 2
            else:
                if c == '\n':
                    out.append('\n')
                i += 1
            continue
        if in_str:
            out.append(c)
            if c == '\\' and i + 1 < n:
                out.append(text[i + 1]); i += 2; continue
            if c == '"':
                in_str = False
            i += 1
            continue
        if text.startswith('//', i):
            j = text.find('\n', i)
            i = n if j < 0 else j
            continue
        if text.startswith('/*', i):
            depth = 1; i += 2; continue
        if c == '"' and not (i > 0 and text[i - 1] == "'" and i + 1 < n and text[i + 1] == "'"):
            in_str = True
        out.append(c)
        i += 1
    return ''.join(out)


def drop_gated(text, pattern):
    """Remove items gated by `pattern`: from the attribute to the end of the brace block or ';'. Returns (kept, dropped)."""
    out, dropped, pos = [], [], 0
    for m in pattern.finditer(text):
        if m.start() < pos:
            continue
        out.append(text[pos:m.start()])
        i, n = m.end(), len(text)
        while i < n and text[i] not in '{;':
            i += 1
        if i < n and text[i] == ';':
            dropped.append(text[m.start():i + 1]); pos = i + 1; continue
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
        dropped.append(text[m.start():i]); pos = i
    out.append(text[pos:])
    return ''.join(out), ''.join(dropped)


def nonblank(text):
    return sum(1 for line in text.splitlines() if line.strip())


def mod_children(path, text):
    """Yield (child_path, test_only) for `mod x;` declarations in a file."""
    d = posixpath.dirname(path)
    base = posixpath.basename(path)
    own_dir = d if base in ('mod.rs', 'lib.rs', 'main.rs') else posixpath.join(d, base[:-3])
    for m in MOD_DECL.finditer(text):
        attrs, name = m.group(1), m.group(2)
        test_only = 'test' if TEST_ONLY_CFG.search(attrs) else ('sim' if SIM_CFG.search(attrs) else None)
        p = PATH_ATTR.search(attrs)
        if p:
            yield posixpath.normpath(posixpath.join(d, p.group(1))), test_only
        else:
            yield posixpath.join(own_dir, name + '.rs'), test_only
            yield posixpath.join(own_dir, name, 'mod.rs'), test_only


def group(path, depth):
    if depth == 0:
        return path
    parts = path.split('/')
    return '/'.join(parts[:depth]) if len(parts) > depth else '/'.join(parts[:-1]) or '(root)'


def in_dirs(name, dirs):
    return any(f'/{d}' in f'/{name}' for d in dirs)


def files_git(repo, rev):
    names = subprocess.run(['git', '-C', str(repo), 'ls-tree', '-r', '--name-only', rev],
                           capture_output=True, text=True, check=True).stdout.split('\n')
    return {name: subprocess.run(['git', '-C', str(repo), 'show', f'{rev}:{name}'], capture_output=True,
                                 check=True).stdout.decode('utf-8', 'replace')
            for name in names if name.endswith('.rs') and not in_dirs(name, TEST_DIRS)}


def files_dir(root):
    root = pathlib.Path(root)
    files = {}
    for p in root.rglob('*.rs'):
        rel = p.relative_to(root).as_posix()
        if rel.startswith('target/') or '/target/' in rel or in_dirs(rel, TEST_DIRS):
            continue
        files[rel] = p.read_text('utf-8', 'replace')
    return files


def files_tree(repo):
    """The working tree as git sees it (tracked plus untracked-not-ignored); a plain directory walk without git."""
    repo = pathlib.Path(repo)
    try:
        names = subprocess.run(['git', '-C', str(repo), 'ls-files', '--cached', '--others', '--exclude-standard'],
                               capture_output=True, text=True, check=True).stdout.split('\n')
    except (OSError, subprocess.CalledProcessError):
        return files_dir(repo)
    return {n: (repo / n).read_text('utf-8', 'replace') for n in names
            if n.endswith('.rs') and not in_dirs(n, TEST_DIRS) and (repo / n).is_file()}


def measure(files, depth):
    """(per-group implementation counts, host-only simulator total, test lines excluded, vendored total)."""
    stripped = {k: strip_comments(v) for k, v in files.items()}
    # A file is test-only if every `mod` declaration that reaches it is test-gated (and at least one reaches it).
    reached = collections.defaultdict(list)
    for path, text in stripped.items():
        for child, test_only in mod_children(path, text):
            if child in stripped:
                reached[child].append(test_only)
    test_files = {p for p, flags in reached.items() if flags and all(f == 'test' for f in flags)}
    sim_files = {p for p, flags in reached.items() if flags and all(f in ('test', 'sim') for f in flags) and p not in test_files}
    changed = True
    while changed:  # children of a test file are test-only
        changed = False
        for path in list(test_files):
            for child, _ in mod_children(path, stripped[path]):
                if child in stripped and child not in test_files:
                    test_files.add(child); changed = True
    impl, sim, tests, vendored = collections.Counter(), 0, 0, 0
    for path, text in stripped.items():
        if path in test_files:
            tests += nonblank(text)
            continue
        if path in sim_files:
            sim += nonblank(text)
            continue
        kept, dropped_sim = drop_gated(text, SIM_CFG)
        kept, dropped_test = drop_gated(kept, TEST_ONLY_CFG)
        sim += nonblank(dropped_sim)
        tests += nonblank(dropped_test)
        if in_dirs(path, VENDORED_DIRS):
            vendored += nonblank(kept)
        else:
            impl[group(path, depth)] += nonblank(kept)
    return impl, sim, tests, vendored


# Operator's sloc-fmt.ps1, 2026-11-02: edition 2021 and stable formatter.
# Format isolated stdin so measurement never edits source or follows module paths.
# The operator's final comparison excludes scripts (tools/ and scripts/).
NORMALIZED_BASIS = "rustfmt-sloc4"
RUSTFMT_VERSION = "rustfmt 1.9.0-stable (48a229ceae 2026-09-01)"


def normalized_files(files):
    rustup = pathlib.Path.home() / ".cargo/bin/rustup"
    command = [str(rustup), "run", "1.98.1", "rustfmt"]
    version = subprocess.run(command + ["--version"], capture_output=True, text=True,
                             check=True, timeout=30).stdout.strip()
    if version != RUSTFMT_VERSION:
        raise ValueError(f"normalization requires {RUSTFMT_VERSION}; found {version}")
    result = {}
    for name, text in files.items():
        if in_dirs(name, VENDORED_DIRS) or name.startswith(("tools/", "scripts/")):
            continue
        formatted = subprocess.run(
            command + ["--edition", "2021", "--config", "skip_children=true", "--emit", "stdout"],
            input=text, capture_output=True, text=True, timeout=30)
        if formatted.returncode:
            raise ValueError(f"rustfmt failed for {name}: {formatted.stderr.strip()}")
        result[name] = formatted.stdout
    return result


def measure_normalized(files, depth):
    return measure(normalized_files(files), depth)


def main(argv):
    mode = argv[1]
    if mode == 'git':
        files, rest = files_git(argv[2], argv[3]), argv[4:]
    elif mode == 'dir':
        files, rest = files_dir(argv[2]), argv[3:]
    else:
        files, rest = files_tree(argv[2]), argv[3:]
    impl, sim, tests, vendored = measure_normalized(files, int(rest[0]) if rest else 1)
    print(f'Basis: {NORMALIZED_BASIS}; {RUSTFMT_VERSION}; edition 2021; scripts excluded')
    for g, n in sorted(impl.items(), key=lambda kv: -kv[1]):
        print(f'{g:40s} {n:8d}')
    print(f'{"TOTAL (implementation, no tests)":40s} {sum(impl.values()):8d}')
    print(f'{"host-only simulator (not counted)":40s} {sim:8d}')
    print(f'{"test code excluded (files + gated items)":40s} {tests:8d}')
    print(f'{"third_party/ (vendored, not counted)":40s} {vendored:8d}')


if __name__ == '__main__':
    main(sys.argv)
