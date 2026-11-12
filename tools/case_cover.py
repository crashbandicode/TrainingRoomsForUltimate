#!/usr/bin/env python3
"""rc.9 coverage, option (b) (docs/COVERAGE-rc9.md): the product lines each test case covers on its own.

    CARGO_TARGET_DIR=<fresh dir> cargo llvm-cov --no-fail-fast --no-report     # builds the instrumented test binaries
    python3 -I tools/case_cover.py measure <fresh dir> <out dir> [--jobs N]
    python3 -I tools/case_cover.py count <fresh dir>                            # cases, table rows, plain tests

A case is one row of a `case_table` (run alone with `TR_TABLE_CASE`), or one plain `#[test]` (run alone with `--exact`).
Each runs in its own process with its own profile; `llvm-cov export` of that profile against the case's own binary gives
its lines. Writes `<out dir>/cases.tsv` (binary, test, case, source file of the table) and `<out dir>/lines/<n>.txt` (one
`path:line` per covered product line, paths as tools/coverage.py normalises them).
"""
import concurrent.futures
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def tools_dir():
    sysroot = subprocess.run(['rustc', '--print', 'sysroot'], capture_output=True, text=True, check=True).stdout.strip()
    host = subprocess.run(['rustc', '-vV'], capture_output=True, text=True, check=True).stdout.split('host: ')[1].split('\n')[0]
    return os.path.join(sysroot, 'lib', 'rustlib', host, 'bin')


def binaries(target):
    deps = os.path.join(target, 'llvm-cov-target', 'debug', 'deps')
    return sorted(os.path.join(deps, f) for f in os.listdir(deps)
                  if '.' not in f and os.path.isfile(os.path.join(deps, f)) and os.access(os.path.join(deps, f), os.X_OK))


def list_cases(binary, repo):
    env = dict(os.environ, TR_TABLE_LIST='1', LLVM_PROFILE_FILE='/dev/null')
    out = subprocess.run([binary, '--list'], capture_output=True, text=True, cwd=repo, env=env).stdout
    tests = [l[:-len(': test')] for l in out.split('\n') if l.endswith(': test')]
    out = subprocess.run([binary, '--nocapture', '--test-threads=1', 'case_table'], capture_output=True, text=True, cwd=repo, env=env).stdout
    rows = {}
    for l in out.split('\n'):
        if 'TABLE-CASE\t' in l:  # the harness prints `test <name> ... ` before a table's first row
            _, file, table, case = l[l.index('TABLE-CASE\t'):].split('\t')
            rows.setdefault(table, []).append((case, file))
    cases = []
    for t in tests:
        if t in rows:
            cases += [(binary, t, case, file) for case, file in rows[t]]
        else:
            cases.append((binary, t, '', ''))
    return cases


def measure_one(args):
    n, (binary, test, case, _), repo, out, tools = args
    # one profile per process: a case may run this binary again (current_exe), and the child's lines are the case's
    rawdir = os.path.join(out, 'raw', str(n))
    os.makedirs(rawdir, exist_ok=True)
    env = dict(os.environ, LLVM_PROFILE_FILE=os.path.join(rawdir, '%p-%m.profraw'))
    if case:
        env['TR_TABLE_CASE'] = case
    run = subprocess.run([binary, '--exact', test, '--test-threads=1', '--nocapture'],
                         capture_output=True, text=True, cwd=repo, env=env)
    ok = run.returncode == 0
    lines = set()
    raws = [os.path.join(rawdir, f) for f in os.listdir(rawdir)]
    if raws:
        data = os.path.join(rawdir, 'case.profdata')
        subprocess.run([os.path.join(tools, 'llvm-profdata'), 'merge', '-sparse'] + raws + ['-o', data], check=True)
        lcov = subprocess.run([os.path.join(tools, 'llvm-cov'), 'export', '-format=lcov', '-instr-profile=' + data,
                               r'-ignore-filename-regex=(/\.cargo/|/rustc/|/toolchain/tools/|/registry/)', binary],
                              capture_output=True, text=True).stdout
        cur = None
        for l in lcov.split('\n'):
            if l.startswith('SF:'):
                p = os.path.normpath(l[3:].strip())
                cur = os.path.relpath(p, repo) if p.startswith(repo + '/') else None
                if cur and not cur.startswith('src/'):
                    cur = None
            elif cur and l.startswith('DA:'):
                ln, hits = l[3:].split(',')[:2]
                if int(hits):
                    lines.add(f'{cur}:{ln}')
        for f in raws + [data]:
            os.remove(f)
    os.rmdir(rawdir)
    with open(os.path.join(out, 'lines', f'{n}.txt'), 'w') as f:
        f.write('\n'.join(sorted(lines)) + ('\n' if lines else ''))
    return n, ok, len(lines)


def main():
    if len(sys.argv) == 3 and sys.argv[1] == 'count':
        cases = [c for b in binaries(os.path.abspath(sys.argv[2])) for c in list_cases(b, os.path.abspath('.'))]
        rows = sum(1 for c in cases if c[2])
        print(f'cases={len(cases)} table rows={rows} in {len({(c[0], c[1]) for c in cases if c[2]})} tables, plain tests={len(cases) - rows}')
        return
    if len(sys.argv) < 4 or sys.argv[1] != 'measure':
        sys.exit(__doc__)
    target, out = os.path.abspath(sys.argv[2]), os.path.abspath(sys.argv[3])
    jobs = int(sys.argv[sys.argv.index('--jobs') + 1]) if '--jobs' in sys.argv else 4
    repo = os.path.abspath('.')
    os.makedirs(os.path.join(out, 'raw'), exist_ok=True)
    os.makedirs(os.path.join(out, 'lines'), exist_ok=True)
    tools = tools_dir()
    cases = []
    for b in binaries(target):
        cases += list_cases(b, repo)
    with open(os.path.join(out, 'cases.tsv'), 'w') as f:
        for n, (b, t, c, file) in enumerate(cases):
            f.write(f'{n}\t{re.sub(r"-[0-9a-f]{16}$", "", os.path.basename(b))}\t{t}\t{c}\t{file}\n')
    print(f'{len(cases)} cases in {len(set(c[0] for c in cases))} binaries', flush=True)
    failed = 0
    with concurrent.futures.ThreadPoolExecutor(jobs) as pool:
        for n, ok, count in pool.map(measure_one, [(n, c, repo, out, tools) for n, c in enumerate(cases)]):
            if not ok:
                failed += 1
                print('FAILED alone:', cases[n][1], cases[n][2], flush=True)
            if n % 250 == 0:
                print(f'{n}/{len(cases)}', flush=True)
    print('done; failed alone:', failed)


main()
