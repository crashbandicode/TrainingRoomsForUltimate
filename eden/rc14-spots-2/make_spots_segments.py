"""Sweet/sour spot rows (rc.14, lane `spots`): input segments, authored offline as Eden input macros (same format and masks as
make_btt_segments.py). Eden counts displayed frames, so a stick held for N frames walks the same distance whatever the host's speed.

`python3 -I make_spots_segments.py` writes segments/sp-*.edenmacro.json and segments-spots.json (the rows.json `segments` entries with
their sha256), deterministically.

* sp-csmash: the C-stick pushed right for 6 frames: a forward smash on the ground (the C-stick attacks by default).
* sp-wf<N>: the left stick at half tilt, toward the CPU, for N frames (a walk, under the dash threshold): spacing for the attack.
* sp-wb<N>: the same walk away from the CPU (a negative spacing in the rows).
* sp-jabsmash-w<W>: a jab, then (26 frames later) W frames of walk away and a forward smash: the follow-up of a Jab > Side smash link.
* sp-fair: a short hop (X tapped for 3 frames) and, 12 frames later, the C-stick right for 4 frames: a forward air.
"""
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from make_btt_segments import A, HERE, X, event, macro, push

# Walking toward the CPU (the reset puts them far apart): the spacings of the x1 sweeps, then four times those for the 1/4-speed GIFs.
WALKS = (4, 8, 12, 16, 17, 18, 19, 20, 21, 22, 23, 24, 26, 28, 36, 44, 52, 60, 68, 76, 32, 48, 64, 68, 72, 76, 84, 80, 88, 96, 104, 112, 144, 176, 208, 240, 272)


def build():
    out = {}
    out['sp-csmash'] = macro('sp-csmash', [event(0, 0, (0, 0, 1.0, 0)), event(6, 0)], 9)
    # spots-2: the same smash with the C-stick held h frames longer, which charges it (the probe pairs each with the frames the game held).
    for h in (30, 66):
        out['sp-csmash-h%d' % h] = macro('sp-csmash-h%d' % h, [event(0, 0, (0, 0, 1.0, 0)), event(6 + h, 0)], 9 + h)
    for frames in WALKS:
        out['sp-wf%d' % frames] = push('sp-wf%d' % frames, (0.5, 0, 0, 0), frames)
    for w in (0, 4, 8, 12):
        ev = [event(0, A), event(3, 0)] + ([event(26, 0, (-0.5, 0, 0, 0))] if w else []) + [event(26 + w, 0, (0, 0, 1.0, 0)), event(32 + w, 0)]
        out['sp-jabsmash-w%d' % w] = macro('sp-jabsmash-w%d' % w, ev, 36 + w)
    for frames in (4, 8, 12, 16, 20, 24, 32, 48, 64, 80, 96):
        out['sp-wb%d' % frames] = push('sp-wb%d' % frames, (-0.5, 0, 0, 0), frames)
    out['sp-fair'] = macro('sp-fair', [event(0, X), event(3, 0), event(15, 0, (0, 0, 1.0, 0)), event(19, 0)], 24)
    # spots-2: a fair with the C-stick `lead` frames after the short hop's X: at x1 speed the knee is low and early (the 1/4-speed GIF takes of the
    # first boot, whose macro frames are emulation frames, put the stick about 4 game frames after the X and landed the knee's sweet and sour hits).
    for lead in (4, 5, 6):
        out['sp-fair-e%d' % lead] = macro('sp-fair-e%d' % lead, [event(0, X), event(3, 0), event(lead, 0, (0, 0, 1.0, 0)), event(lead + 4, 0)], lead + 9)
    return out


def main():
    folder = HERE / 'segments'
    folder.mkdir(exist_ok=True)
    entries = {}
    for name, data in build().items():
        path = folder / ('%s.edenmacro.json' % name)
        text = json.dumps(data, indent=1) + '\n'
        path.write_text(text)
        entries[name] = {'file': 'segments/%s.edenmacro.json' % name, 'sha256': hashlib.sha256(text.encode()).hexdigest(), 'cap': 30}
    (HERE / 'segments-spots.json').write_text(json.dumps(entries, indent=1, sort_keys=True) + '\n')
    print(len(entries), 'segments')


if __name__ == '__main__':
    main()
