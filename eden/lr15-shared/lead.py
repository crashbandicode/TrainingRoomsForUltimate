"""RC9 project probes; canonical eden_test owns admission, input and cleanup."""
import hashlib,importlib.util,json,re,time
from pathlib import Path
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('lr15_steps',HERE/'steps.py')
s=importlib.util.module_from_spec(spec);spec.loader.exec_module(s)
b=s.b;p=s.p
CAT=json.loads((HERE/'catalog.json').read_text())
def choice(name):return next(x for x in CAT if x['name']==name)
def ranks(document,key):
    values=document['menu'][key]
    if values[0]:raise RuntimeError('Explicit ranked fixture required')
    return [CAT[i-1] for i in sorted(range(1,len(values)),key=lambda i:values[i]) if values[i]]
def put(document,key,names):
    template=document['menu'].get(key)
    if template is None:template=json.loads((HERE/'baseline.json').read_text())['menu'][key]
    values=[0]*len(template)
    for rank,name in enumerate(names,1):values[choice(name)['index']]=rank
    document['menu'][key]=values
def next_cycle(ctx,role,name):
    current=json.loads(Path(ctx.instance['settings']).read_text())
    item=next(x for x in ranks(current,'player_roster' if role==0 else 'cpu_roster') if x['name']==name)
    return s.cycle(ctx,role,item['kind'],name.replace(' ','-'))
def dispatch(ctx,action,**args):
    ctx.log({'project_sha256':{x.name:hashlib.sha256(x.read_bytes()).hexdigest() for x in HERE.glob('*.py')},'action':action,'args':args})
    try:return globals()[action](ctx,**args)
    finally:p.snapshot(ctx)
def boot(ctx):
    source=Path(ctx.instance['events_log']);text=source.read_text()
    (ctx.row_dir/'events.log').write_text(text)
    lines=[x for x in text.splitlines() if 'installed' in x or 'alloc' in x]
    if not any('heap_allocations=0' in x for x in lines):raise RuntimeError('Install heap allocation zero absent')
    return {'install':lines,'M5q_numeric':'Unavailable: retained baseline unmetered'}
def final_alloc(ctx):
    start=b.mark(ctx)
    line=b.wait(ctx,start,r'\[TrainingRooms\] hook_stats allocations=0 delta=0 ',20)
    heap=b.wait(ctx,start,r'\[TrainingRooms\] (?:cpu|state)_hook_counts .*heap_allocations=0',20)
    return {'post_rows_allocations':line,'post_rows_heap':heap,'trace_counters':ctx.call('trace.counters')}
def arm_cleanup(ctx):
    points=ctx.call('trace.list')['points']
    if any(x['target']=='main+0x1fca0b0' and x['state']=='armed' for x in points):raise RuntimeError('Removed boolean lookup trace unexpectedly armed')
    live=[x for x in points if x['target']=='main+0xb96770' and x['state']=='armed']
    if live:return live[0]
    return ctx.call('trace.arm',{'target':'main+0xb96770','registers':['x0','x1','x30','pc'],
        'captures':[{'expression':'x0','type':'bytes','size':64},{'expression':'x0+0x8','type':'u32'},
                    {'expression':'x0+0xc','type':'u32'},{'expression':'x0+0x10','type':'u64'},
                    {'expression':'x0+0x20','type':'u64'}],
        'on_return':True,'max_hits':128,'rate_limit':1000})
def cleanup_pair(ctx,point,started,body,role):
    deadline=time.monotonic()+8
    while time.monotonic()<deadline:
        ctx.check_stop();rows=[]
        for line in Path(ctx.instance['trace_jsonl']).read_text().splitlines():
            try:row=json.loads(line)
            except ValueError:continue
            if row.get('point_id')==str(point['id']) and int(row.get('wall_time_ns',0))>=started:rows.append(row)
        for entry in rows:
            if entry.get('event')!='entry':continue
            caps={c['expression']:c for c in entry.get('captures',[]) if not c.get('error')}
            if any(int(caps.get(expr,{}).get('value','-1'),0)!=value for expr,value in [('x0+0x8',int(body,16)),('x0+0xc',6),('x0+0x10',role)]):continue
            for ret in rows:
                if ret.get('event')=='return' and ret.get('thread_id')==entry.get('thread_id') and int(ret['sequence'])>int(entry['sequence']) and ret['registers']['pc']==entry['registers']['x30']:
                    return {'entry':entry,'return':ret}
        time.sleep(.15)
    raise RuntimeError('No paired native cleanup entry/return for current Kirby body/role')
def confirm(ctx,role,item,replacement=False):
    point=arm_cleanup(ctx);started=time.time_ns()
    kind=item['kind'];start=b.mark(ctx)
    ctx.play('hat-next' if role==0 else 'cpu-hat-next')
    # The chord schedules preparation. A ready-before-chord gate would deadlock
    # a target not yet admitted into the finite helper cache.
    request=b.wait(ctx,start,rf'\[KirbyHats\] request role={role} kind=0x{kind:x}\b',40)
    body=re.search(r'body=(0x[0-9a-f]+)',request)[1]
    done=b.wait(ctx,start,rf'\[KirbyHats\] confirmed role={role} kind=0x{kind:x} body={body}\b',15)
    result={'request':request,'confirmed':done,'body':body,'kind':kind}
    if replacement:
        result['cleanup']=b.wait(ctx,start,rf'\[KirbyHats\] native cleanup body={body} .*groups_after=Some\(0\)',5)
        result['settled']=b.wait(ctx,start,rf'\[KirbyHats\] cleanup settled body={body} released=[1-9]\d*',5)
        seq=lambda line:int(re.search(r'#(\d+)',line)[1])
        if not seq(result['cleanup'])<seq(result['settled'])<seq(request)<seq(done):raise RuntimeError('Native cleanup / old lease release / replacement ordering violated')
        result['native_pair']=cleanup_pair(ctx,point,started,body,role)
        if int(result['native_pair']['entry']['registers']['x1'],0)!=0:raise RuntimeError('Replacement used unexpected native cleanup variant')
    result['shot']=str(ctx.shot('confirmed-'+str(role)+'-'+str(kind)))
    return result
def hats(ctx,role=0,resume=False):
    if resume:
        result=json.loads((ctx.row_dir/'hats.json').read_text())
        current=json.loads(Path(ctx.instance['settings']).read_text());choices=ranks(current,'cpu_kirby_hats')
        try:
            result['helper_ready_before_retry']=b.wait(ctx,result['scene_since'],r'\[CharacterSources\] ready slot=\d+ kind=0x0\b',8)
            for n,item in enumerate(choices[:5]+choices[:1]):
                result['attempts'].append(confirm(ctx,role,item,replacement=n>0))
            return result
        finally:(ctx.row_dir/'hats.json').write_text(json.dumps(result,indent=2)+'\n')
    patch=json.loads((HERE/'rc-settings/kirby-hats.json').read_text())
    put(patch,'player_roster' if role==0 else 'cpu_roster',['Captain Falcon','Kirby','R.O.B.'])
    if role==1:
        put(patch,'cpu_kirby_hats',['Mario','Donkey Kong','Link','Samus','Fox'])
    filename='hats-role-'+str(role)+'.json';(HERE/'rc-settings'/filename).write_text(json.dumps(patch)+'\n')
    result={'role':role,'cleanup_trace':arm_cleanup(ctx),'attempts':[]}
    try:
        result['scene_since']=b.fixture(ctx,filename,fresh=True)
        result['nonfirst_prefix']=next_cycle(ctx,role,'Captain Falcon')
        result['kirby']=next_cycle(ctx,role,'Kirby')
        current=json.loads(Path(ctx.instance['settings']).read_text())
        choices=ranks(current,'player_kirby_hats' if role==0 else 'cpu_kirby_hats')
        if len(choices)<5:raise RuntimeError('Five selected hats required')
        result['choices']=choices
        for n,item in enumerate(choices[:5]+choices[:1]):
            result['attempts'].append(confirm(ctx,role,item,replacement=n>0))
            (ctx.row_dir/'hats.json').write_text(json.dumps(result,indent=2)+'\n')
        result['reuse_scope']='Sixth confirmation repeats first kind; exact helper identity/reuse requires trace/log review'
        return result
    finally:
        result['trace_counters']=ctx.call('trace.counters')
        (ctx.row_dir/'hats.json').write_text(json.dumps(result,indent=2)+'\n')
def kirby_rob(ctx):
    result={'cycle':s.kirby(ctx)}
    current=json.loads(Path(ctx.instance['settings']).read_text())
    result['copy']=confirm(ctx,0,ranks(current,'player_kirby_hats')[0])
    result['copied_departure']=s.departure(ctx)
    return result
def copy_loss(ctx):
    previous=json.loads((ctx.runner.out/'RC-HATS-P1/hats.json').read_text())
    last=previous['attempts'][-1]
    point=arm_cleanup(ctx);started=time.time_ns();since=b.mark(ctx)
    ctx.gate('training-hud-unpaused')
    ctx.play('kirby-taunt-loss')
    result={'previous_copy':last,'point':point,'started_ns':started}
    try:
        result['native_loss_trace']=cleanup_pair(ctx,point,started,last['body'],0)
        result['taunt_shot']=str(ctx.shot('native-copy-loss'))
        observed=p.lines(ctx,since)
        if any('[KirbyHats] native cleanup' in x for x in observed):raise RuntimeError('Cleanup originated in plugin replacement during taunt window')
        current=json.loads(Path(ctx.instance['settings']).read_text())
        result['reacquired']=confirm(ctx,0,ranks(current,'player_kirby_hats')[0])
        result['scope']='Native cleanup entry/return after taunt, followed by new confirmed copy; exact body/caller and visual loss require review'
        return result
    finally:(ctx.row_dir/'copy-loss.json').write_text(json.dumps(result,indent=2)+'\n')
def recover(ctx,failed):return b.recover(ctx,failed)
def teardown(ctx):
    patch=json.loads((HERE/'rc-settings/ordinary-return.json').read_text())
    put(patch,'player_roster',['Captain Falcon','Dark Pit','Donkey Kong','Sora','Kazuya','Steve'])
    patch['menu']['cycle_preload_bank_members']=[0,0,1]
    (HERE/'rc-settings/slow-36.json').write_text(json.dumps(patch)+'\n')
    since=b.fixture(ctx,'slow-36.json',fresh=True)
    for name in ['Captain Falcon','Dark Pit','Donkey Kong']:next_cycle(ctx,0,name)
    plan,match=p.membership(ctx,since,0,choice('Sora')['kind'])
    kinds=tuple(int(x.strip(),16) for x in match[2].split(','))
    wanted=tuple(choice(name)['kind'] for name in ['Sora','Kazuya','Steve'])
    if kinds!=wanted:raise RuntimeError('Requested slow group differs from actual current plan: '+plan)
    prefetched=b.prefetch(ctx,since,0,kinds)
    start=b.mark(ctx);result={'plan':plan,'prefetched':prefetched,'route_frames':36}
    try:
        ctx.play('lr14-request-quit-frames');ctx.gate('ready')
        result['lines']=p.lines(ctx,start)
        result['fresh']=b.reenter(ctx)
        result['continuation']=next_cycle(ctx,0,'Captain Falcon')
        if not any('construction drained owner=0' in x for x in result['lines']):raise RuntimeError('Single slow-group/36-frame route missed pending drain; timeline retained')
        return result
    finally:
        result.setdefault('lines',p.lines(ctx,start))
        (ctx.row_dir/'pending-window.json').write_text(json.dumps(result,indent=2)+'\n')
