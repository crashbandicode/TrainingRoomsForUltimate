"""Observe RC8's exact load and its successor; no invalid-read inference from logs."""
import hashlib,importlib.util,json,re,time
from pathlib import Path
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('lr14t_steps',HERE/'steps.py')
s=importlib.util.module_from_spec(spec);spec.loader.exec_module(s)
b=s.b;p=s.p
ELF=Path('/home/intpa/tr-lr14t-inputs/libtraining_rooms.so')
SHA='dc22cb4332f1d47e36f3acbdaa0a60f0cf8d233a05447ee57a91584ad6f9b5c6'
def arm(ctx):
    if hashlib.sha256(ELF.read_bytes()).hexdigest()!=SHA:raise RuntimeError('Diagnostic ELF identity mismatch')
    symbols=ctx.call('symbols.load',{'module':'libtraining_rooms.nro','elf':str(ELF)})
    points={}
    for name,offset in [('load',0x13f58c),('after',0x13f590)]:
        points[name]=ctx.call('trace.arm',{'target':f'libtraining_rooms.nro+{offset:#x}',
            'registers':['x0','x8','x30','pc'],'max_hits':16,'rate_limit':1000})
    armed=ctx.call('trace.list')
    for point in points.values():
        if not any(str(x['id'])==str(point['id']) and x['state']=='armed' for x in armed['points']):
            raise RuntimeError('Plugin caller tracepoint not armed; stop short boot')
    result={'elf_sha256':SHA,'symbols':symbols,'points':points,'modules':ctx.call('modules.list')}
    (ctx.runner.out/'caller-points.json').write_text(json.dumps(result,indent=2)+'\n')
    return result
def hats(ctx):
    result={'hook_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    try:
        since=b.fixture(ctx,'kirby-hats.json',fresh=True)
        result['kirby']=s.cycle(ctx,0,6,'kirby')
        result['mario_ready']=b.wait(ctx,since,r'\[CharacterSources\] ready slot=\d+ kind=0x0\b',20)
        start=b.mark(ctx);ctx.play('hat-next')
        request=b.wait(ctx,start,r'\[KirbyHats\] request role=0 kind=0x0\b',10)
        body=re.search(r'body=(0x[0-9a-f]+)',request)[1]
        result['first']=b.wait(ctx,start,rf'\[KirbyHats\] confirmed role=0 kind=0x0 body={body}\b',15)
        result['dk_ready']=b.wait(ctx,since,r'\[CharacterSources\] ready slot=\d+ kind=0x1\b',25)
        result['predicate']=ctx.call('trace.arm',{'target':'main+0x1fca0b0','condition':f'x1 == {int(body,16)}',
            'registers':['x0','x1','x8','x30','pc'],'on_return':True,'max_hits':16,'rate_limit':1000})
        result['started_ns']=time.time_ns();ctx.play('hat-next')
        points=json.loads((ctx.runner.out/'caller-points.json').read_text())['points']
        expected={str(x['id']) for x in points.values()};deadline=time.monotonic()+10
        while time.monotonic()<deadline:
            ctx.check_stop();rows=[]
            for line in Path(ctx.instance['trace_jsonl']).read_text().splitlines():
                try:row=json.loads(line)
                except ValueError:continue
                if int(row.get('wall_time_ns',0))>=result['started_ns']:rows.append(row)
            result['trace']=rows
            if expected.issubset({str(x.get('point_id')) for x in rows}):break
            time.sleep(.1)
        else:raise RuntimeError('Caller load and successor not both observed within trace cap')
        result['shot']=str(ctx.shot('second-hat-result'))
        return result
    finally:
        result['counters']=ctx.call('trace.counters')
        (ctx.row_dir/'unmapped-trace.json').write_text(json.dumps(result,indent=2)+'\n')
        p.snapshot(ctx)
