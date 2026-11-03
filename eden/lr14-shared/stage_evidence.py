"""Stage small receipts and exact executed source revisions; raw media stays in archive."""
import hashlib,shutil,sys,tarfile
from pathlib import Path
D=Path(__file__).resolve().parent;raw=Path(sys.argv[1]);executed=D/'executed';executed.mkdir(exist_ok=True)
def keep(name,data):
    h=hashlib.sha256(data).hexdigest();p=executed/(Path(name).stem+'-'+h+Path(name).suffix)
    if not p.exists():p.write_bytes(data)
for p in (raw/'rows').glob('*'):
    if p.suffix=='.py' or p.name.startswith('rows') and p.suffix=='.json':keep(p.name,p.read_bytes())
for p in raw.glob('attempt*.tar'):
    with tarfile.open(p) as archive:
        for item in archive:
            if item.isfile() and item.name.startswith('rows/') and item.name.endswith(('.py','.json')):
                keep(item.name,archive.extractfile(item).read())
for p in (raw/'out').rglob('*'):
    if p.is_file() and p.suffix in ('.json','.jsonl','.log'):
        dest=D/'out'/p.relative_to(raw/'out');dest.parent.mkdir(exist_ok=True,parents=True);shutil.copyfile(p,dest)
for name in ('config.json','timing.json','harvest.json','closed-units.json','yoga-unit.txt','yoga-unit-journal.log'):
    shutil.copyfile(raw/name,D/name)
for name in ('lifecycle.json','actions.jsonl','yoga-restore.json','yoga-verify.json'):
    shutil.copyfile(raw/'state'/name,D/('lifecycle-actions.jsonl' if name=='actions.jsonl' else name))
for name in ('console.log','run.json','memory.peak','yoga.trace.jsonl','trace.json'):
    shutil.copyfile(raw/'state/run-yoga'/name,D/name)
review=D/'review-media';review.mkdir(exist_ok=True)
for name in ('RC-MII/yoga-233051Z-140962-settled-Mii-Gunner.png','R3-independent-pause/yoga-233051Z-484745-independent-menu.png'):
    shutil.copyfile(raw/'out'/name,review/Path(name).name)
print('Staged receipts, executed sources, manifests and two reviewed full frames; raw archive: '+str(raw))
