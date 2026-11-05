"""Write eden/phase23/rows-hb-ab/rows.json from the rc2d proof's segments/gates and this run's hooks (research helper)."""
import hashlib
import json
from pathlib import Path

ROWS = Path('/home/intpa/smash-training-lobby/tmp-p23/hitbox-orbs/eden/phase23/rows-hb-ab')
RC2D = json.loads(Path('/home/intpa/eden-runs/tr-p23-surface-rc2d/rows/rows.json').read_text())


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


segments = {k: RC2D['segments'][k] for k in ('main-stage', 'stage-css', 'attract-cancel', 'title-confirm')}
for f in sorted((ROWS / 'segments').glob('*.edenmacro.json')):
    name = f.name.removesuffix('.edenmacro.json')
    segments[name] = {'file': f'segments/{f.name}', 'sha256': sha(f), 'cap': 20}
gates = {k: v for k, v in RC2D['gates'].items() if k in (
    'loaded', 'main-fast', 'main', 'main-settled', 'title-fast', 'stage', 'css', 'installed', 'actual-ready',
    'actual-training-current', 'training-hud-unpaused', 'native-pause')}
for g in gates.values():
    ref = g.get('reference')
    if ref and not ref.startswith('/'):
        assert (ROWS / ref).is_file(), ref
        g['sha256'] = sha(ROWS / ref)


def row(rid, start, end, run, budget, extra=()):
    steps = [{'run': run, 'check': True}, *extra]
    return {'id': rid, 'start': start, 'end': end, 'budget_min': budget, 'steps': steps}


plan = {
    'game_id': RC2D['game_id'],
    'start': RC2D['start'],
    'scope': ('Hitbox own-orbs A/B run ab2 (HB1 route A4 prototype; primary 03:56Z and 11:38Z orders) on the diag NRO of '
              'p23/hitbox-orbs b771f08: P1 Ridley + CPU Steve by in-fight pushes; route B first-frame refusals against '
              'own orbs; both layers on one paused frame per pair with a repeat shot for frame identity (standing; fsmash '
              'in hitlag, accepted on held hit spheres; grab, accepted on a held grab sphere); camera-lag clips with cyan '
              'orbs; Off. Joint flag 0 from ab1 row 06.'),
    'segments': segments,
    'gates': gates,
    'setup': [
        {'state': 'Eden frontend up, game not loaded (defer-load); boot tracing then game.load', 'via': 'hook',
         'ref': 'hb_steps.py:preload (trace.configure unmapped log, symbols, counters on publish_orbs/draw_orbs)'},
        {'state': 'Boot settings: the rc2d fixture (hitbox_vis Off) pinned in config files', 'via': 'fixture',
         'ref': 'files: rc2-settings.json -> user/sdmc/ultimate/TrainingRooms/settings.json'},
        {'state': 'Training, HUD settled', 'via': 'segment', 'ref': 'main-stage + stage-css library segments, tile cursor pick'},
        {'state': 'P1 Ridley, CPU Steve, CPU Behavior Stop, synchronous swaps', 'via': 'fixture',
         'ref': 'hb_steps.py:push (diag-settings-next.json from hb-base-settings.json, Ally gbc-04 route)'},
        {'state': 'Route and A/B view', 'via': 'fixture', 'ref': 'diag-hitbox-route.req (hb_steps.py:route)'},
        {'state': 'Paused on a fixed game frame', 'via': 'segment', 'ref': 'hb-plus, hb-csmash-pK, hb-grab-pK (Plus at frame K)'},
    ],
    'rows': [
        row('00-load', 'boot', 'loaded', 'hb_steps.py:preload', 4),
        {'id': '01-main', 'start': 'loaded', 'end': 'main', 'budget_min': 3,
         'steps': [{'run': 'p4t_boot.py:boot_to_main', 'check': True}, {'gate': 'installed'}, {'shot': 'main'}]},
        {'id': '02-stage', 'start': 'main', 'end': 'stage', 'budget_min': 2,
         'steps': [{'play': 'main-stage'}, {'gate': 'stage'}, {'shot': 'stage'}]},
        row('03-training', 'stage', 'Training-settled', 'hb_steps.py:css_pick_and_enter', 3),
        row('04-fighters', 'Training-settled', 'Training-settled', 'hb_steps.py:fighters', 4),
        row('05-first-frame', 'Training-settled', 'Training-settled', 'hb_steps.py:first_frame', 3),
        row('07-ab-standing', 'Training-settled', 'Training-settled', 'hb_steps.py:ab_standing', 3),
        row('08-ab-attack', 'Training-settled', 'Training-settled', 'hb_steps.py:ab_attack', 8),
        row('09-ab-grab', 'Training-settled', 'Training-settled', 'hb_steps.py:ab_grab', 8),
        row('07b-probe', 'Training-settled', 'Training-settled', 'hb_steps.py:probe_pause', 2),
        row('07c-slow-standing', 'Training-settled', 'Training-settled', 'hb_steps.py:slow_standing', 3),
        row('08c-slow-attack', 'Training-settled', 'Training-settled', 'hb_steps.py:slow_attack', 3),
        row('09c-slow-grab', 'Training-settled', 'Training-settled', 'hb_steps.py:slow_grab', 3),
        row('10-lag', 'Training-settled', 'Training-settled', 'hb_steps.py:lag_clips', 3),
        row('11-off', 'Training-settled', 'Training-settled', 'hb_steps.py:off_and_scan', 2),
    ],
}
(ROWS / 'rows.json').write_text(json.dumps(plan, indent=2) + '\n')
print('rows', len(plan['rows']), 'segments', len(segments), 'gates', len(gates))
