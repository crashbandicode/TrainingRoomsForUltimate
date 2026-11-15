#!/usr/bin/env python3
"""Derive this run's TrainingRooms settings.json fixture (rc.14 cycle fix, 2026-11-14). Run it ON the Eden host,
after prep_run.py made the run folder and before Up, so the values are in place before Training's scene builds.

    /usr/bin/python3 -I cf_fixture.py <run folder> <fixture.json>

`fixture.json` holds menu values (mk-rows.py phase 3: Ready Fighters 6, Player 1 Mario > Pikachu, the CPU cycle). The
run's pinned settings.json (config.json `files`, `to` .../TrainingRooms/settings.json) is read, never changed; the
derived copy goes to <run>/inputs/settings-cf.json with a provenance file, and the pin points at it (path and sha256),
as eden_test.py derives its Qt fixture."""
import hashlib
import json
import sys
from pathlib import Path


def main(run, fixture):
    run = Path(run)
    patch = json.loads(Path(fixture).read_text())
    config = json.loads((run / 'config.json').read_text())
    pins = [pin for item in config['instances'] for pin in item.get('files', [])
            if pin.get('to', '').endswith('TrainingRooms/settings.json')]
    if len(pins) != 1:
        raise SystemExit('expected one pinned TrainingRooms/settings.json, found %d' % len(pins))
    source = Path(pins[0]['path'])
    if hashlib.sha256(source.read_bytes()).hexdigest() != pins[0]['sha256']:
        raise SystemExit('the pinned settings.json changed: %s' % source)
    data = json.loads(source.read_text())
    # A key newer than the pinned copy (`cycle_preload_ready_hats`) is added; the cf-boot row checks every value live.
    added = sorted(set(patch) - set(data['menu']))
    data['menu'].update(patch)
    derived = run / 'inputs' / 'settings-cf.json'
    derived.parent.mkdir(exist_ok=True)
    derived.write_text(json.dumps(data))
    digest = hashlib.sha256(derived.read_bytes()).hexdigest()
    derived.with_suffix('.provenance.json').write_text(json.dumps({
        'source': str(source), 'source_sha256': pins[0]['sha256'], 'derived_sha256': digest,
        'fixture': str(Path(fixture).resolve()), 'changed_keys': sorted(patch), 'added_keys': added}, indent=2) + '\n')
    pins[0].update(path=str(derived), sha256=digest)
    (run / 'config.json').write_text(json.dumps(config, indent=2) + '\n')
    print('settings fixture', derived, digest[:12], sorted(patch), 'added', added)


if __name__ == '__main__':
    main(*sys.argv[1:3])
