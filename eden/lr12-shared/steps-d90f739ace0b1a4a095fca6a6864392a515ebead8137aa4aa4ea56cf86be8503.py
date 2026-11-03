"""Clause-focused project hooks; shared runner owns transport, stop and receipts."""
import importlib.util
import json
import re
import time
from pathlib import Path

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('lr12_base',HERE/'base.py')
b=importlib.util.module_from_spec(spec);spec.loader.exec_module(b)

def snapshot(ctx):
    p=Path(ctx.instance['diag_log'])
    for source in (p,p.with_name('practice-diagnostics.previous.log')):
        if source.exists(): (ctx.row_dir/source.name).write_bytes(source.read_bytes())

def dispatch(ctx,action,**args):
    import hashlib
    ctx.log({'project_hook_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'action':action,'args':args})
    try: return globals()[action](ctx,**args)
    finally: snapshot(ctx)

def lines(ctx,since=0):
    g,_,_,read=b.tools(ctx)
    return g._new(read,since)

def chord(ctx,buttons,hold=120):
    names=buttons.split(',')
    try:
        for button in names[:-1]:ctx.call('input.button',{'button':button,'state':'press','lease_ms':1500})
        ctx.press(names[-1],hold)
    finally:ctx.call('input.reset')

def record_route(ctx,name,action):
    path=HERE/'segments'/(name+'.edenmacro.json')
    if path.exists():return ctx.play(name)
    ctx.call('macro.record',{'name':name,'path':str(path),'player':0,'countdown_ms':0})
    try:return action()
    finally:
        ctx.call('input.reset');ctx.call('macro.stop')
        if path.exists():
            import hashlib
            ctx.runner.plan['segments'][name]={'file':'segments/'+path.name,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'cap':30}
            ctx.runner.manifest_path.write_text(json.dumps(ctx.runner.plan,indent=2)+'\n')

def kirby(ctx):
    b.fixture(ctx,'kirby-hats.json',fresh=True)
    return [b.cycle(ctx,0,target,'kirby-'+format(target,'x')) for target in (6,0xb,0x2d,0x2c,0x1c,0x58,6)]

def hats(ctx):
    g,strict,s,read=b.tools(ctx)
    ready=[x for x in lines(ctx,b.state(ctx)['scene_since']) if re.search(r'\[MiiOrdinaryPlayer\] (?:bank )?ready .*released=true',x)]
    if not ready or not re.search(r'\btarget=0x6\b',ready[-1]):raise RuntimeError('Current adapter-ready Kirby prerequisite absent')
    result={'hats':g.hats(s,read,None,(0,1,2,3,7))}
    result['departure']=g.kirby_to_rob(s,read,strict,None,0xb)
    result['rob']=b.cycle(ctx,0,0x2d,'rob-after-hats')
    ctx.gate('training-hud-unpaused');b.capture(ctx,'settled-rob','Normal ROB body/HUD after copied Kirby departure')
    return result

def membership(ctx,since,role,target):
    def find():
        for line in reversed(lines(ctx,since)):
            m=re.search(rf'\[GroupSwap\] plan owner={role} group=(\d+) kinds=\[([^]]+)\] special=(\w+)',line)
            if m and target in [int(x.strip(),16) for x in m[2].split(',')]:return line,m
    end=time.monotonic()+15
    while time.monotonic()<end:
        found=find()
        if found:return found
        ctx.check_stop();time.sleep(.1)
    raise RuntimeError(f'Current fixture has no planned group for role={role} target={target:#x}')

def special(ctx,name):
    since=b.fixture(ctx,'specials-'+name+'.json',fresh=True)
    results=[]
    # Three-member fixture: b/1f/2e are the first window; 2d/2c/1c is
    # the ordinary detached group. Membership is read from THIS scene's plan.
    for target in (0xb,0x1f,0x2e):results.append(b.cycle(ctx,0,target,'prefix-'+format(target,'x')))
    plan,m=membership(ctx,since,0,0x2d)
    kinds=tuple(int(x.strip(),16) for x in m[2].split(','))
    pref=b.prefetch(ctx,since,0,kinds)
    g,strict,s,read=b.tools(ctx)
    ordinary=g.cold_group(s,read,strict,None,0,0x2d,'ordinary-async')
    if name=='ordinary':return {'plan':plan,'prefetch':pref,'async':ordinary}
    for target in (0x2c,0x1c):results.append(b.cycle(ctx,0,target,'prefix-'+format(target,'x')))
    target={'pt':0x24,'aegis':0x5a,'ic':0x4b}[name]
    plan,m=membership(ctx,since,0,target)
    kinds=tuple(int(x.strip(),16) for x in m[2].split(','))
    pref=b.prefetch(ctx,since,0,kinds)
    start=b.mark(ctx);result=b.cycle(ctx,0,target,'special-'+name)
    build=b.wait(ctx,start,rf'\[GroupSwap\] build owner=0 group={m[1]} .*mode=sync reason=',15)
    built=b.wait(ctx,start,rf'\[GroupSwap\] built owner=0 group={m[1]} ok=true',15)
    ctx.gate('training-hud-unpaused');b.capture(ctx,'settled-'+name,'Native forms/partner body and HUD')
    return {'ordinary_async':ordinary,'plan':plan,'prefetch':pref,'sync':build,'built':built,'ready':result,
            'remaining':'Form switching/partner behavior is only covered if shown in retained frames'}

def mii(ctx):
    # Seed the CPU bank with an ordinary first member, rather than requesting
    # the saved Mii before a native CPU bank exists (lr11 canonical reload).
    patch=json.loads((HERE/'rc-settings/ordinary-return.json').read_text())
    roster=[0]*len(patch['menu']['cpu_roster']);roster[2]=1;roster[89]=2
    patch['menu']['cpu_roster']=roster
    (HERE/'rc-settings/mii-bank.json').write_text(json.dumps(patch)+'\n')
    modules=ctx.call('modules.list')
    entries=modules.get('modules',[]) if isinstance(modules,dict) else modules
    main=next(x for x in entries if x['name']=='main')
    base=int(str(main['base']),0)
    # R+68 is a selector index, not a pointer. Follow the actual holder -> G
    # -> record vector (mii_native_access.rs:133-160, native_values RECORD_LEN).
    record_expr=f'[[[{base+0x593b6a0:#x}]]+0x24d0]'
    point=ctx.call('trace.arm',{'target':'main+0x17e64d0','condition':'x1 == 74','registers':['x0','x1','pc','x30'],
        'captures':[{'expression':record_expr,'type':'bytes','size':208},{'expression':'x0+0x68','type':'u16'},{'expression':'x0+0x58','type':'bytes','size':44}],
        'on_return':False,'max_hits':64,'rate_limit':1000})
    ctx.log({'mii_resource_point':point,'abi':'r_handle.rs:133 R=x0 kind=x1; mii_native_access.rs:133-160 holder/G/vector; R+68 selector index','record_expression':record_expr})
    since=b.fixture(ctx,'mii-bank.json',fresh=True)
    first=b.cycle(ctx,1,1,'cpu-bank-dk')
    plan,m=membership(ctx,since,1,0x4a)
    b.prefetch(ctx,since,1,tuple(int(x.strip(),16) for x in m[2].split(',')))
    result=b.cycle(ctx,1,0x4a,'saved-mii-bank')
    canonical=bytes.fromhex(patch['mii_preferences']['cpu'][2])
    records=[]
    for line in Path(ctx.instance['trace_jsonl']).read_text().splitlines():
        row=json.loads(line)
        if row.get('event')!='entry':continue
        caps={c['expression']:c for c in row.get('captures',[])}
        rec=caps.get(record_expr)
        if not rec or rec.get('error'):continue
        raw=bytes.fromhex(rec.get('hex',rec.get('value','')).removeprefix('0x'))
        index=caps.get('x0+0x68',{})
        if index.get('error') or index.get('value')!='1':continue
        raw=raw[104:208]
        if len(raw)==104 and raw[0x26:88]==canonical[1+0x26:89]:records.append(row)
    (ctx.row_dir/'mii-matched-traces.json').write_text(json.dumps(records,indent=2)+'\n')
    if not records:raise RuntimeError('No resource-read record matches canonical masked CharInfo; retained trace requires review')
    if 'mii=true' not in result['done'] or 'resource_pair=Some(true)' not in result['done']:raise RuntimeError('Exact bank ready missing: '+result['done'])
    return {'first':first,'plan':plan,'ready':result,'matching_record_reads':len(records),'canonical':canonical.hex(),'trace':point}

def notice_roles(ctx,role):
    patch=json.loads((HERE/'rc-settings/ordinary-return.json').read_text())
    for key in ('player_kirby_hats','cpu_kirby_hats'):patch['menu'][key]=[0]*len(patch['menu'][key])
    if role=='cpu':
        patch['menu']['cpu_roster']=[0]*len(patch['menu']['cpu_roster'])
        patch['menu']['cpu_roster'][2]=1
    if role!='both':
        key='cpu_roster' if role=='p1' else 'player_roster'
        patch['menu'][key]=[0]*len(patch['menu'][key])
    name='notice-'+role+'.json';(HERE/'rc-settings'/name).write_text(json.dumps(patch)+'\n')
    since=b.fixture(ctx,name,fresh=True)
    mask={'p1':1,'cpu':2,'both':3}[role]
    start=b.wait(ctx,since,rf'\[PreloadStatus\] state=Waiting .*swap={mask}\b',15)
    b.capture(ctx,'notice-'+role,'Actual named loading role '+role)
    transitions=[]
    if role in ('both','p1'):transitions.append(b.cycle(ctx,0,0xb,'notice-p1'))
    if role in ('both','cpu'):transitions.append(b.cycle(ctx,1,1 if role=='cpu' else 0x4a,'notice-cpu'))
    settled=None
    if role!='both':settled=b.wait(ctx,since,r'\[PreloadStatus\] state=(?:Done|Ready) .*pending=0 unstarted=0 ',25)
    statuses=[x for x in lines(ctx,since) if '[PreloadStatus]' in x]
    b.capture(ctx,'notice-after','Ready total must agree with built+prefetched/planned')
    return {'role':role,'start':start,'transitions':transitions,'statuses':statuses,'settled_total':settled,'ready_total_review':'pending'}

def pilot(ctx):
    ctx.gate('training-hud-unpaused')
    root=Path(ctx.instance['settings']).parent.parent
    events=(root/'TrainingRooms/events.log').read_text();pilotlog=(root/'ssbu-cpu-pilot/events.log').read_text()
    (ctx.row_dir/'pilot-events.log').write_text(pilotlog)
    if 'p1_cpu=not_installed reason=standalone_pilot_present' not in events or 'menu=registered' not in pilotlog:raise RuntimeError('Pilot install/standdown missing')
    gates=b.load('pilot_gates')
    def page(expected):
        end=time.monotonic()+20
        for index in range(8):
            if time.monotonic()>end:break
            record_route(ctx,'pilot-page-step',lambda:chord(ctx,'l,r,down'))
            shot=ctx.shot('pilot-page-'+str(index));result=gates.classify_image(shot,'page')
            if result and result['shape']==expected:return result
        raise RuntimeError('Finite Pilot discovery did not reach selected '+expected+' row')
    def check(expected):
        end=time.monotonic()+3
        while time.monotonic()<end:
            result=gates.classify_image(ctx.shot('pilot-'+expected),'page')
            if result and result['shape']==expected:return result
            time.sleep(.15)
        raise RuntimeError('Pilot selected row did not become '+expected)
    def close(expected):
        from PIL import Image
        for index in range(8):
            ctx.play('pilot-page-step')
            shot=ctx.shot('pilot-close-'+expected+'-'+str(index))
            result=gates.classify_image(shot,'hud')
            with Image.open(shot) as im:
                pixels=list(im.convert('L').crop((994,0,1227,60)).getdata())
            if result and result['shape']==expected and sum(p>140 for p in pixels)/len(pixels)>=.70:
                ctx.gate('training-hud-unpaused');return str(shot)
        raise RuntimeError('Pilot close-to-'+expected+' HUD route not qualified')
    result={'player':page('Player')}
    record_route(ctx,'pilot-arm',lambda:chord(ctx,'zr,right'));result['armed']=check('CPU')
    result['armed_hud']=close('CPU')
    path=ctx.row_dir/'pilot-ai.gif'
    ctx.call('recording.start',{'path':str(path),'duration_ms':5000,'width':1280,'fps':10})
    end=time.monotonic()+10
    while time.monotonic()<end:
        status=ctx.call('recording.status')
        if status.get('error'):raise RuntimeError(str(status))
        if status.get('state') in ('finished','idle','completed') and path.exists():break
        time.sleep(.25)
    else:raise RuntimeError('Pilot AI recording did not finalize')
    result['reopened_cpu']=page('CPU')
    record_route(ctx,'pilot-release',lambda:chord(ctx,'zr,left'));result['released']=check('Player')
    result['released_hud']=close('Player')
    result['manual']=b.recorded_action(ctx,'tr2-jab-x4')
    result['behavior_review']='Actual P1 AI/manual response must be reviewed in both GIFs'
    return result

def r3_attack(ctx):
    b.fixture(ctx,'ordinary-return.json',fresh=True)
    b.cycle(ctx,0,0xb,'attack-falcon')
    b.r3_settings(ctx,True)
    result=b.r3_action(ctx,'tr2-jab-x4')
    b.r3_settings(ctx,False)
    return result

def independent_pause(ctx):
    ctx.gate('training-hud-unpaused');since=b.mark(ctx)
    record_route(ctx,'tr-menu-open',lambda:chord(ctx,'b,up'))
    opened=b.wait(ctx,since,r'\[Ui\] menu opened',8)
    b.capture(ctx,'independent-menu','Independent menu input owner; no native pause inference')
    record_route(ctx,'tr-menu-close',lambda:ctx.press('b',120))
    ctx.gate('training-hud-unpaused')
    result=b.r3_pause(ctx)
    result['independent_menu']=opened
    result['retirement_scope']='Menu close plus native owner begin/end/resume; scene retirement checked in TRIO rows'
    return result

def trio(ctx,index):
    since=b.fixture(ctx,'ordinary-return.json',fresh=True)
    b.cycle(ctx,0,0xb,'trio-falcon');b.cycle(ctx,1,0x4a,'trio-gunner')
    # Observe an actual managed actor before requiring its retirement.
    ready=b.wait(ctx,since,r'\[ManagedSourceCleanup\] canonical ready source=',20)
    before=b.mark(ctx);b.quit_css(ctx)
    clean=b.wait(ctx,before,r'\[GroupSwap\] teardown owner=0 reason=finalize .*skipped=0 alias_ok=true',10)
    retired=b.wait(ctx,before,r'\[ManagedSourceCleanup\] retirement .*entry_destroyed=true reservation_cleared=true .*listener_removed=Some\(true\)',8)
    match=re.search(r'owner=(0x[0-9a-f]+).*ready_actors=(\d+) actor_events=(\d+) valid_actors=(\d+) ready_cleanups=(\d+)',retired)
    if not match or int(match[2])<1 or len(set(match.groups()[1:]))!=1:raise RuntimeError('Managed actor retirement counts disagree: '+retired)
    owner=re.escape(match[1])
    claim=b.wait(ctx,before,rf'\[ManagedSourceCleanup\] (?:extended claim slot=\d+|claim) owner={owner} released=true',8)
    fresh=b.reenter(ctx);continuation=b.cycle(ctx,0,0xb,'trio-continuation')
    return {'ready_source':ready,'clean':clean,'retired':retired,'claim':claim,'fresh':fresh,'continuation':continuation}

def teardown(ctx,index):
    b.cold_prefix(ctx)
    g,strict,s,read=b.tools(ctx)
    since,request=g._request(s,read,strict,0,0x2e,hold=40)
    group,build=g._async_group(s,read,strict,0,since,request)
    queued=b.wait(ctx,since,rf'\[GroupSwap\] bodies queued owner=0 group={group} tasks=',5)
    terminal=g._terminal_observed(read,strict,0,queued,request)
    ctx.log({'pending_gate':queued,'terminal_before_dispatch':terminal})
    if terminal:raise RuntimeError('Build already settled at bodies-queued observation; pending quit unexercised')
    # Shortest previously qualified native route, every screen remains gated.
    ctx.press('plus',40);ctx.gate('native-pause')
    ctx.play('focus-native-quit-frames');ctx.gate('quit-focus')
    ctx.play('confirm-native-quit-frames');ctx.gate('ready')
    clean=b.wait(ctx,since,r'\[GroupSwap\] teardown owner=0 reason=finalize .*skipped=0 alias_ok=true',8)
    drains=[x for x in lines(ctx,since) if re.search(rf'\[GroupSwap\] construction drained owner=0 group={group}\b',x)]
    fresh=b.reenter(ctx);again=b.cycle(ctx,0,0xb,'pending-reentry')
    result={'queued':queued,'clean':clean,'drains':drains,'fresh':fresh,'continuation':again}
    ctx.log({'teardown_observation':result})
    if not drains:raise RuntimeError('Native quit completed after settlement; no pending destruction/drain; continuation observed')
    return result

def recover(ctx,failed):return b.recover(ctx,failed)
def final(ctx):
    result={'counters':b.final(ctx),'same_boot_retries':{}}
    for name,action in (('MII-index89',mii),('HATS-settled-helpers',hats_settled)):
        try:
            value=action(ctx)
            result['same_boot_retries'][name]={'status':'PASS','value':value}
        except Exception as error:
            result['same_boot_retries'][name]={'status':'FAILED','error':repr(error)}
            snapshot(ctx)
            result['same_boot_retries'][name]['continuation']=b.recover(ctx,name)
        ctx.log({'same_boot_retry':name,**result['same_boot_retries'][name]})
    return result

def hats_settled(ctx):
    since=b.fixture(ctx,'kirby-hats.json',fresh=True)
    b.cycle(ctx,0,6,'retry-kirby')
    # Original second press was refused during speculative helper construction.
    # Wait for each configured helper to be ready before replaying the hat chord.
    for kind in (0,1,2,3,7):
        b.wait(ctx,since,rf'\[CharacterSources\] ready slot=\d+ kind=0x{kind:x}\b',15)
    return hats(ctx)
