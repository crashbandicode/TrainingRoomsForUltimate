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
ZL, LEFT, UP, RIGHT, DOWN = 256, 4096, 8192, 16384, 32768


def event(frame, buttons=0, lx=0.0, ly=0.0):
    zero = {'accel': [0, 0, 0], 'delta_us': 10000, 'gyro': [0, 0, 0]}
    return {'buttons': buttons, 'frame': frame, 'motion': [zero, zero], 'sticks': [lx, ly, 0, 0], 'triggers': [0, 0]}


def cstick(ev, rx):
    ev['sticks'][2] = rx
    return ev


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
    # Forward smash on the right stick (the smash stick: no charge, no dash), then Plus `k` frames after it.
    for k in range(8, 46, 2):
        c = 2
        write(f'hb-csmash-p{k}', [event(0), cstick(event(c), 1.0), event(c + 4), event(c + k, PLUS), event(c + k + 6)],
              c + k + 10)
    # Standing grab (shield + attack in one frame), then Plus `k` frames after it.
    for k in range(3, 19):
        g = 2
        write(f'hb-grab-p{k}', [event(0), event(g, ZR | A), event(g + 4), event(g + k, PLUS), event(g + k + 6)], g + k + 10)
    # The same inputs without a pause (the slowed-speed method): c-stick fsmash, standing grab.
    write('hb-csmash', [event(0), cstick(event(2), 1.0), event(6)], 12)
    write('hb-grab', [event(0), event(2, ZR | A), event(6)], 12)
    # Ridley's side special (Space Pirate Rush): its CATCH box is out while he runs (ZR + A jumped and aired in ab2:
    # this profile's shoulder buttons are not shield).
    write('hb-sideb', [event(0), event(2, B, 1.0), event(8, 0, 1.0), event(9)], 14)
    # Training's frame-by-frame speed ("1 Frame (Press ZL)"): menu taps, a one-frame ZL tap, ZL held (normal speed),
    # and the attack / side special read on the frame a ZL tap advances.
    for name, bits in (('down', DOWN), ('up', UP), ('right', RIGHT), ('left', LEFT), ('a', A)):
        write(f'hb-{name}', [event(0), event(2, bits), event(6)], 12)
    write('hb-zl', [event(0), event(2, ZL), event(3)], 8)
    write('hb-zl-hold', [event(0), event(2, ZL), event(92)], 96)
    write('hb-zl-csmash', [event(0), cstick(event(2, ZL), 1.0), cstick(event(3), 1.0), event(5)], 10)
    write('hb-zl-sideb', [event(0), event(2, ZL | B, 1.0), event(3, B, 1.0), event(5)], 10)
    # ZL is also this profile's shield (ab3: each stepped ZL put Ridley in guard, statuses 27/28/29), so shield + A on the
    # stepped frame is a standing grab.
    write('hb-zl-a', [event(0), event(2, ZL | A), event(3, A), event(5)], 10)
    # Shield grab at normal speed (ZL held runs the frozen game and shields), A while shielding, ZL let go two frames
    # later: the game freezes again early in the grab, before its box comes out.
    write('hb-zl-shieldgrab', [event(0), event(2, ZL), event(30, ZL | A), event(32, ZL), event(33)], 40)
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
