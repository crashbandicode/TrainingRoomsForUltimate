"""Read-only lead-engine predicates. Retained images are SHAPE refs, not acceptance.

No controller construction, input, native memory request, trace or lifecycle call.
The old w01 binaries differ; exact current frame matching is required, otherwise
the predicate returns None and the existing engine's finite cap stops the route.
"""
from pathlib import Path
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parent


def _config():
    return json.loads((ROOT / 'pilot-markers.json').read_text())


def _mask(image, box):
    return bytes(int(p >= 180) for p in image.convert('L').crop(box).getdata())


def _distance(a, b):
    return sum(x != y for x, y in zip(a, b)) / len(a)


def classify_image(path, kind):
    """Return Player/CPU SHAPE evidence only; blank/opposite/unknown refuses."""
    from PIL import Image
    cfg = _config()
    if kind not in ('page', 'hud'):
        raise ValueError(kind)
    with Image.open(path) as current:
        if list(current.size) != cfg['size']:
            return None
        scores = {}
        for state in ('Player', 'CPU'):
            best = 1.0
            for reference in cfg[kind]['references'][state]:
                refpath = ROOT / 'refs' / Path(reference['path']).name
                if hashlib.sha256(refpath.read_bytes()).hexdigest() != reference['sha256']:
                    raise RuntimeError('Changed retained pilot shape reference')
                with Image.open(refpath) as ref:
                    if current.size != ref.size:
                        return None
                    if kind == 'page':
                        if any(_distance(_mask(current, box), _mask(ref, box)) > cfg['page']['anchor_max_mismatch']
                               for box in cfg['page']['anchors']):
                            continue
                    best = min(best, _distance(_mask(current, cfg[kind]['value_box']),
                                               _mask(ref, cfg[kind]['value_box'])))
            scores[state] = best
        state = min(scores, key=scores.get)
        other = 'CPU' if state == 'Player' else 'Player'
        if (scores[state] > cfg[kind]['max_mismatch']
                or scores[other] - scores[state] < cfg[kind]['min_margin']):
            return None
        return {'shape': state, 'kind': kind, 'scores': scores, 'path': str(path),
                'limits': 'old-binary text-shape reference; not actual armed ownership, AI or release acceptance'}


def pilot_page(ctx, before, args):
    expected = args['state']
    if expected not in ('Player', 'CPU'):
        raise ValueError('pilot state must be explicit')
    result = classify_image(ctx.t.shot('pilot-page-' + expected.lower()), 'page')
    return result if result and result['shape'] == expected else None


def pilot_hud(ctx, before, args):
    expected = args['state']
    if expected not in ('Player', 'CPU'):
        raise ValueError('pilot state must be explicit')
    path = ctx.t.shot('pilot-hud-' + expected.lower())
    result = classify_image(path, 'hud')
    # These known Training-grid references have a bright top-right area when
    # Deluxe is closed; the pilot panel is dark. Do not accept a native tag
    # still visible UNDER an open pilot page. Other stages/layouts refuse.
    from PIL import Image
    with Image.open(path) as image:
        pixels = list(image.convert('L').crop((994, 0, 1227, 60)).getdata())
    bright_fraction = sum(p > 140 for p in pixels) / len(pixels)
    if not result or result['shape'] != expected or bright_fraction < .70:
        return None
    result['closed_grid_panel_bright_fraction'] = bright_fraction
    return result


def pilot_prerequisites(ctx, before, args):
    """Actual current-boot install observations through EXISTING log transport.

    Lead maps t.lines('pilot') to CURRENT profile's ssbu-cpu-pilot/events.log.
    No reference-run logs are read by this predicate. Binding boot freshness of
    the transport is the lead lifecycle's obligation, as for events/diag.
    """
    events = ctx.t.lines('events')
    pilot = ctx.t.lines('pilot')
    diag = ctx.read_diag()
    own = next((l for l in events if 'plugin=training_rooms/' in l and
                'p1_cpu=not_installed reason=standalone_pilot_present' in l), None)
    common = next((l for l in events if 'cpu_common_load ' in l and 'cpu_common=installed' in l), None)
    installed = next((l for l in pilot if re.search(r'\bplugin=ssbu-cpu-pilot/0\.2\.1\b', l)
                      and 'result=installed' in l and 'menu=registered' in l), None)
    registration = next((l for l in diag if 'deluxe_row registered ' in l
                         and (m := re.search(r'\bhost_status=(0x[0-9a-fA-F]+|\d+)', l))
                         and int(m[1], 0) != 0), None)
    if not all((own, common, installed, registration)):
        return None
    return {'candidate_stand_down': own, 'common_installed': common,
            'pilot_registered': installed, 'deluxe_registration': registration,
            'limits': 'current-boot installation/registration only, NOT actual armed state or AI/release'}


def bindings():
    """Merge into the lead manifest; predicates load before any Up/input."""
    def pred(function, args=None, cap=10):
        return {'kind': 'predicate', 'module': 'slim_pilot_gates', 'function': function,
                'args': args or {}, 'cap_s': cap}
    return {
        'pilot-prerequisites-qualified': pred('pilot_prerequisites', cap=15),
        'pilot-open-player-row-qualified': pred('pilot_page', {'state': 'Player'}),
        'pilot-set-cpu-qualified': pred('pilot_page', {'state': 'CPU'}, 5),
        'pilot-reopen-cpu-row-qualified': pred('pilot_page', {'state': 'CPU'}),
        'pilot-set-player-qualified': pred('pilot_page', {'state': 'Player'}, 5),
        'pilot-close-to-training-armed-qualified': pred('pilot_hud', {'state': 'CPU'}),
        'pilot-close-to-training-released-qualified': pred('pilot_hud', {'state': 'Player'}),
    }


def apply_bindings(manifest):
    """Lead invokes AFTER build_manifest, BEFORE dry_run/Up; in-memory only.

    Missing recordings stay null and are allowed in the authorized first-record
    phase. Three unknown behavior/media gates remain explicit strict refusals.
    No lead file or live process is changed by this function.
    """
    def pin(path):
        return {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
    manifest['modules']['slim_pilot_gates'] = pin(ROOT / 'pilot_gates.py')
    manifest['modules']['slim'] = pin(ROOT / 'actions.py')
    manifest['files'] += [pin(ROOT / 'pilot-markers.json')]
    cfg = _config()
    for kind in ('page', 'hud'):
        for refs in cfg[kind]['references'].values():
            for ref in refs:
                if ref not in manifest['files']:
                    manifest['files'].append(ref)
    gates = manifest['gates']
    gates.update(bindings())
    def pred(function, args=None, module='slim', cap=15):
        return {'kind': 'predicate', 'module': module, 'function': function,
                'args': args or {}, 'cap_s': cap}
    for state, values in (('on', [1, 0]), ('off', [0, 1])):
        name = 'slim-settings-' + state
        gates[name] = pred('settings_values', {'values': {'hitbox_vis': values, 'frame_advantage': values}}, 'lead')
        gates['r3-settings-' + state + '-qualified'] = pred('gate_all', {'gates': ['closed-training', name]}, 'lead', 30)
    gates['r3-attack-observe-qualified'] = pred('framepass_gate')
    gates['r3-catch-observe-qualified'] = pred('catch_gate', cap=5)
    gates['slim-pause-begin'] = {'kind':'regex','source':'diag','fresh':True,'cap_s':10,'pattern':'pause_owner begin '}
    gates['slim-pause-end'] = {'kind':'regex','source':'diag','fresh':True,'cap_s':10,'pattern':r'pause_owner end .*owners=0x00\b'}
    gates['slim-resume-checked'] = {'kind':'regex','source':'diag','fresh':True,'cap_s':10,'pattern':r'pause_owner resume_checked .*leaked=0\b'}
    gates['r3-native-pause-open-qualified'] = pred('gate_all', {'gates':['slim-pause-begin','native-pause']}, 'lead', 30)
    gates['r3-native-pause-close-qualified'] = pred('gate_all', {'gates':['slim-pause-end','slim-resume-checked','closed-training']}, 'lead', 30)
    unknown = ('r3-stock-shield-observe', 'pilot-observe-p1-ai', 'pilot-manual-control')
    for name in unknown:
        gates[name + '-qualified'] = pred('missing_marker', {'requirement': name + ': actual active-window GIF/P1 behavior positive not yet qualified'}, cap=10)
    # Real engine previously defaulted EVERY Slim start gate to closed-training;
    # a selected pilot CPU/Player row is not closed Training. Keep exact states.
    edges = {
        'r3-settings-on':('training.closed','training.visuals-on','closed-training'),
        'r3-attack-observe':('training.visuals-on','training.visuals-on','r3-settings-on-qualified'),
        'r3-catch-observe':('training.visuals-on','training.visuals-on','r3-settings-on-qualified'),
        'r3-stock-shield-observe':('training.visuals-on','training.visuals-on','r3-settings-on-qualified'),
        'r3-native-pause-open':('training.visuals-on','native.pause.visuals-on','r3-settings-on-qualified'),
        'r3-native-pause-close':('native.pause.visuals-on','training.visuals-on','native-pause'),
        'r3-settings-off':('training.visuals-on','training.closed','r3-settings-on-qualified'),
        'pilot-prerequisites':('training.closed','training.closed','closed-training'),
        'pilot-open-player-row':('training.closed','deluxe.pilot.player','closed-training'),
        'pilot-set-cpu':('deluxe.pilot.player','deluxe.pilot.cpu','pilot-open-player-row-qualified'),
        'pilot-close-to-training-armed':('deluxe.pilot.cpu','training.pilot.cpu','pilot-set-cpu-qualified'),
        'pilot-observe-p1-ai':('training.pilot.cpu','training.pilot.cpu','pilot-close-to-training-armed-qualified'),
        'pilot-reopen-cpu-row':('training.pilot.cpu','deluxe.pilot.cpu','pilot-close-to-training-armed-qualified'),
        'pilot-set-player':('deluxe.pilot.cpu','deluxe.pilot.player','pilot-set-cpu-qualified'),
        'pilot-close-to-training-released':('deluxe.pilot.player','training.closed','pilot-set-player-qualified'),
        'pilot-manual-control':('training.closed','training.closed','pilot-close-to-training-released-qualified'),
    }
    resolved = set()
    for name, (start, end, gate) in edges.items():
        segment = manifest['segments'][name]
        segment.update(start=start, end=end, start_gate=gate, end_gate=name+'-qualified')
        if name not in unknown:
            resolved.add(name + '-qualified')
    manifest['unresolved'] = [item for item in manifest.get('unresolved', []) if item.get('gate') not in resolved]
    return {'concrete_gate_names': sorted(resolved), 'strict_unknowns': list(unknown),
            'missing_recordings_allowed_before_Up_for_authorized_recording_phase': True,
            'runtime_qualified': False}
