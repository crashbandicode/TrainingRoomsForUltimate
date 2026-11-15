import json,subprocess,sys
from pathlib import Path
r=Path('/home/intpa/eden-runs/rc15-crop-replay-d520d325')
p=r/'rows/crop_runtime.py'
s=p.read_text()
start=s.index('    cov.prep(ctx, \'cov-replay\')',s.index('def replay_clip_redraw'))
end=s.index("    ctx.press('a', ms=40)",start)
s=s[:start]+"    frames = cov.diag(ctx)\n    out = {'continued_from': 'visible Fighter Mismatch Keep; prior failed receipt preserved'}\n"+s[end:]
p.write_text(s)
j=json.loads((r/'rows/rows.json').read_text())
j['rows']=[dict(j['rows'][-1],id='crop-replay-continuation')]
j['rows'][0]['start']='training'
j['start']='training'
(r/'rows/continue.json').write_text(json.dumps(j,indent=2))
print(list(r.glob('**/instances.json')),flush=True)
inst=r/'instances.json'
if not inst.exists():inst=next(r.glob('**/instances.json'))
raise SystemExit(subprocess.call([sys.executable,'/home/intpa/ssbu-mod-playbook/tools/eden-runner/runner.py','run',str(r/'rows/continue.json'),'--instances',str(inst),'--out',str(r/'out-continuation')]))
