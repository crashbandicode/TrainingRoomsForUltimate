#!/usr/bin/python3 -I
"""Write the hitbox A/B run's input segments (Eden input macros, P1) into segments/ and print their sha256.

Button bits (from the library chords): A 1, B 2, Y 8, L 64, R 128, ZR 512, Plus 1024. A macro counts displayed
emulation frames, so a Plus at frame N pauses the game on the same game frame every play: that is what puts both
layers of the A/B on one frozen frame.
"""
import hashlib
import json
from pathlib import Path

OUT = Path(__file__).resolve().parent / 'segments'
A, B, Y, L, R, ZR, PLUS = 1, 2, 8, 64, 128, 512, 1024


def event(frame, buttons=0, lx=0.0, ly=0.0):
    zero = {'accel': [0, 0, 0], 'delta_us': 10000, 'gyro': [0, 0, 0]}
    return {'buttons': buttons, 'frame': frame, 'motion': [zero, zero], 'sticks': [lx, ly, 0, 0], 'triggers': [0, 0]}


def macro(name, events, duration):
    return {'bindings': {'keyboard': '', 'gamepad': ''}, 'format': 'eden-input-macro', 'game_id': '01006A800016E000',
            'parent': {'name': '', 'marker': ''}, 'player': 0, 'version': 1, 'name': name, 'events': events,
            'markers': [], 'duration_frames': duration}


def write(name, events, duration):
    path = OUT / f'{name}.edenmacro.json'
    path.write_text(json.dumps(macro(name, events, duration), indent=1) + '\n')
    print(name, hashlib.sha256(path.read_bytes()).hexdigest())


def main():
    OUT.mkdir(exist_ok=True)
    # Reset positions (L+R+A, one frame).
    write('hb-lra', [event(0), event(5, L | R | A), event(19)], 40)
    # Forward smash (stick right, A tapped the next frame), then Plus `k` frames after A.
    for k in (14, 18, 22, 26, 30):
        a = 3
        write(f'hb-fsmash-p{k}', [event(0), event(2, 0, 1.0), event(a, A, 1.0), event(a + 2, 0, 1.0), event(a + 3),
                                  event(a + k, PLUS), event(a + k + 6)], a + k + 10)
    # Standing grab (shield + attack in one frame), then Plus `k` frames after it.
    for k in (6, 7, 8, 9, 10, 12):
        g = 2
        write(f'hb-grab-p{k}', [event(0), event(g, ZR | A), event(g + 4), event(g + k, PLUS), event(g + k + 6)], g + k + 10)
    # Pause, camera mode, back.
    write('hb-plus', [event(0), event(2, PLUS), event(8)], 14)
    write('hb-y', [event(0), event(2, Y), event(8)], 14)
    write('hb-b', [event(0), event(2, B), event(8)], 14)
    # P1 walks right then left (the camera follows), for the lag clip.
    write('hb-walk', [event(0), event(2, 0, 1.0), event(92, 0, -1.0), event(182)], 190)
    # Camera mode: pan the paused camera right, then up.
    write('hb-cam-pan', [event(0), event(2, 0, 1.0), event(62, 0, 0.0, 1.0), event(122)], 130)


if __name__ == '__main__':
    main()
