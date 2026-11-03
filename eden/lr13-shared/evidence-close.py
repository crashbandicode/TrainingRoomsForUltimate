import hashlib,json,re,shutil,tarfile
from pathlib import Path
b=Path('/home/intpa/smash-training-lobby/artifacts/trainingrooms-lead-astra4/rc8/lr13-shared')
d=Path('/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work/eden/lr13-shared')
s=b/'final-inputs';state=b/'final-state';state.mkdir(exist_ok=False)
with tarfile.open(b/'tr-lr13-state-evidence.tgz') as t:t.extractall(state,filter='data')
def write(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
life=json.loads((state/'lifecycle.json').read_text());assert life['phase']=='down' and life['released']
for name in ('lifecycle.json','actions.jsonl','instances.json','yoga-restore.json','yoga-verify.json'):
    shutil.copyfile(state/name,d/('lifecycle-actions.jsonl' if name=='actions.jsonl' else name))
for p in (state/'run-yoga').iterdir():
    if p.is_file():shutil.copyfile(p,d/p.name)
for p in (state/'post-run-yoga/user/sdmc/ultimate/TrainingRooms').glob('*.log'):shutil.copyfile(p,d/p.name)
shutil.copyfile(s/'rows.json',d/'rows.json')
for folder in ('segments','rc-settings'):shutil.copytree(s/folder,d/folder,dirs_exist_ok=True)
for p in (s/'out').rglob('*'):
    if p.is_file() and p.suffix in ('.json','.jsonl','.log'):
        target=d/'out'/p.relative_to(s/'out');target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,target)
receipts=[]
for p in (s/'out').glob('*.json'):
    r=json.loads(p.read_text())
    if 'status' in r and 'id' in r:receipts.append(r)
counts={kind:sum(r['status']==kind and not r['id'].startswith('recover-') for r in receipts) for kind in ('PASS','FAILED')}
counts['recovery_pass']=sum(r['status']=='PASS' and r['id'].startswith('recover-') for r in receipts)
counts['unrun']=json.loads((s/'out/batch-end.json').read_text())['unrun']
write(d/'receipt-accounting.json',counts)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
scan={}
for name in ('console.log','exact-unit-journal.log'):
    file=d/name;text=file.read_text(errors='replace')
    scan[name]={'bytes':file.stat().st_size,'sha256':sha(file),'matches':[x for x in text.splitlines() if re.search(r'Unmapped|panic|abort|crash|SIGBUS|fatal error|core-dump|status=7/BUS',x,re.I)]}
unit=json.loads((d/'exact-unit-status.json').read_text());props=dict(x.split('=',1) for x in unit['stdout'].splitlines() if '=' in x)
scan['unit']=props;scan['shutdown_acceptance']=props.get('Result')=='success' and props.get('KillMode')=='mixed' and props.get('ExecMainStatus')!='7' and not any(v['matches'] for k,v in scan.items() if k.endswith('.log'))
write(d/'console-scan.json',scan)
for name in ('up.log','rows.log','down.log','loop-exit.json'):shutil.copyfile(b/name,d/name)
pins=json.loads((d/'tool-pins.json').read_text())
pins['retained_hook_revisions']={p.name:sha(p) for p in d.glob('steps*.py')}
pins['manifest_bindings']={r['id']:{'sha256':r['manifest_sha256'],'retained':(d/'out/manifests'/(r['manifest_sha256']+'.json')).exists()} for r in receipts}
assert all(v['retained'] for v in pins['manifest_bindings'].values())
write(d/'tool-pins.json',pins)
print(json.dumps({'counts':counts,'shutdown':scan['shutdown_acceptance'],'unit':props,'cleanup_failures':life.get('cleanup_failures')},indent=2))
