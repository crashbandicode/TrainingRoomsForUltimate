"""Project caller of the shared CLI; retain failures, defer and recover on same boot."""
import json,subprocess,time,shutil
from pathlib import Path
root=Path('/home/intpa/tr-lr11-inputs')
manifest=root/'rows.json';out=root/'out'
runner='/home/intpa/ssbu-mod-playbook/tools/eden-runner/runner.py'
instances='/home/intpa/tr-lr11-yoga-state/instances.json'
deadline=time.monotonic()+1000
deferred=[]
for attempt in range(40):
    remaining=int(deadline-time.monotonic())
    if remaining<90: break
    result=subprocess.run(['/usr/bin/python3','-B','-I',runner,'run',str(manifest),'--instances',instances],timeout=remaining)
    if result.returncode==0: break
    if (out/'STOP').exists(): break
    if result.returncode==4:
        # New reference needs human review, not automatic promotion.
        raise RuntimeError('Reference REVIEW requires lead checkpoint')
    plan=json.loads(manifest.read_text())
    failed=[]
    for row in plan['rows']:
        receipt=out/(row['id']+'.json')
        if receipt.exists() and json.loads(receipt.read_text()).get('status')=='FAILED':failed.append((row,receipt))
    if not failed: break
    row,receipt=failed[-1]
    if row['id'].startswith('recover-') or row['start']!='training':
        deferred.append({'id':row['id'],'cause':'No qualified continuation start; '+json.loads(receipt.read_text()).get('error','')})
        break
    archive=out/'failed';archive.mkdir(exist_ok=True)
    shutil.copyfile(receipt,archive/(row['id']+'.json'))
    deferred.append({'id':row['id'],'cause':json.loads(receipt.read_text()).get('error'),'receipt':str(archive/receipt.name),
                     'continuation':'recover-'+row['id']})
    index=plan['rows'].index(row)
    plan['rows'][index]={'id':'recover-'+row['id'],'start':'training','end':'training','steps':[
        {'run':'steps.py:recover','args':{'failed':row['id']},'check':True}]}
    manifest.write_text(json.dumps(plan,indent=2)+'\n')
    (out/'deferred.json').write_text(json.dumps(deferred,indent=2)+'\n')
    # Only the runner's row-failure latch is cleared; external STOP is honored.
    (out/'STOP-INPUT').unlink(missing_ok=True)
else: raise RuntimeError('Same-boot failure iteration cap')
plan=json.loads(manifest.read_text())
unrun=[r['id'] for r in plan['rows'] if not (out/(r['id']+'.json')).exists()]
(out/'batch-end.json').write_text(json.dumps({'deferred':deferred,'unrun':unrun,'remaining_s':deadline-time.monotonic()},indent=2)+'\n')
