"""Code-size gate: count the implementation (tools/sloc.py rules) and compare it with size-budget.toml.

size-budget.toml holds a list of [[budget]] entries, oldest first. Each has `scope` ("total" or a path prefix such
as "src/runtime/roster"), `ceiling`, `date` (YYYY-MM-DD), `reason` and, where needed, `approved_by`. The last entry
of a scope is its ceiling. The first entry of a scope, and every entry that raises its scope's ceiling, must name
the lead's approval in `approved_by`; lowering a ceiling (the ratchet) needs only a reason. Dates never go back.

Prints the per-module table (depth 3) and each budgeted scope. The budget gates the as-measured count (the
operator's rules); the test-fixed count and, with --normalize, the rustfmt-normalized + test-fixed count are printed
for the milestone reports until the operator rules on the method. Exit status: 0 within budget, 1 over a ceiling,
2 when the budget file breaks the rules above.
usage: python3 -I tools/size_gate.py [--repo DIR] [--budget FILE] [--rev REV] [--depth N] [--normalize]"""
import argparse
import collections
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))  # python3 -I leaves the script's folder out
import sloc  # noqa: E402

try:
    import tomllib
except ImportError:  # Python < 3.11
    tomllib = None

DATE = re.compile(r'^\d{4}-\d{2}-\d{2}$')


def check_budget(entries):
    """(ceilings by scope, list of rule violations)."""
    errors, ceilings, last_date = [], {}, {}
    for number, entry in enumerate(entries, 1):
        where = f'[[budget]] #{number}'
        scope, ceiling = entry.get('scope'), entry.get('ceiling')
        date, reason, approved = entry.get('date', ''), entry.get('reason', ''), entry.get('approved_by', '')
        if not isinstance(scope, str) or not scope or scope.endswith('/'):
            errors.append(f'{where}: scope must be "total" or a path prefix without a trailing slash')
            continue
        if not isinstance(ceiling, int) or isinstance(ceiling, bool) or ceiling <= 0:
            errors.append(f'{where} ({scope}): ceiling must be a positive integer')
            continue
        if not isinstance(date, str) or not DATE.match(date):
            errors.append(f'{where} ({scope}): date must be YYYY-MM-DD')
        elif date < last_date.get(scope, ''):
            errors.append(f'{where} ({scope}): date {date} is before the previous entry\'s {last_date[scope]}')
        if not isinstance(reason, str) or not reason.strip():
            errors.append(f'{where} ({scope}): reason is required')
        previous = ceilings.get(scope)
        if (previous is None or ceiling > previous) and not (isinstance(approved, str) and approved.strip()):
            what = 'the first ceiling' if previous is None else f'a raise from {previous} to {ceiling}'
            errors.append(f'{where} ({scope}): {what} needs approved_by (the lead\'s approval)')
        ceilings[scope] = ceiling
        if isinstance(date, str) and DATE.match(date):
            last_date[scope] = date
    if 'total' not in ceilings:
        errors.append('no "total" ceiling')
    return ceilings, errors


def scope_count(per_file, scope):
    if scope == 'total':
        return sum(per_file.values())
    return sum(n for name, n in per_file.items() if name == scope or name.startswith(scope + '/'))


def run(repo, budget_path, rev=None, depth=3, out=sys.stdout, normalize=False):
    if tomllib is None:
        print('size_gate: Python 3.11 or newer is required (tomllib)', file=out)
        return 2
    try:
        entries = tomllib.loads(pathlib.Path(budget_path).read_text('utf-8')).get('budget', [])
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f'size_gate: cannot read {budget_path}: {error}', file=out)
        return 2
    ceilings, errors = check_budget(entries)
    items = sloc.items_git(repo, rev) if rev else sloc.items_tree(repo)
    per_file, vendored = sloc.totals(items, 0)
    modules = collections.Counter()
    for name, n in per_file.items():
        modules[sloc.group(name, depth)] += n
    total = sum(per_file.values())
    print(f'Implementation lines (tools/sloc.py rules) at {rev or "the working tree"}, depth {depth}:', file=out)
    for name, n in sorted(modules.items(), key=lambda kv: -kv[1]):
        print(f'  {name:44s} {n:8d}  {100 * n / max(total, 1):5.1f}%', file=out)
    print(f'  {"TOTAL":44s} {total:8d}', file=out)
    print(f'  {"third_party/ (vendored, not counted)":44s} {vendored:8d}', file=out)
    fixed = sum(sloc.totals(items, 1, test_fixed=True)[0].values())
    print(f'  {"test-fixed (report only)":44s} {fixed:8d}', file=out)
    if normalize:
        norm = sum(sloc.totals(sloc.items_normalized(repo, rev or 'HEAD'), 1, test_fixed=True)[0].values())
        print(f'  {"rustfmt-normalized + test-fixed (report only)":44s} {norm:8d}  ({rev or "HEAD"})', file=out)
    over = []
    print('Budget (size-budget.toml):', file=out)
    for scope in sorted(ceilings, key=lambda s: (s != 'total', s)):
        n, ceiling = scope_count(per_file, scope), ceilings[scope]
        state = 'OVER' if n > ceiling else 'ok'
        print(f'  {scope:44s} {n:8d} / {ceiling:8d}  headroom {ceiling - n:+7d}  {state}', file=out)
        if n > ceiling:
            over.append(scope)
    for error in errors:
        print(f'size_gate: budget rule: {error}', file=out)
    if errors:
        return 2
    if over:
        print('size_gate: over budget in ' + ', '.join(over) + '. Cut code, or record a raise with the lead\'s '
              'approval as a new [[budget]] entry (docs/SIZE-PLAN.md).', file=out)
        return 1
    return 0


def main(argv=None):
    here = pathlib.Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--repo', default=str(here))
    parser.add_argument('--budget', default=None, help='default: <repo>/size-budget.toml')
    parser.add_argument('--rev', default=None, help='count a git revision instead of the working tree')
    parser.add_argument('--depth', type=int, default=3)
    parser.add_argument('--normalize', action='store_true',
                        help='also report the rustfmt-normalized, test-fixed count of --rev (default HEAD); needs rustfmt')
    args = parser.parse_args(argv)
    return run(args.repo, args.budget or str(pathlib.Path(args.repo) / 'size-budget.toml'), args.rev, args.depth,
               normalize=args.normalize)


if __name__ == '__main__':
    sys.exit(main())
