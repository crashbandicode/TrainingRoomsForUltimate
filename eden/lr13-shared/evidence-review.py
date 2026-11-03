"""Review actual current bank/time-qualified traces; no aggregate-count inference."""
import hashlib,json,re,struct,sys
from pathlib import Path
d=Path(sys.argv[1]) if len(sys.argv)>1 else Path('/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work/eden/lr13-shared')
out=d/'out' if (d/'out').is_dir() else d
def load(p):return json.loads(p.read_text())
def seq(x):return int(re.search(r'#(\d+)',x)[1])
review={}
receipt=out/'RC-MII.json'
if receipt.exists():
    r=load(receipt);t=load(out/'RC-MII/tuple-observation.json') if (out/'RC-MII/tuple-observation.json').exists() else None
    rows=[json.loads(x) for x in (d/'yoga.trace.jsonl').read_text().splitlines()]
    if t:
        start=int(r['started']*1e9);end=int(r['ended']*1e9)
        alias=int(re.search(r'alias=(0x[0-9a-f]+)',t['plan'])[1],0)
        canonical=bytes.fromhex(t['canonical'])
        expected={'class':canonical[0],'specials':list(canonical[0x59:0x5d]),'colour':canonical[0x5d],'voice':canonical[0x5e],
            'outfit_hash40':int.from_bytes(canonical[0x5f:0x64],'little'),'hat_hash40':int.from_bytes(canonical[0x64:0x69],'little')}
        ids=[str(x.get('id',x.get('point_id'))) for x in t['tuple_points']]
        native=[];getters=[]
        window=[x for x in rows if start<=int(x.get('wall_time_ns',0))<=end]
        for x in window:
            if x.get('event')=='entry' and x.get('point_id')==ids[2] and int(x['registers']['x0'],0)==alias:
                caps={c['expression']:c for c in x.get('captures',[]) if not c.get('error')}
                c=caps.get(t['cpu_row'])
                if not c:continue
                raw=bytes.fromhex(c.get('hex',c.get('value','')).removeprefix('0x'))
                if len(raw)!=0x260:continue
                tokens=list(struct.unpack_from('<4H',raw,0x1aa))
                specials=list(raw[0x1b2:0x1b6])
                native.append({'sequence':x['sequence'],'frame':x['frame'],'point_id':x['point_id'],'tokens':tokens,'specials':specials,
                    'specials_match':specials==expected['specials'],'colour_voice_match':tokens[2:]==[expected['colour'],expected['voice']]})
        for i,key in enumerate(('outfit_hash40','hat_hash40')):
            entries=[x for x in window if x.get('point_id')==ids[i] and x.get('event')=='entry']
            returns=[x for x in window if x.get('point_id')==ids[i] and x.get('event')=='return']
            getters.append({'field':key,'point_id':ids[i],'entries':len(entries),'returns':len(returns),
                'matching_returns':[{'sequence':x['sequence'],'frame':x['frame'],'thread':x['thread_id'],'value':x['registers']['x0']} for x in returns if int(x['registers']['x0'],0)&((1<<40)-1)==expected[key]],
                'pairing_limit':'Provider return matching guarantees thread/LR/SP; correlate actual entry identity and observation scope separately'})
        review['mii']={'expected':expected,'current_bank':hex(alias),'window_ns':[start,end],'native_tuple_reads':native,'getters':getters,
            'masked_charinfo_reads':t['matching_record_reads'],'bank_ready':t['ready']['done'],
            'tuple_components_match':bool(native) and all(x['specials_match'] and x['colour_voice_match'] for x in native) and all(x['matching_returns'] for x in getters)}
    else:review['mii']={'status':r['status'],'error':r.get('error'),'complete_tuple':'unverified'}
review['teardown']=[]
for path in out.glob('RC-TEARDOWN-*/pending-window.json'):
    r=load(path)
    def ms(line):return int(line.split()[0])
    review['teardown'].append({'path':str(path.relative_to(d)),**r,
        'queued_to_finalize_ms':[ms(r['finalize'])-ms(x) for x in r['queued']],
        'queued_to_built_ms':[ms(y)-ms(x) for x in r['queued'] for y in r['built']],
        'pending_drain_proven':bool(r['queued'] and r['drains']) and all(seq(x)<seq(r['finalize']) for x in r['queued'])})
for folder in out.glob('RC-TEARDOWN-*'):
    if not folder.is_dir() or (folder/'pending-window.json').exists():continue
    lines=[]
    for name in ('practice-diagnostics.previous.log','practice-diagnostics.log'):
        file=folder/name
        if file.exists():lines+=file.read_text().splitlines()
    lines=sorted({seq(x):x for x in lines if re.search(r'#\d+',x)}.values(),key=seq)
    queued=[x for x in lines if '[GroupSwap] bodies queued owner=0' in x]
    if not queued:continue
    q=queued[-1];group=re.search(r'group=(\d+)',q)[1]
    after=[x for x in lines if seq(x)>seq(q)]
    built=next((x for x in after if f'[GroupSwap] built owner=0 group={group} ' in x),None)
    paused=next((x for x in after if 'pause_owner begin' in x),None)
    clean=next((x for x in after if '[GroupSwap] teardown owner=0 reason=finalize' in x),None)
    ms=lambda line:int(line.split()[0])
    r=load(out/(folder.name+'.json'))
    review['teardown'].append({'row':folder.name,'status':r['status'],'error':r.get('error'),'queued':q,'built':built,'pause':paused,'finalize_in_row':clean,
        'queued_to_built_ms':ms(built)-ms(q) if built else None,'queued_to_pause_ms':ms(paused)-ms(q) if paused else None,'pending_drain_proven':False})
(d/'review.json').write_text(json.dumps(review,indent=2)+'\n')
print(json.dumps(review,indent=2))
