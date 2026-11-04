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
    label='P1' if role=='p1' else 'the CPU'
    draw=b.wait(ctx,since,rf'notice_draw header="Fighters" body="Swap {label} once to load"',10)
    shot=b.capture(ctx,'notice-'+role,'Only '+role+' owes a swap')
    result=cycle(ctx,0 if role=='p1' else 1,0xb if role=='p1' else 1,'notice-'+role)
    ready=b.wait(ctx,since,r'\[PreloadStatus\] state=(?:Done|Ready) .*pending=0 unstarted=0 ',25)
    return {'waiting':line,'draw':draw,'shot':shot,'swap':result,'ready':ready,'ready_shot':b.capture(ctx,'notice-total','Settled loaded total'),'statuses':[x for x in p.lines(ctx,since) if '[PreloadStatus]' in x]}

def kirby(ctx):
    b.fixture(ctx,'kirby-hats.json',fresh=True)
    return [cycle(ctx,0,k,'kirby-'+format(k,'x')) for k in (6,0xb,0x2d,0x2c,0x1c,0x58,6)]

def hats(ctx):
    since=b.state(ctx)['scene_since'];results=[]
    failure=None
    try:
        for kind in (0,1,2,3,7):
            ready=b.wait(ctx,since,rf'\[CharacterSources\] ready slot=\d+ kind=0x{kind:x}\b',20)
            start=b.mark(ctx);ctx.play('hat-next')
            request=b.wait(ctx,start,rf'\[KirbyHats\] request role=0 kind=0x{kind:x}\b',8)
            body=re.search(r'body=(0x[0-9a-f]+)',request)[1]
            confirmed=b.wait(ctx,start,rf'\[KirbyHats\] confirmed role=0 kind=0x{kind:x} body={body}\b',10)
            results.append({'helper':ready,'request':request,'confirmed':confirmed,'shot':b.capture(ctx,'hat-'+str(kind),'Kirby hat '+str(kind))})
    except RuntimeError as error:failure=repr(error)
    result={'hats':results,'failure':failure,'departure':departure(ctx) if results else None}
    (ctx.row_dir/'hats-and-departure.json').write_text(json.dumps(result,indent=2)+'\n')
    if failure:raise RuntimeError(failure)
    return result

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
    resource=f'[[{base+0x5324680:#x}]+0xf0]'
    record=f'[[[{base+0x593b6a0:#x}]]+0x24d0]'
    setup=ctx.call('trace.arm',{'target':'main+0x67ead0','condition':'x0 == 1','registers':['x0','x1','x2','x30','pc'],
        'captures':[{'expression':hex(e),'type':'bytes','size':0x260},{'expression':'x2','type':'bytes','size':10},
                    {'expression':resource+'+0x58','type':'bytes','size':44},{'expression':record,'type':'bytes','size':208}],
        'max_hits':32,'rate_limit':1000})
    result=p.mii(ctx);result.update(tuple_points=points,cpu_row=hex(e),full_tuple_review='Current-point/current-bank/time E+1aa..1b5 and paired outfit/hat getter returns must match saved identity')
    result.update(native_setup_point=setup,setup_resource_expression=resource,setup_record_expression=record)
    (ctx.row_dir/'tuple-observation.json').write_text(json.dumps(result,indent=2)+'\n')
    return result

def pilot(ctx):
    from PIL import Image
    ctx.gate('training-hud-unpaused');gates=b.load('pilot_gates')
    ctx.log({'pilot_marker_sha256':hashlib.sha256((HERE/'pilot-markers.json').read_bytes()).hexdigest()})
    def selected(want,step=False):
        for i in range(8):
            if step:ctx.play('pilot-page-step')
            shot=ctx.shot('pilot-current-'+want+'-'+str(i));shape=gates.classify_image(shot,'page')
            if shape and shape['shape']==want:return shape
            if not step:time.sleep(.15)
        raise RuntimeError('Current selected Pilot shape did not become '+want)
    def close():
        for i in range(8):
            ctx.play('pilot-page-step');shot=ctx.shot('pilot-current-closed-'+str(i))
            with Image.open(shot) as im:pixels=list(im.convert('L').crop((994,0,1227,60)).getdata())
            if sum(x>140 for x in pixels)/len(pixels)>=.70:
                ctx.gate('training-hud-unpaused');return str(shot)
        raise RuntimeError('Pilot panel did not close')
    result={'manual_before':selected('Player',True)};armed=False;panel=True
    try:
        p.record_route(ctx,'pilot-arm',lambda:p.chord(ctx,'zr,right'));armed=True
        result['armed']=selected('CPU');result['closed_armed']=close();panel=False
        path=ctx.row_dir/'pilot-current-ai.gif'
        ctx.call('recording.start',{'path':str(path),'duration_ms':5000,'width':1280,'fps':10})
        end=time.monotonic()+10
        while time.monotonic()<end:
            status=ctx.call('recording.status')
            if status.get('state') in ('finished','idle','completed') and path.exists():break
            ctx.check_stop();time.sleep(.25)
        else:raise RuntimeError('Pilot AI clip not finalized')
        result['ai_clip']=str(path);result['reopened_cpu']=selected('CPU',True);panel=True
        p.record_route(ctx,'pilot-release',lambda:p.chord(ctx,'zr,left'))
        result['released']=selected('Player');armed=False
        result['closed_released']=close();panel=False
        result['manual_clip']=b.recorded_action(ctx,'tr2-jab-x4')
        return result
    finally:
        if armed:
            if not panel:selected('CPU',True)
            p.record_route(ctx,'pilot-release',lambda:p.chord(ctx,'zr,left'))
            result['finally_released']=selected('Player');close()
        (ctx.row_dir/'pilot-current-result.json').write_text(json.dumps(result,indent=2)+'\n')

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
def hats_canonical(ctx):
    patch=json.loads((HERE/'rc-settings/kirby-hats.json').read_text())
    patch['menu'].update(cycle_preload_original_bank=[0,1],cycle_preload_group_swap=[0,1],cycle_preload_bank_members=[1,0,0])
    (HERE/'rc-settings/hats-canonical.json').write_text(json.dumps(patch)+'\n')
    b.fixture(ctx,'hats-canonical.json',fresh=True)
    cycle(ctx,0,6,'canonical-kirby')
    return hats(ctx)

def final(ctx):
    result={'counters':b.final(ctx),'same_boot_retries':{}}
    for name,action in [('HATS-canonical-owner',lambda:hats_canonical(ctx)),('NOTICE-cpu-drawn',lambda:notice_roles(ctx,'cpu')),('PILOT-current-shape',lambda:pilot(ctx)),('MII-native-setup',lambda:mii(ctx))]:
        started=time.time_ns()
        try:result['same_boot_retries'][name]={'status':'PASS','value':action()}
        except Exception as error:
            p.snapshot(ctx)
            result['same_boot_retries'][name]={'status':'FAILED','error':repr(error),'continuation':b.recover(ctx,name)}
        result['same_boot_retries'][name].update(started_ns=started,ended_ns=time.time_ns())
        (ctx.row_dir/(name+'.json')).write_text(json.dumps(result['same_boot_retries'][name],indent=2)+'\n')
    return result
