"""spots-3: the one-boot plan. Takes the copied `spots` plan (rows/rows.json, integration-3's), keeps its boot rows (GDB stub, load, main menu,
route to Training) and replaces the spot rows with spots-3's: `kr-gutcheck` (the gust labelled Sour), `sp-rage`, `sp-charge`, `sp-equal`,
`sp-late`, `sp-off`, every check through `spots3_rows.py` (the knockback oracle, the old damage oracle beside it). Writes the `sp-downb` segment
(down + B in one frame) with its sha256. `python3 -I build_spots3_plan.py` rewrites rows/rows.json in place, deterministically.
"""
import hashlib
import json
import sys
from pathlib import Path

ROWS = Path(__file__).resolve().parent / 'rows'
sys.path.insert(0, str(ROWS))
from make_btt_segments import B, event, macro  # noqa: E402

BOOT = ('g-gdb-on', 'q00-load', 'g-gdb-attached', 'm-main', 'p-route')
KROOL, MARIO, MARTH = 66, 0, 21


def row(rid, budget, steps, expect):
    return {'id': rid, 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused', 'group': 'rc14-spots-3',
            'on_failure': 'continue', 'budget_min': budget, 'steps': steps, 'expect': expect}


def run(fn, **args):
    step = {'run': 'spots3_rows.py:%s' % fn, 'on': 'diag', 'check': True}
    if args:
        step['args'] = args
    return step


def main():
    plan = json.loads((ROWS / 'rows.json').read_text())
    seg = macro('sp-downb', [event(0, B, (0, -1.0, 0, 0)), event(4, 0)], 8)
    text = json.dumps(seg, indent=1) + '\n'
    (ROWS / 'segments' / 'sp-downb.edenmacro.json').write_text(text)
    plan['segments']['sp-downb'] = {'file': 'segments/sp-downb.edenmacro.json', 'sha256': hashlib.sha256(text.encode()).hexdigest(), 'cap': 30}
    boot = [r for r in plan['rows'] if r['id'] in BOOT]
    assert [r['id'] for r in boot] == list(BOOT), [r['id'] for r in boot]
    # Boot 2 (the record-slot attribution, approved ~03:20Z): sp-rage first (its start gate missed after kr-gutcheck in boot 1).
    plan['rows'] = boot + [
        row('sp-rage', 5, [run('sp_rage')],
            "spots-3: the CPU Mario jabs P1 Marth past 35% (Mash Jab, Always, 30 s), then stops; Marth's forward smash lands with rage. The "
            "received/slot knockback ratio of the raged hit is logged for the window; every label agrees with the record-slot oracle."),
        row('sp-stale-tip', 4, [run('sp_stale_tip')],
            "spots-3, boot 1's defect: the CPU Marth mashes Jab (Always) on P1 Mario; a stale tip hit (the game's 5-damage slot, damage ratio "
            "under 1.0) is labelled Sweet on P1's lane; every label agrees with the record-slot oracle."),
        row('kr-gutcheck', 10, [run('kr_gutcheck')],
            "spots-3: P1 King K. Rool's Gut Check while the CPU Marth mashes Forward Tilt, then Jab (Always): the counter's weaker hitbox (the gust, "
            "the game's Sour slot) is labelled Sour on the CPU's lane. [SpotsWind] keeps each recorded slot's no-reaction byte (is the gust a "
            "windbox?) and a 1/4-speed clip with Windboxes only shows the counter."),
        row('sp-charge', 8, [run('sp_setup', p1=MARTH, cpu=MARIO, frame_data=False, labels=True),
                             run('sp_charge', holds=[0, 30, 66], walks=[8, 28], rounds=1)],
            "spots-2's charged forward smash (Marth vs Mario) under the record-slot oracle: the tip Sweet and the blades Sour at every charge."),
        row('sp-late', 6, [run('sp_late')],
            "Captain Falcon's forward air at the spacings where its late hits land: a Late label; every label agrees with the oracle."),
    ]
    plan['recover'] = [{'run': 'spots3_rows.py:sp_recover', 'check': True}]
    plan['scope'] = ('rc.14 spots-3 (spots by knockback): the diag build of rc14/spots-3, the btt rig card set and boot chain, Training only, '
                     'muted (volume 0), the GDB stub on before game.load. Settings through the diag reload file. No card; nothing pushed.')
    plan['setup'] = [s for s in plan['setup'] if 'charge' not in s.get('state', '')][:2] + [
        {'state': 'P1 King K. Rool vs a CPU that mashes an attack (Mash and Mash Triggers set by the reload file)', 'via': 'hook',
         'ref': 'spots3_rows.py:kr_gutcheck (diag-settings-next.json)'},
        {'state': 'the counter input and the charged smashes', 'via': 'segment',
         'ref': 'segments/sp-downb.edenmacro.json (build_spots3_plan.py), sp-csmash-h30, sp-csmash-h66 (sha256 pinned)'}]
    (ROWS / 'rows.json').write_text(json.dumps(plan, indent=1) + '\n')
    print('rows', [r['id'] for r in plan['rows']], 'budget', sum(r.get('budget_min', 0) for r in plan['rows']))


if __name__ == '__main__':
    main()
