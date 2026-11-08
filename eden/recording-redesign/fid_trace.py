"""Frame-by-frame comparison of a played macro with what reached the controller (rc.5 cpu-fidelity lane).

Input is the persistent trace JSONL of one play window, from points named in the run config:
- `apply`: the instruction that writes a played frame, with the packed frame in a register
  (buttons | lstick_x << 32 | lstick_y << 40 | rstick_x << 48 | rstick_y << 56);
- `tick`: one record per consuming frame (the CPU's AI frame, or Player 1's poll);
- `motion`: `MotionModule::change_motion` (x0 accessor, x1 the motion's Hash40).

`compare` lines the applied frames up with the file's frames and reports dropped frames (in the
file, never applied), consumer frames with no frame inside the run (the AI's or the pad's own word
stood), and frames applied more than once. Usage:

    python3 -I fid_trace.py <trace.jsonl> <macro.json> <apply id> <tick id> [<motion id>]
"""
import json
import sys
import zlib

MOTIONS = [
    'wait', 'wait_2', 'wait_3', 'wait_4', 'walk_slow', 'walk_middle', 'walk_fast', 'walk_brake', 'dash', 'run',
    'run_brake', 'turn', 'turn_dash', 'turn_run', 'squat', 'squat_wait', 'squat_rv', 'jump_squat', 'jump_f',
    'jump_b', 'jump_aerial_f', 'jump_aerial_b', 'fall', 'fall_f', 'fall_b', 'fall_aerial', 'landing_light',
    'landing_heavy', 'attack_11', 'attack_12', 'attack_13', 'attack_100', 'attack_100_end', 'attack_dash',
    'attack_s3_s', 'attack_s3_hi', 'attack_s3_lw', 'attack_hi3', 'attack_lw3', 'attack_s4_s', 'attack_s4_hi',
    'attack_s4_lw', 'attack_s4_hold', 'attack_hi4', 'attack_lw4', 'attack_air_n', 'attack_air_f', 'attack_air_b',
    'landing_air_n', 'special_n', 'special_air_n', 'special_s', 'special_hi', 'special_lw', 'guard_on', 'guard',
    'guard_off', 'guard_damage', 'appeal_lw_l', 'appeal_lw_r', 'appeal_hi_l', 'appeal_hi_r', 'appeal_s_l',
    'appeal_s_r', 'catch', 'catch_dash', 'catch_turn', 'escape', 'escape_f', 'escape_b', 'damage_n_1',
    'damage_n_2', 'damage_n_3', 'entry_l', 'entry_r', 'rebirth', 'stand_wait',
]


def hash40(name):
    data = name.encode()
    return (len(data) << 32) | zlib.crc32(data)


NAMES = {hash40(name): name for name in MOTIONS}


def motion_name(value):
    return NAMES.get(value & 0xff_ffff_ffff, hex(value))


def unpack(word):
    def signed(shift):
        byte = (word >> shift) & 0xff
        return byte - 256 if byte & 0x80 else byte
    return (word & 0xffff_ffff, signed(32), signed(40), signed(48), signed(56))


def file_frames(macro):
    return [(f['buttons'], f['lstick_x'], f['lstick_y'], f['rstick_x'], f['rstick_y']) for f in macro['frames']]


def register(record, name):
    value = (record.get('registers') or {}).get(name)
    if value is None:
        return None
    return int(value, 16) if isinstance(value, str) else int(value)


def records(lines):
    out = []
    for line in lines:
        line = line.strip()
        if not line.startswith('{'):
            continue
        try:
            item = json.loads(line)
        except ValueError:
            continue
        if item.get('event') in ('entry', 'return'):
            out.append(item)
    return out


def packed(record, reg):
    """The packed frame from one register, or from two ("x8+x9": the buttons word, then the four stick bytes)."""
    if '+' in reg:
        low, high = (register(record, name) for name in reg.split('+', 1))
        if low is None or high is None:
            return None
        return (low & 0xffff_ffff) | ((high & 0xffff_ffff) << 32)
    return register(record, reg)


def applied(recs, apply_id, reg):
    """[(sequence, vsync frame, unpacked frame)] in trace order."""
    out = []
    for r in recs:
        if r.get('point_id') == apply_id:
            word = packed(r, reg)
            if word is not None:
                out.append((int(r['sequence']), int(r['frame']), unpack(word)))
    return out


def ticks(recs, tick_id):
    return [(int(r['sequence']), int(r['frame'])) for r in recs if r.get('point_id') == tick_id]


def compare(expected, applied_frames, tick_seqs):
    """`expected`: the file's frames. `applied_frames`: [(seq, vsync, frame)] of one play. `tick_seqs`: [(seq,
    vsync)] of every consumer frame in the window.

    Returns counts plus the first mismatches. A frame is matched to the next equal file frame at or after the
    current position, so a drop is a skipped file index and a repeat is an applied frame equal to the one before
    that matches no later file frame first."""
    pos = 0
    matched = []
    unmatched = []
    for seq, vsync, frame in applied_frames:
        found = None
        for index in range(pos, len(expected)):
            if expected[index] == frame:
                found = index
                break
        if found is None:
            unmatched.append({'seq': seq, 'vsync': vsync, 'frame': frame})
            continue
        matched.append((found, seq, vsync))
        pos = found + 1
    indices = [index for index, _, _ in matched]
    dropped = [i for i in range(len(expected)) if i not in set(indices)]
    if matched:
        first, last = matched[0][1], matched[-1][1]
        applied_seqs = {seq for seq, _, _ in applied_frames}
        inside = [(seq, vsync) for seq, vsync in tick_seqs if first <= seq <= last]
        # A tick that comes after an apply and before the next one, with no apply of its own in between.
        gaps = 0
        ordered = sorted([(seq, 'a') for seq in applied_seqs if first <= seq <= last] +
                         [(seq, 't') for seq, _ in inside])
        pending_apply = False
        for _, kind in ordered:
            if kind == 'a':
                pending_apply = True
            elif pending_apply:
                pending_apply = False
            else:
                gaps += 1
        ticks_inside = len(inside)
    else:
        gaps = 0
        ticks_inside = 0
    return {
        'file_frames': len(expected),
        'applied': len(applied_frames),
        'matched': len(matched),
        'dropped': len(dropped),
        'dropped_first': dropped[:12],
        'unmatched': len(unmatched),
        'unmatched_first': unmatched[:6],
        'ticks_inside_run': ticks_inside,
        'ticks_without_frame': gaps,
        'in_order': indices == sorted(indices),
    }


def motions(recs, motion_id):
    """[(vsync, accessor, name)]."""
    out = []
    for r in recs:
        if r.get('point_id') != motion_id:
            continue
        accessor, value = register(r, 'x0'), register(r, 'x1')
        if accessor is None or value is None:
            continue
        out.append((int(r['frame']), hex(accessor), motion_name(value)))
    return out


def main(argv):
    path, macro_path, apply_id, tick_id = argv[1:5]
    reg = 'x9'
    if ':' in apply_id:
        apply_id, reg = apply_id.split(':', 1)
    recs = records(open(path, encoding='utf-8', errors='replace'))
    expected = file_frames(json.load(open(macro_path)))
    report = compare(expected, applied(recs, apply_id, reg), ticks(recs, tick_id))
    if len(argv) > 5:
        report['motions'] = motions(recs, argv[5])
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main(sys.argv)
