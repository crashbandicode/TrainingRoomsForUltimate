"""lr14 project-specific probes; canonical eden_test owns the run."""
import importlib.util,json,re,time,hashlib
from pathlib import Path
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('lr14_steps',HERE/'steps.py')
s=importlib.util.module_from_spec(spec);spec.loader.exec_module(s)
b=s.b;p=s.p
CAT=json.loads((HERE/'catalog.json').read_text())
def choice(name):return next(x for x in CAT if x['name']==name)
def ranks(document,key):
    values=document['menu'][key]
    if values[0]:raise RuntimeError('Explicit ranked fixture required')
    return [CAT[i-1] for i in sorted(range(1,len(values)),key=lambda i:values[i]) if values[i]]
def put(document,key,names):
    values=[0]*len(document['menu'][key])
    for rank,name in enumerate(names,1):values[choice(name)['index']]=rank
    document['menu'][key]=values
def next_cycle(ctx,role,name):
    key='player_roster' if role==0 else 'cpu_roster'
    current=json.loads(Path(ctx.instance['settings']).read_text())
    selected=next(x for x in ranks(current,key) if x['name']==name)
    return s.cycle(ctx,role,selected['kind'],name.replace(' ','-'))
def dispatch(ctx,action):
    ctx.log({'project_sha256':{x.name:hashlib.sha256(x.read_bytes()).hexdigest() for x in HERE.glob('*.py')},'action':action})
    try:return globals()[action](ctx)
    finally:p.snapshot(ctx)
def boot(ctx):
    source=Path(ctx.instance['events_log']);(ctx.row_dir/'events.log').write_bytes(source.read_bytes())
    lines=[x for x in source.read_text().splitlines() if 'alloc' in x or 'installed' in x]
    return {'install':lines,'comparison':'PENDING offline M5q comparison; no allocation delta inferred from RSS'}
def mii_core(ctx):
    patch=json.loads((HERE/'rc-settings/ordinary-return.json').read_text())
    put(patch,'cpu_roster',['Donkey Kong','Mii Gunner'])
    (HERE/'rc-settings/mii-bank.json').write_text(json.dumps(patch)+'\n')
    since=b.fixture(ctx,'mii-bank.json',fresh=True)
    first=next_cycle(ctx,1,'Donkey Kong')
    target=choice('Mii Gunner')['kind'];plan,m=p.membership(ctx,since,1,target)
    b.prefetch(ctx,since,1,tuple(int(x.strip(),16) for x in m[2].split(',')))
    result=next_cycle(ctx,1,'Mii Gunner')
    if 'mii=true' not in result['done'] or 'resource_pair=Some(true)' not in result['done']:raise RuntimeError('Missing Mii bank ready identity')
    return {'first':first,'plan':plan,'ready':result,'canonical':patch['mii_preferences']['cpu'],'identity_limit':'full setup trace review required'}
def mii(ctx):
    p.mii=mii_core
    return s.mii(ctx)
def pause(ctx):
    ctx.gate('training-hud-unpaused');start=b.mark(ctx)
    p.record_route(ctx,'tr-menu-open',lambda:p.chord(ctx,'b,up'))
    opened=b.wait(ctx,start,r'\[Ui\] menu opened',8)
    shot=ctx.shot('independent-menu')
    p.record_route(ctx,'tr-menu-close',lambda:ctx.press('b',120))
    ctx.gate('training-hud-unpaused')
    native=b.r3_pause(ctx)
    b.quit_css(ctx)
    retirement=[x for x in p.lines(ctx,start) if 'pause_owner' in x or 'native_pause' in x]
    fresh=b.reenter(ctx)
    return {'menu_producer':opened,'shot':str(shot),'native':native,'retirement':retirement,'fresh':fresh,'unverified':'Renderer-unavailable fixture absent in frozen diagnostic binary'}
def hats(ctx):
    since=b.fixture(ctx,'kirby-hats.json',fresh=True)
    next_cycle(ctx,0,'Kirby')
    current=json.loads(Path(ctx.instance['settings']).read_text());choices=ranks(current,'player_kirby_hats')
    result={'choices':choices,'attempts':[]}
    try:
        for n,item in enumerate(choices):
            kind=item['kind'];helper=b.wait(ctx,since,rf'\[CharacterSources\] ready slot=\d+ kind=0x{kind:x}\b',20)
            start=b.mark(ctx);ctx.play('hat-next')
            request=b.wait(ctx,start,rf'\[KirbyHats\] request role=0 kind=0x{kind:x}\b',8)
            body=re.search(r'body=(0x[0-9a-f]+)',request)[1]
            confirmed=b.wait(ctx,start,rf'\[KirbyHats\] confirmed role=0 kind=0x{kind:x} body={body}\b',10)
            result['attempts'].append({'helper':helper,'request':request,'confirmed':confirmed,'shot':str(ctx.shot('hat-'+str(n)))})
            if n==0:
                result['lookup_trace']=ctx.call('trace.arm',{'target':'main+0x1fca0b0','condition':f'x1 == {int(body,16)}','registers':['x0','x1','x30','pc'],'on_return':True,'max_hits':32,'rate_limit':1000})
                result['clear_trace']=ctx.call('trace.arm',{'target':'main+0xb96770','registers':['x0','x1','x30','pc'],'max_hits':32,'rate_limit':1000})
    finally:
        result['trace_counters']=ctx.call('trace.counters')
        (ctx.row_dir/'hats-trace.json').write_text(json.dumps(result,indent=2)+'\n')
def teardown_recover(ctx):return b.recover(ctx,'RC-TEARDOWN')
def teardown_retry(ctx):return teardown(ctx,'lr14-request-quit-frames')
def teardown(ctx,macro='lr13-request-quit-frames'):
    patch=json.loads((HERE/'rc-settings/ordinary-return.json').read_text())
    put(patch,'player_roster',['Captain Falcon','Dark Pit','Sora','Kazuya'])
    (HERE/'rc-settings/cold-slow.json').write_text(json.dumps(patch)+'\n')
    since=b.fixture(ctx,'cold-slow.json',fresh=True)
    next_cycle(ctx,0,'Captain Falcon');next_cycle(ctx,0,'Dark Pit')
    target=choice('Sora')['kind'];plan,m=p.membership(ctx,since,0,target)
    start=b.mark(ctx)
    observed=p.lines(ctx,since)
    prefetched=[x for x in observed if f'prefetched owner=0 group={m[1]} ' in x]
    ctx.log({'pending_plan':plan,'prefetched_before_request':prefetched})
    ctx.play(macro)
    try:ctx.gate('ready')
    finally:
        (ctx.row_dir/'pending-window.json').write_text(json.dumps({'plan':plan,'prefetched':prefetched,'lines':p.lines(ctx,start)},indent=2)+'\n')
    drained=[x for x in p.lines(ctx,start) if 'construction drained owner=0' in x]
    fresh=b.reenter(ctx)
    if not drained:raise RuntimeError('UI route did not finalize within pending construction; see pending-window.json')
    return {'drained':drained,'fresh':fresh}
