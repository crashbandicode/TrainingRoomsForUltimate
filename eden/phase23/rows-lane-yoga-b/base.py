"""Phase1 project actions; the canonical shared runner owns input and receipts."""
import copy
import hashlib
import importlib.util
import json
import re
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent

def load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / (name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

def tools(ctx):
    g = load('rc_roster_segment')
    strict = load('strict_predicates')
    p = Path(ctx.instance['diag_log'])
    def read():
        lines = []
        for path in (p.with_name('practice-diagnostics.previous.log'), p):
            if path.exists(): lines.extend(path.read_text(errors='replace').splitlines())
        return lines
    class Session:
        def clock(self): return time.monotonic()
        def sleep(self, seconds): ctx.check_stop(); time.sleep(seconds)
        def press(self, chord, hold):
            names = {('zl,right',200):'p1-next-frames', ('zl,left',200):'cpu-next-frames',
                     ('zl,right',40):'lr10-p1-short', ('b,lstick',200):'hat-next'}
            ctx.play(names[(chord,hold)])
    g._visual = lambda _s,_review,name,expected: capture(ctx,name,expected)
    return g, strict, Session(), read

def capture(ctx, name, expected):
    path = ctx.shot(name)
    ctx.log({'media':str(path),'expectation':expected,'review':'pending'})
    return str(path)

def wait(ctx, since, pattern, cap=20):
    g,_,s,read=tools(ctx)
    return g._wait(s,read,since,lambda line:re.search(pattern,line),pattern,cap)

def mark(ctx):
    g,_,_,read=tools(ctx)
    return g.mark(read)

def cycle(ctx, role, target, name):
    g,strict,s,read=tools(ctx)
    return g.cycle(s,read,strict,None,role,target,name)

def state(ctx):
    path=ctx.runner.out/'project-state.json'
    return json.loads(path.read_text()) if path.exists() else {'fixture':'ordinary-return.json','scene_since':0}

def save_state(ctx, data):
    (ctx.runner.out/'project-state.json').write_text(json.dumps(data,indent=2)+'\n')

def reload_settings(ctx, patch, label):
    # Merge onto the complete original input, retaining defaults/unknowns/preferences.
    baseline=json.loads((HERE/'baseline.json').read_text())
    baseline['menu'].update(patch.get('menu',{}))
    for key,value in patch.items():
        if key!='menu': baseline[key]=value
    since=mark(ctx)
    root=Path(ctx.instance['settings']).parent
    temp=root/'diag-settings-next.tmp'
    temp.write_text(json.dumps(baseline)+'\n')
    temp.replace(root/'diag-settings-next.json')
    ack=wait(ctx,since,r'diag_settings_reload .*issues=0\b',15)
    (ctx.row_dir/(label+'-settings.json')).write_text(json.dumps(baseline,indent=2)+'\n')
    ctx.log({'settings_publication':label,'ack':ack})
    return since

def quit_css(ctx):
    ctx.gate('training-hud-unpaused')
    ctx.press('plus',120)
    ctx.gate('pause');ctx.gate('native-pause')
    ctx.play('focus-native-quit-frames');ctx.gate('quit-focus')
    ctx.play('confirm-native-quit-frames');ctx.gate('ready')

def reenter(ctx):
    load('nav_recover').to_training(ctx)
    since=mark(ctx)
    st=state(ctx);st['scene_since']=since;save_state(ctx,st)
    return since

def fixture(ctx, name, fresh=False):
    st=state(ctx)
    if st['fixture']==name and not fresh: return st['scene_since']
    ctx.gate('training-hud-unpaused')
    patch=json.loads((HERE/'rc-settings'/name).read_text())
    since=reload_settings(ctx,patch,name.replace('.json',''))
    wait(ctx,since,r'\[CyclePreload\] applied',15)
    quit_css(ctx)
    since=reenter(ctx)
    st=state(ctx);st['fixture']=name;save_state(ctx,st)
    return since

def prefetch(ctx, since, role, kinds):
    text=', '.join(format(k,'x') for k in kinds)
    return wait(ctx,since,rf'\[GroupSwap\] prefetched owner={role} group=\d+ kinds=\[{re.escape(text)}\]',40)

def notice(ctx):
    ctx.gate('training-hud-unpaused')
    g,_,_,read=tools(ctx)
    lines=g._new(read,0)
    found=[x for x in lines if '[PreloadStatus]' in x or 'notice_draw' in x]
    bad=[x for x in found if 'state=Paused' in x and re.search(r'\bpending=0\b',x)]
    if bad: raise RuntimeError('False Paused with pending=0: '+bad[-1])
    capture(ctx,'notice-initial','Actual loading roles/ready total; log text alone does not prove all three roles')
    return {'observed':found,'open':'P1/CPU/both loading and exact ready total require full-batch review'}

def mii(ctx):
    ctx.gate('training-hud-unpaused')
    g,strict,s,read=tools(ctx)
    result=g.mii_gunner(s,read,strict,None,state(ctx)['scene_since'],lambda:bool(json.loads((HERE/'baseline.json').read_text())['mii_preferences']['cpu'][2]))
    result['native_install']=load('navigation').verify_cpu_install_trace(ctx)
    result['return_dk']=cycle(ctx,1,1,'mii-dk')
    result['return_lucas']=cycle(ctx,1,0x28,'mii-lucas')
    result['open']='Pointed complete native Mii identity remains unverified; scalar/class/install is not complete payload proof'
    return result

def r3_settings(ctx, enabled):
    ctx.gate('training-hud-unpaused')
    reload_settings(ctx,{'menu':{'hitbox_vis':[1,0] if enabled else [0,1], 'frame_advantage':[1,0] if enabled else [0,1]}},'r3-on' if enabled else 'r3-off')
    ctx.gate('training-hud-unpaused')
    return {'enabled':enabled}

def recorded_action(ctx, name):
    path=ctx.row_dir/(name+'.gif')
    ctx.call('recording.start',{'path':str(path),'duration_ms':8000,'width':1280,'fps':10})
    try:
        ctx.play(name)
    finally:
        deadline=time.monotonic()+15
        while time.monotonic()<deadline:
            result=ctx.call('recording.status')
            if result.get('error'): raise RuntimeError(str(result))
            if result.get('state') in ('finished','idle','completed') and path.exists(): break
            time.sleep(.25)
        else: raise RuntimeError('Recording did not finalize: '+name)
        ctx.log({'active_window_media':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'recording':result})
    return str(path)

def r3_action(ctx, name):
    ctx.gate('training-hud-unpaused')
    since=mark(ctx)
    media=recorded_action(ctx,name)
    predicates=load('slim_actions')
    g,_,s,read=tools(ctx)
    result={'media':media,'review':'pending active-frame review'}
    if name=='tr2-jab-x4':
        result['pass']=g._wait(s,read,since,predicates.attack_pass,'spawned attack geometry',10)
        result['stats']=g._wait(s,read,since,predicates.attack_stats,'attack stats zero heap/hurt',15)
    elif name=='tr2-grab':
        result['catch']=g._wait(s,read,since,predicates.catch_command,'fresh catch spheres',5)
    else: result['open']='Native shield positive requires actual GIF review; input success is not shield acceptance'
    return result

def r3_pause(ctx):
    ctx.gate('training-hud-unpaused');since=mark(ctx)
    ctx.press('plus',120);ctx.gate('native-pause')
    begin=wait(ctx,since,r'pause_owner begin ',10)
    capture(ctx,'r3-pause','Native settled pause, CPUs=1, no visualizer leak')
    since=mark(ctx);ctx.play('pause-close');ctx.gate('training-hud-unpaused')
    return {'begin':begin,'end':wait(ctx,since,r'pause_owner end .*owners=0x00',10),
            'resume':wait(ctx,since,r'pause_owner resume_checked .*leaked=0',10),
            'open':'Independent pause producer and frame-advantage positive/retirement still require specific evidence'}

def pilot(ctx):
    ctx.gate('training-hud-unpaused')
    root=Path(ctx.instance['settings']).parent.parent
    events=(root/'TrainingRooms/events.log').read_text(errors='replace')
    pilot=(root/'ssbu-cpu-pilot/events.log').read_text(errors='replace')
    (ctx.row_dir/'pilot-events.log').write_text(pilot)
    (ctx.row_dir/'trainingrooms-events.log').write_text(events)
    if 'p1_cpu=not_installed reason=standalone_pilot_present' not in events or 'result=installed' not in pilot or 'menu=registered' not in pilot:
        raise RuntimeError('Pilot actual standdown/install/registration prerequisite missing')
    # Delivered exact text-shape gates refuse different resolutions; no guessed overlay close.
    from PIL import Image
    shot=capture(ctx,'pilot-start','Current native Training/P1 manual state before finite page recording')
    size=list(Image.open(shot).size)
    cfg=json.loads((HERE/'pilot-markers.json').read_text())
    if size!=cfg['size']:
        raise RuntimeError(f'Pilot route deferred: delivered page/HUD markers require {cfg["size"]}, current {size}; next state remains gated Training')
    raise RuntimeError('Pilot standdown/install observed; selected-row arm/close/release route remains unqualified. Defer without toggling unknown ownership; next state gated Training')

def kirby(ctx):
    fixture(ctx,'kirby-hats.json',fresh=True)
    results=[]
    for target in (6,0x0b,0x2d,0x2c,0x1c,6):
        results.append(cycle(ctx,0,target,'kirby-'+format(target,'x')))
    return results

def hats(ctx):
    # A failed Kirby prefix must not turn a chord on another body into hat proof.
    g,strict,s,read=tools(ctx)
    st=state(ctx)
    lines=g._new(read,st['scene_since'])
    ready=[x for x in lines if '[RosterTxn] ready owner=0 ' in x]
    if not ready or not re.search(r'\btarget=0x6\b',ready[-1]):
        raise RuntimeError('HATS deferred: current Kirby ready prerequisite absent')
    result={'hats':g.hats(s,read,None,(0,1,2,3,7))}
    result['departure']=g.kirby_to_rob(s,read,strict,None,0x0b)
    result['rob']=cycle(ctx,0,0x2d,'rob-after-hats')
    return result

def special(ctx, name):
    since=fixture(ctx,'specials-'+name+'.json',fresh=True)
    prefix=(0x0b,0x1f)
    results=[cycle(ctx,0,k,'special-prefix-'+format(k,'x')) for k in prefix]
    prefetch(ctx,since,0,(0x2e,0x2d))
    g,strict,s,read=tools(ctx)
    results.append(g.cold_group(s,read,strict,None,0,0x2e,'special-ordinary-async'))
    if name=='ordinary': return {'ordinary_async':results}
    for target in (0x2d,0x2c,0x1c): results.append(cycle(ctx,0,target,'special-prefix-'+format(target,'x')))
    kinds={'pt':(0x24,0x25,0x26),'aegis':(0x5a,0x5b),'ic':(0x4b,0x4c)}[name]
    # Native special group prefetch must precede its first construction too.
    prefetch(ctx,since,0,kinds)
    result=cycle(ctx,0,kinds[0],'special-'+name)
    build=wait(ctx,since,r'\[GroupSwap\] build owner=0 .*mode=sync reason=',15)
    return {'ordinary':results,'special':result,'sync':build,
            'open':'Initial native group observed; every form/partner action needs checkpoint/behavior review'}

def cold_prefix(ctx):
    since=fixture(ctx,'ordinary-return.json',fresh=True)
    cycle(ctx,0,0x0b,'pending-falcon');cycle(ctx,0,0x1f,'pending-darkpit')
    line=prefetch(ctx,since,0,(0x2e,0x2d))
    ctx.log({'exact_pending_target_prefetched':line})

def busy(ctx):
    cold_prefix(ctx)
    g,strict,s,read=tools(ctx)
    result=g.busy(s,read,strict,None,0,0x2e,0x2d,'busy')
    if result.get('exercised') is False: cycle(ctx,0,0x2d,'busy-later')
    return result

def teardown(ctx,index):
    cold_prefix(ctx)
    g,strict,s,read=tools(ctx)
    def immediate_quit(_s):
        ctx.press('plus',40);ctx.gate('pause');ctx.gate('native-pause')
        ctx.play('focus-native-quit-frames');ctx.gate('quit-focus')
        ctx.play('confirm-native-quit-frames');ctx.gate('ready')
    return g.interrupted_scene(s,read,strict,None,0,0x2e,immediate_quit,lambda _s:reenter(ctx),0x0b,index)

def trio(ctx,index):
    fixture(ctx,'ordinary-return.json',fresh=True)
    cycle(ctx,0,0x0b,'trio-falcon');cycle(ctx,1,0x4a,'trio-gunner')
    result=load('navigation').quit_warm_training(ctx)
    result['reentry_since']=reenter(ctx)
    result['continuation']=cycle(ctx,0,0x0b,'trio-continuation')
    return result

def recover(ctx, failed):
    ctx.call('input.reset')
    out=load('nav_recover').to_training(ctx)
    ctx.log({'recovered_after':failed,'nav_recover':out})
    return out

def final(ctx):
    result=notice(ctx)
    result['trace_counters']=ctx.call('trace.counters')
    return result
