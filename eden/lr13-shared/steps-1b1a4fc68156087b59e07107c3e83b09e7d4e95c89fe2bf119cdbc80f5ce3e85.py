"""lr13 project probes; shared runner/lifecycle retain ownership of execution."""
import hashlib,importlib.util,json,re,time
from pathlib import Path
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('prior',HERE/'prior.py')
p=importlib.util.module_from_spec(spec);spec.loader.exec_module(p)
b=p.b
LOADED={x.name:hashlib.sha256(x.read_bytes()).hexdigest() for x in HERE.glob('*.py')}
original_cycle=b.cycle

def before(ctx,role):
    ctx.runner.marks[(ctx.on,'hud-before-'+str(role))]=ctx.shot('hud-before-'+str(role))

def settled(ctx,role,name):
    ctx.gate('hud-settled-'+str(role))
    return b.capture(ctx,'settled-'+name,'Settled full body AND matching HUD; visual review required')

def cycle(ctx,role,target,name):
    before(ctx,role)
    result=original_cycle(ctx,role,target,name)
    result['settled_shot']=settled(ctx,role,name)
    return result
b.cycle=cycle

def dispatch(ctx,action,**args):
    ctx.log({'loaded_project_sha256':LOADED,'action':action,'args':args})
    try:return globals()[action](ctx,**args)
    finally:p.snapshot(ctx)

def roster(patch,key,index):
    a=[0]*len(patch['menu'][key]);a[index]=1;patch['menu'][key]=a

def notice_roles(ctx,role):
    patch=json.loads((HERE/'rc-settings/ordinary-return.json').read_text())
    # Nonempty original-only rosters produce no alternatives after original exclusion.
    # Native Training selection remains Steve/Lucas at each gated reentry.
    for key in ('player_kirby_hats','cpu_kirby_hats'):roster(patch,key,7)
    roster(patch,'cpu_roster',2 if role=='cpu' else 38)
    if role=='cpu':roster(patch,'player_roster',77)
    name='notice-valid-'+role+'.json';(HERE/'rc-settings'/name).write_text(json.dumps(patch)+'\n')
    since=b.fixture(ctx,name,fresh=True)
    mask={'p1':1,'cpu':2}[role]
    line=b.wait(ctx,since,rf'\[PreloadStatus\] state=Waiting .*swap={mask}\b',18)
    shot=b.capture(ctx,'notice-'+role,'Only '+role+' owes a swap')
    result=cycle(ctx,0 if role=='p1' else 1,0xb if role=='p1' else 1,'notice-'+role)
    return {'waiting':line,'shot':shot,'swap':result,'statuses':[x for x in p.lines(ctx,since) if '[PreloadStatus]' in x]}

def kirby(ctx):
    b.fixture(ctx,'kirby-hats.json',fresh=True)
    return [cycle(ctx,0,k,'kirby-'+format(k,'x')) for k in (6,0xb,0x2d,0x2c,0x1c,0x58,6)]

def hats(ctx):
    since=b.state(ctx)['scene_since'];results=[]
    for kind in (0,1,2,3,7):
        ready=b.wait(ctx,since,rf'\[CharacterSources\] ready slot=\d+ kind=0x{kind:x}\b',20)
        start=b.mark(ctx);ctx.play('hat-next')
        request=b.wait(ctx,start,rf'\[KirbyHats\] request role=0 kind=0x{kind:x}\b',8)
        body=re.search(r'body=(0x[0-9a-f]+)',request)[1]
        confirmed=b.wait(ctx,start,rf'\[KirbyHats\] confirmed role=0 kind=0x{kind:x} body={body}\b',10)
        results.append({'helper':ready,'request':request,'confirmed':confirmed,'shot':b.capture(ctx,'hat-'+str(kind),'Kirby hat '+str(kind))})
    return {'hats':results,'departure':departure(ctx)}

def departure(ctx):
    start=b.mark(ctx);away=cycle(ctx,0,0xb,'copied-kirby-away')
    fallback=b.wait(ctx,start,r'\[KirbyPark\] fallback owner=0 .*canonical Reload',8)
    return {'away':away,'canonical':fallback,'rob':cycle(ctx,0,0x2d,'normal-rob')}

def special(ctx,name):
    since=b.fixture(ctx,'specials-'+name+'.json',fresh=True)
    prefix=[cycle(ctx,0,k,'prefix-'+format(k,'x')) for k in (0xb,0x1f,0x2e)]
    plan,m=p.membership(ctx,since,0,0x2d)
    b.prefetch(ctx,since,0,tuple(int(x.strip(),16) for x in m[2].split(',')))
    before(ctx,0);g,strict,s,read=b.tools(ctx)
    ordinary=g.cold_group(s,read,strict,None,0,0x2d,'ordinary-async')
    ordinary['settled']=settled(ctx,0,'ordinary-rob')
    if name=='ordinary':return {'prefix':prefix,'ordinary':ordinary}
    prefix.extend(cycle(ctx,0,k,'prefix-'+format(k,'x')) for k in (0x2c,0x1c))
    target=cycle(ctx,0,0x5a,'aegis')
    paused=b.r3_pause(ctx)
    return {'ordinary':ordinary,'target':target,'pause':paused}

def mii(ctx):
    modules=ctx.call('modules.list');entries=modules.get('modules',[]) if isinstance(modules,dict) else modules
    base=int(str(next(x for x in entries if x['name']=='main')['base']),0)
    points=[]
    for target,hint in (('main+0x3282b70',69),('main+0x32870b0',0)):
        points.append(ctx.call('trace.arm',{'target':target,'condition':f'x1 == {hint}','registers':['x0','x1','x30','pc'],'on_return':True,'max_hits':64,'rate_limit':1000}))
    e=base+0x52c5730+0x260
    points.append(ctx.call('trace.arm',{'target':'main+0x17e64d0','condition':'x1 == 74','registers':['x0','x1','x30','pc'],
        'captures':[{'expression':hex(e),'type':'bytes','size':0x260},{'expression':'x0+0x58','type':'bytes','size':44}],
        'max_hits':64,'rate_limit':1000}))
    result=p.mii(ctx);result.update(tuple_points=points,cpu_row=hex(e),full_tuple_review='Current-point/current-bank/time E+1aa..1b5 and paired outfit/hat getter returns must match saved identity')
    (ctx.row_dir/'tuple-observation.json').write_text(json.dumps(result,indent=2)+'\n')
    return result

def pilot(ctx):
    # Current selected Player ref is exact. Always release after an arm attempt,
    # retaining unknown CPU shape for same-boot review instead of leaving P1 armed.
    try:return p.pilot(ctx)
    finally:
        p.record_route(ctx,'pilot-release',lambda:p.chord(ctx,'zr,left'))
        ctx.shot('pilot-release-finally')

def independent_pause(ctx):
    b.fixture(ctx,'ordinary-return.json',fresh=True)
    start=b.mark(ctx)
    pause=b.r3_pause(ctx)
    b.quit_css(ctx)
    retired=[x for x in p.lines(ctx,start) if 'pause_owner' in x or 'native_pause' in x]
    fresh=b.reenter(ctx)
    ctx.log({'native_pause_retirement':retired,'pause':pause,'fresh':fresh,
        'unproven':'Renderer-unavailable independence: this immutable build has renderer installed; native observer source order alone is insufficient'})
    return {'pause':pause,'retirement':retired,'fresh':fresh,'independent_renderer':'DEFERRED; no renderer-unavailable fixture in this immutable plugin set'}

def teardown(ctx,index):
    b.cold_prefix(ctx);start=b.mark(ctx)
    ctx.play('lr13-request-quit-frames');ctx.gate('ready')
    observed=p.lines(ctx,start)
    queued=[x for x in observed if '[GroupSwap] bodies queued owner=0' in x]
    built=[x for x in observed if '[GroupSwap] built owner=0' in x]
    drains=[x for x in observed if '[GroupSwap] construction drained owner=0' in x]
    clean=b.wait(ctx,start,r'\[GroupSwap\] teardown owner=0 reason=finalize .*skipped=0 alias_ok=true',8)
    fresh=b.reenter(ctx);continuation=cycle(ctx,0,0xb,'pending-reentry')
    result={'macro_frames':22,'queued':queued,'built':built,'drains':drains,'finalize':clean,'fresh':fresh,'continuation':continuation}
    (ctx.row_dir/'pending-window.json').write_text(json.dumps(result,indent=2)+'\n')
    if not queued or not drains:raise RuntimeError('No queued-before-finalize pending drain: see pending-window.json')
    return result

def recover(ctx,failed):return b.recover(ctx,failed)
def final(ctx):return b.final(ctx)
