"""Build fid-rows.json from accept-diag-rows.json (boot route, gates, recover) and one build's trace points.

    python3 -I make_fid_rows.py <points.json> <out rows.json> [--strict]

`points.json` names the targets in that build's ELF (`fid-points-<build>.json`); the rows arm them.
"""
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main(argv):
    points = json.loads(Path(argv[1]).read_text())
    strict = '--strict' in argv
    base = json.loads((HERE / 'accept-diag-rows.json').read_text())
    segments = base['segments']
    segments['multi-input'] = {'file': 'segments/multi-input.edenmacro.json',
                               'sha256': sha(HERE / 'segments' / 'multi-input.edenmacro.json'), 'cap': 30}
    suffix = '_strict' if strict else ''
    rows = [next(row for row in base['rows'] if row['id'] == 'acc-boot')]
    for row_id, hook, expect in (
        ('fid-fixture', 'fid_fixture', 'The fidelity fixture (jab, forward tilt, jump, held direction) played on P1, '
                                       'then on the CPU: every file frame reaches the controller once, one per frame, '
                                       'in order, and the CPU jabs (attack_11) with no smash motion.'),
        ('fid-record', 'fid_record', 'The multi-input segment recorded on P1 and played on P1, then recorded on the CPU '
                                     'role and played on the CPU: the same frame-for-frame check against the recorded '
                                     'files.'),
        ('fid-chord3', 'fid_chord3', 'The docs-GIF fixture of the reported smash (chord3: three one-frame 0x8201 '
                                     'presses, loop 5, a start save) played on the CPU: the CPU jabs (attack_11), '
                                     'no smash motion.'),
    ):
        rows.append({'id': row_id, 'start': 'training', 'end': 'training',
                     'steps': [{'run': 'fid.py:%s%s' % (hook, suffix), 'check': True, 'on': 'diag'}],
                     'expect': expect, 'timed': False, 'build': 'diag', 'on': 'diag',
                     'start_gate': 'training-hud-unpaused'})
    plan = {
        'game_id': base['game_id'],
        'start': 'boot',
        'early_stop': False,
        'scope': 'rc.5 CPU playback fidelity, one diag instance (%s).' % points.get('build', '?'),
        'setup': [{'state': 'Training, both fighters Mario, menu closed, volume 0. fid-fixture copies the fidelity '
                            'fixture; fid-record records through the chords.',
                   'via': 'fixture', 'ref': 'fixtures/fidelity (make_fidelity.py), segments/multi-input'}],
        'segments': segments,
        'gates': base['gates'],
        'recover': base['recover'],
        'trace_points': points['points'],
        'rows': rows,
    }
    Path(argv[2]).write_text(json.dumps(plan, indent=2) + '\n')


if __name__ == '__main__':
    main(sys.argv)
