"""Reproduce current-row, current-bank Mii tuple and HATS lookup observations."""
import json,re,struct
from pathlib import Path
D=Path(__file__).resolve().parent
def load(p):return json.loads(p.read_text())
def decode(raw):
    return {'index':struct.unpack_from('<H',raw,0x1a8)[0],
            'tokens':list(struct.unpack_from('<4H',raw,0x1aa)),
            'specials':list(raw[0x1b2:0x1b6])}
def main():
    trace=[json.loads(x) for x in (D/'yoga.trace.jsonl').read_text().splitlines()]
    receipt=load(D/'out/RC-MII.json');t=load(D/'out/RC-MII/tuple-observation.json')
    canonical=bytes.fromhex(next(x for x in t['canonical'] if x))
    alias=int(re.search(r'alias=(0x[0-9a-f]+)',t['plan'])[1],0)
    expected={'specials':list(canonical[0x59:0x5d]),'colour':canonical[0x5d],'voice':canonical[0x5e],
              'outfit_hash40':int.from_bytes(canonical[0x5f:0x64],'little'),
              'hat_hash40':int.from_bytes(canonical[0x64:0x69],'little')}
    window=[x for x in trace if receipt['started']*1e9<=int(x.get('wall_time_ns',0))<=receipt['ended']*1e9]
    reads=[]
    for x in window:
        if x.get('point_id')!=t['native_setup_point']['id']:continue
        caps={c['expression']:c for c in x['captures'] if not c['error']}
        resource=caps[t['setup_resource_expression']+'+0x58']
        raw=bytes.fromhex(resource['hex'])
        if int(resource['address'],0)!=alias+0x58 or int.from_bytes(raw[:4],'little')!=canonical[0]:continue
        e=decode(bytes.fromhex(caps[t['cpu_row']]['hex']))
        fe=list(struct.unpack('<5H',bytes.fromhex(caps['x2']['hex'])))
        record=bytes.fromhex(caps[t['setup_record_expression']]['hex'])[0x68:0x68+0x58]
        masked=bytes(0x26)+record[0x26:]
        owner_index=struct.unpack_from('<H',raw,0x10)[0]
        owner=list(struct.unpack_from('<4I',raw,0x14))
        getters=[]
        for point,key,hint,tag in [('3','outfit_hash40',e['tokens'][0],0x13),('4','hat_hash40',e['tokens'][1],0x5a)]:
            wanted=(tag<<56)|(hint<<40)|expected[key]
            matching=[{'sequence':y['sequence'],'frame':y['frame'],'value':y['registers']['x0']} for y in window
                      if y.get('point_id')==point and y.get('event')=='return' and int(y['registers']['x0'],0)==wanted]
            getters.append({'point':point,'expected':hex(wanted),'matching_returns':matching})
        reads.append({'sequence':x['sequence'],'frame':x['frame'],'resource':resource['address'],'e':e,'fe':fe,
                      'owner_index':owner_index,'owner_tokens':owner,'masked_charinfo_matches':masked==canonical[1:0x59],
                      'specials_match':e['specials']==expected['specials'],
                      'colour_voice_match':e['tokens'][2:]==[expected['colour'],expected['voice']],
                      'mirrors_match':fe==[e['index']]+e['tokens'] and owner_index==e['index'] and owner==e['tokens'],
                      'getters':getters})
    hats=[x for x in trace if x.get('point_id')=='7']
    pending=[]
    for path in sorted((D/'out').glob('RC-TEARDOWN*/pending-window.json')):
        data=load(path);lines=data['lines'];pick=lambda needle:next((s for s in lines if needle in s),None)
        build=pick('[GroupSwap] build owner=0');queued=pick('[GroupSwap] bodies queued owner=0')
        done=pick('[GroupSwap] built owner=0');finalize=pick('[GroupSwap] teardown owner=0 reason=finalize')
        pause=pick('pause_owner begin');drain=pick('[GroupSwap] construction drained owner=0')
        delta=lambda a,b:int(b.split()[0])-int(a.split()[0]) if a and b else None
        pending.append({'path':str(path.relative_to(D)),'prefetched':data['prefetched'],
                        'build':build,'queue':queued,'built':done,'finalize':finalize,'drain':drain,
                        'build_to_pause_ms':delta(build,pause),'build_to_built_ms':delta(build,done),
                        'build_to_finalize_ms':delta(build,finalize),'queue_to_built_ms':delta(queued,done)})
    result={'mii':{'expected':expected,'bank':hex(alias),'reads':reads,'ready':t['ready']['done'],
                   'scope':'CPU Gunner bank setup; native D mirrors and chara hash40 not independently captured'},
            'hats':{'lookup':hats,'cleanup_counter':load(D/'out/RC-HATS/hats-trace.json')['trace_counters']['points'][-1]},
            'teardown':pending}
    (D/'trace-review.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
if __name__=='__main__':main()
