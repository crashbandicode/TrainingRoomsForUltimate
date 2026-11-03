"""Same-boot shared-runner caller; failures retained with both log generations."""
import json,subprocess,time,shutil
from pathlib import Path
root=Path('/home/intpa/tr-lr12-inputs');manifest=root/'rows.json';out=root/'out'
runner='/home/intpa/ssbu-mod-playbook/tools/eden-runner/runner.py'
instances='/home/intpa/tr-lr12-yoga-state/instances.json'
deadline=time.monotonic()+960
deferred=[]
for attempt in range(40):
    remaining=int(deadline-time.monotonic())
    if remaining<75:break
    result=subprocess.run(['/usr/bin/python3','-B','-I',runner,'run',str(manifest),'--instances',instances],timeout=remaining)
    if result.returncode==0 or (out/'STOP').exists():break
    if result.returncode==4:raise RuntimeError('Reference REVIEW requires lead checkpoint')
    plan=json.loads(manifest.read_text());failed=[]
    for row in plan['rows']:
        receipt=out/(row['id']+'.json')
        if receipt.exists() and json.loads(receipt.read_text()).get('status')=='FAILED':failed.append((row,receipt))
    if not failed:break
    row,receipt=failed[-1]
    for inst in json.loads(Path(instances).read_text()).values():
        p=Path(inst['diag_log'])
        for source in (p,p.with_name('practice-diagnostics.previous.log')):
            if source.exists():shutil.copyfile(source,out/row['id']/source.name)
    if row['id'].startswith('recover-') or row['start']!='training':break
    archive=out/'failed';archive.mkdir(exist_ok=True);shutil.copyfile(receipt,archive/receipt.name)
    deferred.append({'id':row['id'],'cause':json.loads(receipt.read_text()).get('error'),'receipt':str(archive/receipt.name),'continuation':'recover-'+row['id']})
    plan['rows'][plan['rows'].index(row)]={'id':'recover-'+row['id'],'start':'training','end':'training','steps':[
        {'run':'steps.py:dispatch','args':{'action':'recover','failed':row['id']},'check':True}]}
    manifest.write_text(json.dumps(plan,indent=2)+'\n')
    (out/'deferred.json').write_text(json.dumps(deferred,indent=2)+'\n')
    (out/'STOP-INPUT').unlink(missing_ok=True)
plan=json.loads(manifest.read_text())
(out/'batch-end.json').write_text(json.dumps({'deferred':deferred,'unrun':[r['id'] for r in plan['rows'] if not (out/(r['id']+'.json')).exists()],'remaining_s':deadline-time.monotonic()},indent=2)+'\n')
