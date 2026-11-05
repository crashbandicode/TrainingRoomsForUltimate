"""rows.json for the rc.3 own-orbs proof boot: the boot rows of make_rows.py, then the proof rows (HB-OWN-ORBS, primary
13:44Z). Each row starts where the previous one ends."""
import hashlib
import json
import subprocess
from pathlib import Path

ROWS = Path(__file__).resolve().parent
subprocess.run(['/usr/bin/python3', '-I', str(ROWS / 'make_rows.py')], check=True)
p = ROWS / 'rows.json'
j = json.loads(p.read_text())
for f in sorted((ROWS / 'segments').glob('*.edenmacro.json')):
    name = f.name.removesuffix('.edenmacro.json')
    j['segments'][name] = {'file': f'segments/{f.name}', 'sha256': hashlib.sha256(f.read_bytes()).hexdigest(), 'cap': 20}
keep = ('00-load', '01-main', '02-stage', '03-training', '04-fighters')
j['rows'] = [r for r in j['rows'] if r['id'] in keep]
j['scope'] = ('rc.3 own-orbs proof (decision HB-OWN-ORBS, option 1) on the diag build of the release route: two Ridleys '
              'standing, attack, grab, shield, Off, the plugin menu and the vanilla pause, save/load, L+R+A, the docs GIF, an '
              'ordinary fighter, the unmapped/panic scan.')


def add(rid, hook, args=None, budget=3):
    prev = j['rows'][-1]['end']
    j['rows'].append({'id': rid, 'start': prev, 'end': prev, 'budget_min': budget,
                      'steps': [{'run': f'hb_steps.py:{hook}', 'check': True, 'args': args or {}}]})


add('10-two-ridleys', 'proof_two_ridleys')
add('11-attack', 'proof_attack')
add('12-grab', 'proof_grab')
add('13-shield', 'proof_shield')
add('14-off', 'proof_off')
add('15-menus', 'proof_menus')
add('16-save-load', 'proof_save_load')
add('17-lra', 'proof_lra')
add('18-gif', 'proof_gif', {'seconds': 7})
add('19-ordinary', 'proof_ordinary')
add('20-scan', 'proof_scan')
p.write_text(json.dumps(j, indent=2) + '\n')
print([r['id'] for r in j['rows']])
