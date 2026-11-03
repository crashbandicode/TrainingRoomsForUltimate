"""Reconcile recorded events/counters and fresh-row entry contracts, offline only."""
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path


def analyze(records, counters, rows, points):
    events = Counter((r.get('point_id'), r.get('event')) for r in records)
    errors = Counter(r.get('event') for r in records
                     for c in r.get('captures', []) if c.get('error'))
    sequences = [int(r['sequence']) for r in records if 'sequence' in r]
    losses = [r for r in records if r.get('event') not in ('entry', 'return')]
    checks = {'contiguous_sequences': sequences == list(range(1, len(records) + 1)),
              'no_loss_records': not losses,
              'output_ok': counters.get('boot_error') == counters.get('output_error') == ''
                           and counters.get('file_lost') == '0'}
    totals = {}
    for p in counters['points']:
        pid = str(p['id']); entry = events[pid, 'entry']; returns = events[pid, 'return']
        checks['counters:' + pid] = (int(p['recorded']) == entry + returns
            and int(p['hits']) == int(p['matched']) == entry == returns
            and all(p.get(k) == '0' for k in ('dropped', 'abandoned_returns', 'return_overflow')))
        totals[pid] = {'target': p['target'], 'entries': entry, 'returns': returns}
    fresh = {}
    for row in rows:
        for suffix, target, expected in [
            ('warm-groups', 'main+0x653240', {'x0+0xe0': '1', 'x1+0x18': '74', 'x2': '1'}),
            ('warm-return', 'main+0x6524e0', {'x0+0xe0': '1', 'x0+0x18': '74'})]:
            if not row['id'].endswith(suffix):continue
            matches = []
            for r in records:
                if r.get('event') != 'entry' or r.get('point_id') != points[target]:continue
                if not row['started'] <= int(r['wall_time_ns']) / 1e9 <= row['ended']:continue
                captures = {c['expression']: c for c in r['captures']}
                if all(captures.get(k, {}).get('value') == v
                       and captures[k].get('error') == '' for k,v in expected.items()):
                    matches.append(r['sequence'])
            fresh[row['id']] = matches
            checks['fresh:' + row['id']] = bool(matches)
    # Pair only per point/process/thread, in stack order. No return argument identity is inferred.
    stacks = defaultdict(list); pairs = 0; unmatched_returns = 0
    for r in records:
        key = (r.get('point_id'), r.get('process_id'), r.get('thread_id'))
        if r.get('event') == 'entry':stacks[key].append(r)
        elif r.get('event') == 'return':
            if stacks[key]:stacks[key].pop();pairs += 1
            else:unmatched_returns += 1
    checks['balanced_entry_return_stacks'] = not unmatched_returns and not any(stacks.values())
    return {'checks': checks, 'record_count': len(records), 'points': totals,
            'capture_errors_by_event': dict(errors), 'fresh_cpu_entry_sequences': fresh,
            'stack_pairs': pairs, 'stream_reconciliation_pass': all(checks.values()),
            'scope': 'Entry captures only. Return-time captures are not entry arguments. Not clean-console or whole-RC acceptance.'}


def main():
    root = Path(__file__).resolve().parent
    rows = [json.loads(p.read_text()) for p in sorted((root/'receipts').glob('*.json'))]
    first = next(r for r in rows if r['id'] == '01-main')
    points = next(r['value']['point_ids'] for r in first['results'] if r.get('run') == 'steps.py:trace_wiring')
    last = next(r for r in rows if r['id'] == 'final-continuation')
    counters = next(r['value'] for r in last['results'] if r.get('run') == 'steps.py:finish_trace')
    raw = (root/'yoga.trace.jsonl').read_bytes()
    result = analyze([json.loads(line) for line in raw.splitlines()], counters, rows, points)
    result['trace_sha256'] = hashlib.sha256(raw).hexdigest()
    (root/'trace-analysis.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
    return 0 if result['stream_reconciliation_pass'] else 1


if __name__ == '__main__':raise SystemExit(main())
