"""Offline RC9 receipt/trace review. No acceptance inferred from PASS totals."""
import argparse,hashlib,json,re
from pathlib import Path
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def seq(line):return int(re.search(r'#(\d+)',line)[1])
def stamp(line):return int(line.split()[0])
def pair_check(pair,body,role):
    e,r=pair['entry'],pair['return']
    assert e['event']=='entry' and r['event']=='return'
    assert all(e[k]==r[k] for k in ('point_id','thread_id','process_id'))
    assert int(e['sequence'])<int(r['sequence']) and int(e['wall_time_ns'])<=int(r['wall_time_ns'])
    assert e['registers']['x30']==r['registers']['pc']
    assert 'main+0xb96770' in e['locations'].values()
    caps={c['expression']:c for c in e['captures'] if not c.get('error')}
    for expr,value in [('x0+0x8',body),('x0+0xc',6),('x0+0x10',role)]:
        assert int(caps[expr]['value'],0)==value
    raw=bytes.fromhex(caps['x0']['hex'])
    assert int.from_bytes(raw[8:12],'little')==body and int.from_bytes(raw[12:16],'little')==6
    assert int.from_bytes(raw[16:24],'little')==role
    accessor=int(caps['x0+0x20']['value'],0)
    assert accessor>0x1000 and int.from_bytes(raw[32:40],'little')==accessor
    return {'sequences':[e['sequence'],r['sequence']],'body':body,'role':role,'kind':6,
            'fighter':e['registers']['x0'],'accessor':hex(accessor),'caller':e['locations'].get(e['registers']['x30']),
            'variant':e['registers']['x1']}
def teardown(lines):
    def one(text):return next(x for x in lines if text in x)
    q=one('bodies queued owner=0');f=one('[Destroy] plan owner=0 reason=finalize')
    d=one('construction drained owner=0');t=one('[GroupSwap] teardown owner=0');l=one('[Ledger] reconcile owner=0 phase=finalize')
    assert seq(q)<seq(f)<seq(d)<seq(t)<seq(l)
    assert 'waited=1' in d and 'alias_ok=true' in d
    assert 'map_after=0' in t and 'skipped=0' in t and 'alias_ok=true' in t
    assert 'map=0 task_pending=0 predicted=[] actual=[] residual=[] missing=[]' in l
    assert not any('[GroupSwap] built owner=0' in x and seq(q)<seq(x)<seq(d) for x in lines)
    return {'sequences':[seq(x) for x in (q,f,d,t,l)],'queue_to_finalize_ms':stamp(f)-stamp(q),
            'queue_to_drain_ms':stamp(d)-stamp(q),'drain_wait_us':int(re.search(r' us=(\d+)',d)[1]),'interruptions':1}
def receipt_binding(root,receipt):
    path=root/'out/manifests'/(receipt['manifest_sha256']+'.json')
    assert sha(path)==receipt['manifest_sha256'], 'manifest bytes differ'
    plan=json.loads(path.read_text());assert any(x['id']==receipt['id'] for x in plan['rows']), 'row absent from bound plan'
    return receipt['manifest_sha256']
def summarize(root):
    load=lambda p:json.loads((root/p).read_text())
    plan=load('rows/rows.json');receipts=[load('out/'+r['id']+'.json') for r in plan['rows']]
    all_receipts=[(p,json.loads(p.read_text())) for p in (root/'out').glob('*.json') if 'status' in json.loads(p.read_text()) and 'id' in json.loads(p.read_text())]
    bound=[{'path':str(p.relative_to(root)),'status':r['status'],'manifest':receipt_binding(root,r)} for p,r in all_receipts]
    actions=[json.loads(x) for x in (root/'out/actions.jsonl').read_text().splitlines()]
    hooks=[]
    for a in actions:
        if 'project_sha256' not in a:continue
        matches=[]
        for folder in [root/'rows']+sorted((root/'attempts').glob('*/rows')):
            if all((folder/k).is_file() and sha(folder/k)==v for k,v in a['project_sha256'].items()):matches.append(str(folder.relative_to(root)))
        assert matches, 'unretained executed hook revision'
        hooks.append({'action':a['action'],'lead_sha256':a['project_sha256']['lead.py'],'retained_in':matches})
    trace=[json.loads(x) for x in (root/'state/run-yoga/yoga.trace.jsonl').read_text().splitlines()]
    byseq={x['sequence']:x for x in trace}
    hats=load('out/RC-HATS-P1/hats.json');pairs=[];retention=[]
    assert [a['kind'] for a in hats['attempts']]==[0,1,2,3,7,0]
    lines=(root/'out/RC-HATS-P1/practice-diagnostics.log').read_text().splitlines()
    for old,new in zip(hats['attempts'],hats['attempts'][1:]):
        assert seq(old['confirmed'])<seq(new['cleanup'])<seq(new['settled'])<seq(new['request'])<seq(new['confirmed'])
        assert 'released=1' in new['settled'] and 'groups_after=Some(0)' in new['cleanup']
        pair=new['native_pair'];checked=pair_check(pair,int(new['body'],16),0)
        assert checked['variant']=='0x0'
        for entry in pair.values():assert byseq[entry['sequence']]==entry, 'trace receipt differs from full trace'
        slot=int(re.search(r'source_slot=(\d+)',old['request'])[1])
        installs=[x for x in lines if f'[CharacterSources] install slot={slot} ' in x and seq(x)>seq(old['request'])]
        assert not any(seq(x)<seq(new['settled']) for x in installs), 'old source replaced before cleanup settled'
        retention.append({'old_kind':old['kind'],'slot':slot,'cleanup':seq(new['cleanup']),'release':seq(new['settled']),
                          'next_slot_install':seq(installs[0]) if installs else None})
        pairs.append(checked)
    loss=load('out/RC-HATS-copy-loss/copy-loss.json');loss_pair=pair_check(loss['native_loss_trace'],1,0)
    assert loss_pair['variant']=='0x1' and loss_pair['caller']=='main+0xb99ef0'
    for entry in loss['native_loss_trace'].values():assert byseq[entry['sequence']]==entry
    assert loss['reacquired']['kind']==0
    counters=load('out/RC-BOOT-postrows.json')['results'][0]['value']['trace_counters']
    assert counters['file_lost']=='0'
    assert all(all(int(p[k])==0 for k in ('dropped','abandoned_returns','return_overflow')) for p in counters['points'])
    assert all(p['target']!='main+0x1fca0b0' for p in counters['points'])
    assert not any(a.get('method')=='trace.arm' and a.get('params',{}).get('target')=='main+0x1fca0b0' for a in actions)
    td=load('out/RC-TEARDOWN-slow36.json')['results'][0]['value'];td_summary=teardown(td['lines'])
    assert 'kinds=[5d, 5c, 58]' in td['plan'] and td['route_frames']==36
    td_summary['continuation_ready']=td['continuation']['done'];td_summary['scope']='fresh scene in the same process; one interruption only'
    console=root/'state/run-yoga/console.log';journal=root/'unit-journal.log'
    hits=lambda p,pattern:[x for x in p.read_text(errors='replace').splitlines() if re.search(pattern,x,re.I)]
    life=load('state/lifecycle.json');props=dict(x.split('=',1) for x in (root/'unit-properties.txt').read_text().splitlines())
    assert hooks, 'no executed hook bindings'
    return {'artifact_identity':load('artifact-binding.json'),'final_manifest_sha256':sha(root/'rows/rows.json'),'final_counts':{status:sum(r['status']==status for r in receipts) for status in ('PASS','FAILED')},
        'retained_failed_archives':len(list((root/'out').glob('*.FAILED.*.json'))),'receipt_bindings':bound,'executed_hooks':hooks,
        'native_replacement_pairs':pairs,'source_slot_retention':retention,'source_slots':[int(re.search(r'source_slot=(\d+)',a['request'])[1]) for a in hats['attempts']],
        'lease_scope':'Old slots unchanged through cleanup+released=1; subsequent slot reuse observed. Exact token/generation identity not captured.',
        'copy_loss':loss_pair,'teardown':td_summary,'trace_counters':counters,
        'console_bytes':console.stat().st_size,'console_sha256':sha(console),'journal_sha256':sha(journal),
        'console_health_matches':hits(console,r'Unmapped|HW\.Memory|panic|abort|crash|SIGBUS|SIGSEGV'),
        'journal_health_matches':hits(journal,r'SEGV|SIGBUS|core.dump|code=dumped|panic|fatal|runtime.*(?:expired|exceeded)'),
        'down':{'released':life['released'],'cleanup_failures':life['cleanup_failures'],'restore':load('state/yoga-restore.json'),
                'verify':load('state/yoga-verify.json'),'unit':props},
        'unverified':['CPU HATS (two no-request attempts)','cancellation','KO','exact lease-token identity','three-interruption acceptance','M5q numeric comparison (unmetered)']}
def main():
    p=argparse.ArgumentParser();p.add_argument('archive',type=Path);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
    result=summarize(a.archive);a.out.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:result[k] for k in ('final_counts','retained_failed_archives','console_health_matches','journal_health_matches','unverified')},indent=2))
if __name__=='__main__':main()
