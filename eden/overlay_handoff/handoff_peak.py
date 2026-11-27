"""Per-frame counts of the overlay's command hooks from an Eden persistent trace (trace.* JSONL) of the release NRO.

A frame is the span between two consecutive entries of the frame point (`overlay_process_end_hook`, phase 8): only
complete spans count, so what precedes the first frame entry and follows the last is left out. For every other point the
script reports how many entries fell into each span: the total, the highest count, the 99.9th and 50th percentiles
(nearest rank) and the threads that ran it. `six` is the sum of the catch, rebound and four wind sites; `hand_off` is
every command that needs a pool node.

Loss is never hidden: a line that is not an entry or return record, and every sequence number missing between the first
and the last record, are reported, and the exit code is 2 unless --allow-loss.

usage: python3 -I handoff_peak.py <trace.jsonl> --frame <point_id> --point <name>=<point_id> ... [--allow-loss] [--out file]
"""
import argparse
import json
import sys

SIX = ('catch', 'rebound', 'wind', 'wind_arg10', 'wind_rad', 'wind_rad_arg9')


def read(path):
    """The entry records in sequence order, the lines that are no record, and the missing sequence numbers."""
    entries, other, seen = [], [], []
    with open(path, encoding='utf-8') as lines:
        for number, line in enumerate(lines, 1):
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
                sequence = int(row['sequence'])
                event, point = row['event'], str(row['point_id'])
            except (ValueError, KeyError, TypeError):
                other.append((number, line[:160]))
                continue
            if event not in ('entry', 'return'):
                other.append((number, line[:160]))
                continue
            seen.append(sequence)
            if event == 'entry':
                entries.append((sequence, point, str(row.get('thread_id', '?'))))
    entries.sort()
    seen.sort()
    duplicates = len(seen) - len(set(seen))
    missing = (seen[-1] - seen[0] + 1 - len(set(seen))) if seen else 0
    return entries, other, missing, duplicates


def rank(sorted_counts, numerator, denominator):
    """Nearest-rank percentile `numerator/denominator` of an ascending list, in whole numbers."""
    count = len(sorted_counts)
    return sorted_counts[max(0, (count * numerator + denominator - 1) // denominator - 1)] if count else 0


def measure(entries, frame, points):
    """Counts per complete frame span for each named point (and `six` when its sites are all named)."""
    names = {point: name for name, point in points.items()}
    spans, threads, current, open_span, first_frame = [], {name: set() for name in points}, None, None, None
    frame_threads = set()
    for sequence, point, thread in entries:
        if point == frame:
            frame_threads.add(thread)
            if current is not None:
                spans.append((open_span, current))
            current, open_span = dict.fromkeys(points, 0), sequence
            first_frame = sequence if first_frame is None else first_frame
        elif point in names and current is not None:
            current[names[point]] += 1
            threads[names[point]].add(thread)
    report = {'frames': len(spans), 'frame_threads': sorted(frame_threads), 'points': {}}
    series = {name: [counts[name] for _, counts in spans] for name in points}
    if all(name in points for name in SIX):
        series['six'] = [sum(counts[name] for name in SIX) for _, counts in spans]
    for name, counts in series.items():
        ordered = sorted(counts)
        top = max(range(len(counts)), key=counts.__getitem__) if counts else None
        report['points'][name] = {
            'total': sum(counts),
            'max': ordered[-1] if ordered else 0,
            'p99_9': rank(ordered, 999, 1000),
            'p50': rank(ordered, 1, 2),
            'max_at_sequence': spans[top][0] if top is not None else None,
            'threads': sorted(threads.get(name, ())),
        }
    return report


def main(argv):
    parser = argparse.ArgumentParser()
    parser.add_argument('trace')
    parser.add_argument('--frame', required=True)
    parser.add_argument('--point', action='append', default=[])
    parser.add_argument('--allow-loss', action='store_true')
    parser.add_argument('--out')
    args = parser.parse_args(argv)
    points = dict(item.split('=', 1) for item in args.point)
    entries, other, missing, duplicates = read(args.trace)
    report = measure(entries, args.frame, points)
    report.update({'trace': args.trace, 'entries': len(entries), 'other_lines': len(other), 'other_samples': other[:5],
                   'missing_sequences': missing, 'duplicate_sequences': duplicates})
    text = json.dumps(report, indent=1, sort_keys=True)
    if args.out:
        with open(args.out, 'w', encoding='utf-8') as out:
            out.write(text + '\n')
    print(text)
    return 0 if args.allow_loss or not (other or missing or duplicates) else 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
