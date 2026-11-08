"""The CPU-fidelity fixture and segment (rc.5 cpu-fidelity lane, 2026-11-08).

`fixtures/fidelity/macros/fidelity.json` is one macro with a jab, a forward tilt, a jump and a held
direction, active on both roles, with no start save. Every neutral frame carries a small left-stick Y
(|y| <= 20, inside the 24-unit neutral limit) that changes each frame, and the held direction ramps
its X by one unit a frame, so a dropped or doubled frame shows in the trace by value, not only by count.

`segments/multi-input.edenmacro.json` is the same four inputs on Player 1's pad, for the rows that
record through the chord.
"""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
IDLE = 0x8000
JAB = 0x8201
JUMP = 0x8004


def frames():
    out = []

    def neutral(count):
        for _ in range(count):
            out.append((IDLE, 0, (len(out) % 41) - 20))

    neutral(10)
    out += [(JAB, 0, 0)] * 2
    neutral(30)
    out += [(IDLE, 60, 0)] * 3 + [(JAB, 60, 0)] * 2 + [(IDLE, 60, 0)]
    neutral(40)
    out += [(JUMP, 0, 0)] * 6
    neutral(70)
    out += [(IDLE, 30 + step, 0) for step in range(40)]
    neutral(30)
    return out


def recording():
    return {
        'schema_version': 1,
        'name': 'Fidelity',
        'game_version': '13.0.5',
        'start': {'fighter_kind': 0, 'costume': 0, 'facing': 1.0, 'situation': 0, 'status': 0,
                  'speed_x': 0.0, 'speed_y': 0.0},
        'placement': {'delta_x': 0.0, 'delta_y': 0.0, 'momentum': 'preserve_recorded'},
        'frames': [{'buttons': b, 'lstick_x': x, 'lstick_y': y, 'rstick_x': 0, 'rstick_y': 0}
                   for b, x, y in frames()],
    }


def index():
    return {
        'schema_version': 1,
        'migrated': True,
        'last_used': {'macro': {}, 'save': {}, 'sequence': {}, 'combo': {}},
        'hot': {'macro': ['fidelity'], 'save': [], 'sequence': [], 'combo': []},
        'loop': {'player': 1, 'cpu': 1},
        'active': {'player': 'fidelity', 'cpu': 'fidelity'},
        'chord_notice_seen': True,
    }


MOTION = [{'accel': [0, 0, -1], 'delta_us': 1000, 'gyro': [0, 0, 0]}] * 2


def event(frame, buttons=0, x=0.0, y=0.0):
    return {'buttons': buttons, 'frame': frame, 'motion': MOTION, 'sticks': [x, y, 0, 0], 'triggers': [0, 0]}


def segment():
    """A for 2 frames; forward tilt (stick 0.5, A on its 4th and 5th frame); X for 6 frames; stick 0.4 for 40."""
    events = [
        event(0, 1), event(2),
        event(32, 0, 0.5), event(35, 1, 0.5), event(37, 0, 0.5), event(38),
        event(78, 4), event(84),
        event(154, 0, 0.4), event(194),
    ]
    return {'bindings': {'gamepad': '', 'keyboard': ''}, 'duration_frames': 214, 'events': events,
            'format': 'eden-input-macro', 'game_id': '01006A800016E000', 'markers': [], 'name': 'multi-input',
            'parent': {'marker': '', 'name': ''}, 'player': 0, 'version': 1}


def main():
    root = HERE / 'fixtures' / 'fidelity'
    (root / 'macros').mkdir(parents=True, exist_ok=True)
    (root / 'macros' / 'fidelity.json').write_text(json.dumps(recording(), indent=2) + '\n')
    (root / 'recording-index.json').write_text(json.dumps(index(), indent=2) + '\n')
    (HERE / 'segments' / 'multi-input.edenmacro.json').write_text(json.dumps(segment(), separators=(',', ':')) + '\n')


if __name__ == '__main__':
    main()
