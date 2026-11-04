"""Extract bounded lr14 route timelines; not a universal UI lower bound."""
import hashlib,json,re
from pathlib import Path
D=Path(__file__).resolve().parent
def timeline(lines):
    def first(pattern):return next((x for x in lines if re.search(pattern,x)),None)
    observations={
        'queue':first(r'\[GroupSwap\] bodies queued owner=0\b'),
        'built':first(r'\[GroupSwap\] built owner=0\b'),
        'pause':first(r'pause_owner begin'),
        'earliest_scene_end_observed':first(r'\[(?:Destroy|GroupSwap|Registry|Ledger)\].*owner=0 .*reason=finalize'),
        'group_teardown':first(r'\[GroupSwap\] teardown owner=0 reason=finalize'),
        'drained':first(r'\[GroupSwap\] construction drained owner=0\b')}
    queue=observations['queue']
    delta=lambda line:int(line.split()[0])-int(queue.split()[0]) if line and queue else None
    return {'observations':observations,'queue_relative_ms':{k:delta(v) for k,v in observations.items()}}
def main():
    result=[]
    for path in sorted((D.parent/'lr14-shared/out').glob('RC-TEARDOWN*/pending-window.json')):
        data=json.loads(path.read_text())
        if 'lines' not in data:continue
        result.append({'route':path.parent.name,'source':str(path.relative_to(D.parent)),
            'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),**timeline(data['lines'])})
    output={'scope':'Earliest explicit owner0 finalization marker retained in each route window; not an inferred actual scene-change timestamp or universal UI bound. Negative pause offsets mean pause preceded bodies-queued.', 'routes':result}
    (D/'teardown-timeline.json').write_text(json.dumps(output,indent=2)+'\n')
    print(json.dumps([{'route':r['route'],**r['queue_relative_ms']} for r in result],indent=2))
if __name__=='__main__':main()
