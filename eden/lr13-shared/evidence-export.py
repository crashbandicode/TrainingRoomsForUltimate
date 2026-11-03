"""Exact owned-unit shutdown readback and canonical Down evidence."""
import json,tarfile,subprocess
from pathlib import Path
root=Path('/home/intpa/tr-lr13-yoga-state')
life=json.loads((root/'lifecycle.json').read_text())
if life.get('phase')!='down' or not life.get('released'):raise RuntimeError('Down/release incomplete')
run=json.loads((root/'run-yoga/run.json').read_text())
command=['systemctl','--user','show',run['unit'],'--property=Id,LoadState,ActiveState,SubState,Result,ExecMainCode,ExecMainStatus,KillMode,MainPID,ExecMainExitTimestamp']
status=subprocess.run(command,capture_output=True,text=True,timeout=10)
(root/'run-yoga/exact-unit-status.json').write_text(json.dumps({'command':command,'returncode':status.returncode,'stdout':status.stdout,'stderr':status.stderr},indent=2)+'\n')
journal=subprocess.run(['journalctl','--user','-u',run['unit'],'--no-pager','-o','short-iso-precise'],capture_output=True,text=True,timeout=15)
(root/'run-yoga/exact-unit-journal.log').write_text(journal.stdout)
files=[root/name for name in ('lifecycle.json','actions.jsonl','instances.json','yoga-restore.json','yoga-verify.json')]
files += [p for p in (root/'run-yoga').iterdir() if p.is_file() and (p.suffix in ('.log','.json','.jsonl') or p.name=='memory.peak')]
for name in ('TrainingRooms','ssbu-tile-cursor','ssbu-cpu-pilot','ssbu_online_deluxe_rewrite'):
    files+=list((root/'post-run-yoga/user/sdmc/ultimate'/name).glob('*.log'))
with tarfile.open('/home/intpa/tr-lr13-state-evidence.tgz','w:gz') as archive:
    for p in files:
        if p.is_file():archive.add(p,arcname=str(p.relative_to(root)),recursive=False)
print(json.dumps({'phase':life['phase'],'released':life['released'],'cleanup_failures':life.get('cleanup_failures'),'unit':status.stdout}))
