"""Summarize matched F6 receipts without treating absent thresholded gaps as zero."""
import argparse
import hashlib
import json
from pathlib import Path
import re

from f6_split_table import parse, parse_stalls


def cache_evidence(path):
    inventories = {}
    for phase in ('before', 'after'):
        file = path / f'cache-{phase}.json'
        if file.exists():
            record = json.loads(file.read_text())
            inventories[phase] = dict(sha256=hashlib.sha256(file.read_bytes()).hexdigest(), utc=record['utc'],
                                     valid=all(name in record.get('roots', {}) for name in ('user/shader','user/cache','xdg-cache')))
        else:
            inventories[phase] = None
    return dict(inventories=inventories, complete=all(row and row['valid'] for row in inventories.values()) and not (path/'cache-error.txt').exists(),
                limit='Preserved caches, fixed Off-before-On order. Missing inventories cannot be reconstructed; '
                      'no cold-cache, cache-identity or final performance acceptance claim.')


def window(lines, start_ms, end_ms, owner=0):
    """Keep endpoint-selected samples; crossing/rounded boundaries are explicit."""
    samples = []
    for line in lines:
        rows = parse_stalls([line])
        if not rows or rows[0]['owner'] != owner:
            continue
        stamp = int(line.split()[0])
        if start_ms <= stamp <= end_ms:
            row = rows[0]
            row.update(endpoint_ms=stamp, raw=line,
                       fully_inside=stamp * 1000 - row['gap_us'] >= (start_ms + 1) * 1000
                       and stamp < end_ms)
            samples.append(row)
    return dict(start_ms=start_ms, end_ms=end_ms, samples=samples,
                endpoint_sample_max_us=max((r['gap_us'] for r in samples), default=None),
                contained_sample_max_us=max((r['gap_us'] for r in samples if r['fully_inside']), default=None),
                limit='Only gaps >60000us are logged. None is censored/unknown, never zero. '
                      'Endpoint selection alone does not prove a fully contained gap.')


def summarize(proof, lines):
    transactions = proof['transactions']
    assert len(transactions) == 17
    picks = {}
    for press, label in ((3, 'first_kirby'), (11, 'repeat_kirby')):
        matches = [r for r in transactions if r['phase'] == 'lap' and r['role'] == 0 and r['press'] == press]
        assert len(matches) == 1 and matches[0]['target'] == 6
        row = matches[0]
        start, end = int(row['submit'].split()[0]), int(row['ready'].split()[0])
        assert end >= start
        assert end - start == row['request_to_ready_ms']
        picks[label] = dict(request_to_ready_ms=end-start, submit=row['submit'], ready=row['ready'],
                            gaps=window(lines, start, end))
    begin = int(transactions[0]['submit'].split()[0])
    end = int(picks['first_kirby']['ready'].split()[0])
    cold_lines = [line for line in lines if line.split() and line.split()[0].isdigit()
                  and begin <= int(line.split()[0]) <= end]
    builds = [b for b in parse(cold_lines) if b['owner'] == 0 and b['kinds'] == [6, 11, 31]]
    return dict(label=proof['label'], identities=proof['identities'], native_baseline=proof['native_baseline'],
                transactions=transactions, **picks, cold_bootstrap_to_first_kirby=window(lines, begin, end),
                cold_kirby_builds=builds,
                limits=['Setup/control spans exclude queued worker execution.',
                        'Cold three-member prefetch and native single-fighter load are different work.',
                        'No exact subthreshold maximum or zero-hitch conclusion from this report.'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('off', type=Path)
    parser.add_argument('on', type=Path)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    summaries = []
    hashes = {}
    for path, native in ((args.off, True), (args.on, False)):
        proof_path = path / 'f6-matched-proof.json'
        proof = json.loads(proof_path.read_text())
        assert proof['native_baseline'] == native
        lines = []
        for name in ('practice-diagnostics.previous.log', 'practice-diagnostics.log'):
            log = path / 'TrainingRooms' / name
            if log.exists():
                lines.extend(log.read_text().splitlines())
                hashes[str(log)] = hashlib.sha256(log.read_bytes()).hexdigest()
        hashes[str(proof_path)] = hashlib.sha256(proof_path.read_bytes()).hexdigest()
        summaries.append(summarize(proof, lines))
    off, on = summaries
    assert off['identities']['source'] == on['identities']['source']
    assert off['identities']['nro'] == on['identities']['nro']
    result = dict(off=off, on=on, input_sha256=hashes,
                  cache_evidence=dict(off=cache_evidence(args.off), on=cache_evidence(args.on)),
                  qualification='Development comparison only; cache evidence and fixed-order limitations apply.',
                  request_latency_delta_on_minus_off_ms={
                      key: on[key]['request_to_ready_ms']-off[key]['request_to_ready_ms']
                      for key in ('first_kirby', 'repeat_kirby')})
    args.out.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result['request_latency_delta_on_minus_off_ms']))


if __name__ == '__main__':
    main()
