"""Reproduce the bounded caller-load observation from the full retained run."""
import argparse,hashlib,json,re
from pathlib import Path
def summarize(root):
    load=lambda path:json.loads((root/path).read_text())
    probe=load('out/RC8-unmapped-trace/unmapped-trace.json')
    points=load('out/caller-points.json')['points']
    rows=probe['trace']
    before=next(x for x in rows if x.get('point_id')==str(points['load']['id']))
    after=next(x for x in rows if x.get('point_id')==str(points['after']['id']))
    native=next(x for x in rows if x.get('point_id')==str(probe['predicate']['id']) and x['event']=='return')
    if not (int(native['sequence'])<int(before['sequence'])<int(after['sequence']) and native['thread_id']==before['thread_id']==after['thread_id'] and before['frame']==after['frame']):raise ValueError('Caller observation order/context mismatch')
    console=root/'state/run-yoga/console.log';text=console.read_text(errors='replace')
    journal=root/'unit-journal.log';jtext=journal.read_text(errors='replace')
    hits=lambda pattern,body:[x for x in body.splitlines() if re.search(pattern,x,re.I)]
    life=load('state/lifecycle.json')
    return {'elf_sha256':load('out/caller-points.json')['elf_sha256'],
        'observed':{'native_return_x0':native['registers']['x0'],'load_x0':before['registers']['x0'],
        'effective_address':hex(int(before['registers']['x0'],0)+0x20),'successor_x8':after['registers']['x8'],
        'sequences':[native['sequence'],before['sequence'],after['sequence']],'frame':before['frame'],'thread':before['thread_id']},
        'console_sha256':hashlib.sha256(console.read_bytes()).hexdigest(),'console_bytes':console.stat().st_size,
        'memory_lines':hits(r'HW\.Memory|Unmapped',text),'fatal_console':hits(r'panic|abort|crash|SIGBUS|SIGSEGV',text),
        'fatal_journal':hits(r'SEGV|SIGBUS|core.dump|code=dumped|panic|fatal|runtime.*(?:expired|exceeded)',jtext),
        'cleanup_failures':life['cleanup_failures'],'released':life['released'],
        'restore':load('state/yoga-restore.json'),'verify':load('state/yoga-verify.json'),
        'scope':'Executed low-address load returned zero with no memory log if lists empty; page-zero mapping and emulator fallback mechanism unobserved'}
def main():
    p=argparse.ArgumentParser();p.add_argument('archive',type=Path);a=p.parse_args();result=summarize(a.archive)
    Path(__file__).with_name('review.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ('restore','verify')},indent=2))
if __name__=='__main__':main()
