"""Read-only post-Down unit evidence and bounded run archive; run on Eden host."""
import json,subprocess,sys
from pathlib import Path
run=Path(sys.argv[1]);journal=json.loads((run/'state/lifecycle.json').read_text())
assert journal['released'] and not journal.get('cleanup_failures'), 'Canonical Down must succeed first'
units=[]
for item in journal['instances']:
    unit=item['unit'];key=item['key']
    state=subprocess.run(['systemctl','--user','show',unit,'--property=Result,ExecMainCode,ExecMainStatus,ActiveState,MainPID,ControlGroup,MemoryPeak,KillMode'],capture_output=True,text=True,check=True,timeout=15).stdout
    (run/(key+'-unit.txt')).write_text(state)
    log=subprocess.run(['journalctl','--user','-u',unit,'--no-pager','-o','short-iso-precise'],capture_output=True,text=True,check=True,timeout=20).stdout
    (run/(key+'-unit-journal.log')).write_text(log)
    units.append({'key':key,'unit':unit,'state':dict(x.split('=',1) for x in state.splitlines() if '=' in x)})
(run/'closed-units.json').write_text(json.dumps(units,indent=2)+'\n')
files=['config.json','timing.json','RESULTS.md','harvest.json','rows','out','state','closed-units.json']
files += [p.name for p in run.glob('*.log')]+[p.name for p in run.glob('*-unit.txt')]+[p.name for p in run.glob('attempt*.tar')]
subprocess.run(['tar','--exclude=state/snapshot-yoga','-cf',str(run/'evidence.tar'),'-C',str(run),*files],check=True,timeout=60)
print(json.dumps({'released':journal['released'],'cleanup_failures':journal.get('cleanup_failures'),'units':units,'archive':str(run/'evidence.tar')}))
