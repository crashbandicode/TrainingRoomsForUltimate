"""Writes rows.json for the Break the Targets boot (btt-3, 2026-11-14): B-2's boot chain and shared segments and gates
(rows.base.json, the rc14-egg-target-2 / d2-btt-cam-1 rig that booted, loaded and reached Training with P1 Ridley), then the
btt rows. Run `python3 -I make_btt_segments.py && python3 -I make_btt_fixture.py && python3 -I make_btt_rows.py`.

Order: cheap, state-driven rows first; each chains on the Training end state; the Change Stage rows and the GIF takes
last (they are the slow ones); the rows that may wedge the game (none here) would go after them. A failed row does not
stop the boot (`on_failure: continue` with `btt_recover`), except the boot chain and Change Stage.
"""
import json
import os
from pathlib import Path

HERE = Path(__file__).resolve().parent
HUD = 'training-hud-unpaused'
GIF = {'docs': 'docs'}


def hook(name, **args):
    step = {'run': 'btt_rows.py:' + name, 'on': 'diag', 'check': True}
    if args:
        step['args'] = args
    return step


def row(row_id, steps, expect, budget, group='rc14-btt', on_failure='continue'):
    return {'id': row_id, 'start': 'training', 'end': 'training', 'start_gate': HUD, 'group': group, 'on_failure': on_failure,
            'budget_min': budget, 'steps': steps, 'expect': expect}


def gif(name, nested):
    return {'gif': {'name': name, 'preset': 'docs', 'steps': [dict(n) for n in nested]}, 'on': 'diag'}


def main():
    base = json.loads((HERE / 'rows.base.json').read_text())
    by_id = {r['id']: r for r in base['rows']}
    segments = dict(base['segments'])
    segments.update(json.loads((HERE / 'segments-btt.json').read_text()))
    # Recipe-only segments of the base that were never recorded (no file): none of these rows plays them, so they stay out of the plan.
    segments = {k: v for k, v in segments.items() if (HERE / v['file']).is_file() or str(v['file']).startswith('/')}
    boot = [dict(by_id[i], budget_min=by_id[i].get('budget_min', 3)) for i in ('g-gdb-on', 'q00-load', 'g-gdb-attached', 'm-main', 'p-route')]
    rows = boot + [
        row('btt-st-bf', [hook('change_stage', key='bf')],
            "Change Stage to Battlefield with the mode Off; the game's own [PracticeStageRequest] says the stage id is the table's 0.", 5, on_failure='stop'),
        row('btt-E26m', [hook('e26_menu')],
            'The Place targets row (Fighters & Stages > Practice games) closes the menu into the placing view and turns Break the Targets On: a [Btt] phase=Placing line, menu.btt = [0, 1].', 3),
        row('btt-E26', [hook('e26')],
            'Placing (acceptance row 26): three targets at the cursor, an overlapping A refused, B on release removes the last (3 to 2), B + D-pad Up opens the menu and removes nothing, closing it opens Ready with 2 targets.', 4),
        row('btt-E28', [hook('e28')],
            'Ready (row 28): X with no targets refused; X with three goes to Ready at 0:00.00; no clock for 6 s; the first input starts it; L+R+A returns to Ready with all three.', 4),
        row('btt-E29', [hook('e29')],
            'Run (row 29): P1 breaks the three targets; the clock stops on the last break (Done! 3 in ... (best ...) for 3 s); Ready again with every target back; a second run shows the fastest as its best.', 6),
        row('btt-E41', [hook('e41', key='bf')],
            'Save (row 41, btt-5): ZL + Down twice on Battlefield makes two NEW files in layouts/ (Layout 1 and 2 · Battlefield), each with the 3 targets; no other file changes; the newer is chosen 1/2; the Saved layouts list shows this stage\'s files newest first.', 4),
        row('btt-E43', [hook('e43', key='bf')],
            'The Saved layouts screen (rows 43, btt-5): A on the newest opens Load, Rename, Duplicate, Delete; Rename (the keyboard, "Ledge trap") keeps the same file and puts it on top; Duplicate makes Layout N · Battlefield with the same targets at the top; Delete asks once (Keep it, Delete it) and then removes only that file.', 5),        row('btt-E31a', [hook('e31', key='bf')],
            'Camera (row 31) on Battlefield: stills of the whole stage in Placing, Ready and Running; the game\'s own camera with the menu open and with the mode Off; the [Btt] camera lines agree.', 5),
        row('btt-G1', [hook('g1_prep'), gif('btt-place-three.gif', [hook('g1_action')])],
            'GIF: the menu on Place targets, A, the stage zooms out, three targets placed, notice 3/10 (Battlefield, 1280 wide, under 10 s).', 3),
        row('btt-G2', [hook('g2_prep'), gif('btt-run-three.gif', [hook('g2_action')])],
            'GIF: Ready with three targets, the run, the clock stopping, the Done notice (1280 wide, under 10 s).', 3),
        row('btt-G3', [hook('g3_prep'), gif('btt-save-load.gif', [hook('g3_action')])],
            'GIF: ZL + Down saves a new layout file (Saved: Layout N · Battlefield), Y twice clears, ZL + Up loads it back (1280 wide, under 10 s).', 3),        row('btt-st-bbf', [hook('change_stage', key='bbf')],
            'Change Stage to Big Battlefield (a wide stage) with the mode Off; the stage id is the table\'s 2, which is the seeded layouts file\'s name.', 5),
        row('btt-E42', [hook('e42', key='bbf')],
            'Load (row 42, btt-5): the seeded layouts/seeded-big-battlefield.json (not written by this boot) is read when the stage opens: ZL + Right names it 1/1 with 3 targets, ZL + Up loads the 3 into Placing (a use: its used number rises), X opens Ready.', 3),
        row('btt-E31b', [hook('e31', key='bbf')],
            'Camera (row 31) on a wide stage: stills in Placing, Ready and Running on Big Battlefield, the game\'s camera with the menu open and with the mode Off.', 4),
        row('btt-X-stages', [hook('x_stages', first='bf', second='bbf')],
            'Two stages (btt-5): on Big Battlefield the list and the chords show only its own layouts; a Save there makes Layout 1 · Big Battlefield and no Battlefield file changes; back on Battlefield ZL + Right steps to the older layout (2/2) and ZL + Up loads it, which lists it first; no Big Battlefield file changes.', 7),
        row('btt-X-saves', [hook('x_saves', key='bf')],
            'The chords on the hot five (btt-5): saves until Battlefield has 7 files (each a new one); ZL + Right steps 2/5 .. 5/5, 1/5, 2/5 through the five newest, ZL + Left back to 1/5; the two oldest stay files; ZL + Up on 3/5 loads it and it becomes 1/5; ZL + Y twice deletes it and the five refill (6 files, 5 hot); no temporary file is left.', 5),        row('btt-off', [hook('btt_off')], 'The mode Off, the menu closed, the fighters reset: the boot ends as the others do.', 1),
    ]
    # BTT_ROWS (comma-separated row ids; btt-5's boot): only those rows after the boot chain, in this order.
    only = [r for r in os.environ.get('BTT_ROWS', '').split(',') if r]
    if only:
        rows = rows[:len(boot)] + [r for r in rows[len(boot):] if r['id'] in only]
    plan = {
        'game_id': base['game_id'], 'start': 'frontend-ready', 'early_stop': False, 'on_failure': 'continue',
        'scope': ('rc.14 Break the Targets (SPEC V2.5, acceptance rows 26-31 and 41-43) on the primary lane\'s DIAG build: B-2\'s '
                  'card set and boot chain (rc14-egg-target-2, d2-btt-cam-1), P1 Ridley vs the CPU on Battlefield and Big '
                  'Battlefield. Muted, the GDB stub on before game.load, one boot, the layouts file for Big Battlefield seeded. '
                  'No card; nothing pushed.'),
        'setup': [
            {'state': 'Training, P1 Ridley vs the CPU, after the GDB-stub boot chain', 'via': 'hook',
             'ref': 'gdb_rc11.py:gdb_on, gdb_kick; mem.py:m_main; pillrows.py:p_route (the d2-btt-cam-1 chain that passed)'},
            {'state': 'Battlefield or Big Battlefield in Training, the mode Off', 'via': 'hook',
             'ref': 'btt_rows.py:change_stage (Change Stage grid, tiles 1 and 3 right of Waiting Room), stage id checked against [PracticeStageRequest]'},
            {'state': 'Break the Targets On and Placing', 'via': 'hook',
             'ref': 'btt_rows.py:to_placing (the Place targets row; the live setting plus a menu close if the row path fails)'},
            {'state': 'the placing cursor, the chords and the attacks', 'via': 'segment',
             'ref': 'segments/btt-*.edenmacro.json (make_btt_segments.py, frame-exact stick moves and same-frame chords, sha256 pinned); chord-save, chord-load, menu-open, x-press, jab, ftilt, sh-nair, fsmash, reset-center from the base'},
            {'state': 'a saved layout file the boot did not write: Big Battlefield, three targets (btt-5)', 'via': 'fixture',
             'ref': 'config files: fixtures/btt/seeded-big-battlefield.json -> user/sdmc/ultimate/TrainingRooms/layouts/seeded-big-battlefield.json (stage_btt.ps1)'},
        ],
        'segments': segments, 'gates': base['gates'],
        'recover': [{'run': 'btt_rows.py:btt_recover', 'check': True}],
        'gif_presets': {}, 'rows': rows,
    }
    (HERE / 'rows.json').write_text(json.dumps(plan, indent=1) + '\n')
    budget = sum(r.get('budget_min', 0) for r in rows)
    print(len(rows), 'rows,', budget, 'budget minutes')


if __name__ == '__main__':
    main()
