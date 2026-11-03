"""Summarize canonical receipts, full console and exact unit journal after Down."""
import ast,collections,hashlib,json,re,sys,tarfile
from pathlib import Path
D=Path(__file__).resolve().parent
def load(p):return json.loads(p.read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    raw=Path(sys.argv[1]);life=load(raw/'state/lifecycle.json')
    rows=[(p.name,load(p)) for p in sorted((raw/'out').glob('*.json'))]
    rows=[(n,r) for n,r in rows if 'status' in r and 'id' in r]
    health={}
    for p in [raw/'state/run-yoga/console.log',raw/'yoga-unit-journal.log']:
        lines=p.read_text(errors='replace').splitlines()
        hits=[{'line':i,'text':s} for i,s in enumerate(lines,1) if re.search(r'Unmapped|SIGBUS|core-dump|fatal error|panic|\babort\b|\bcrash\b|status=7/BUS',s,re.I)]
        health[str(p.relative_to(raw))]={'sha256':sha(p),'matches':hits}
    manifests={sha(p):str(p.relative_to(raw)) for p in (raw/'rows').glob('rows*.json')}
    sources={sha(p):str(p.relative_to(raw)) for p in (raw/'rows').glob('*.py')}
    for p in raw.glob('attempt*.tar'):
        with tarfile.open(p) as archive:
            for item in archive:
                if item.isfile() and (item.name.endswith('.py') or item.name=='rows/rows.json'):
                    digest=hashlib.sha256(archive.extractfile(item).read()).hexdigest()
                    (sources if item.name.endswith('.py') else manifests)[digest]=p.name+':'+item.name
    bindings={n:manifests.get(r['manifest_sha256']) for n,r in rows if 'manifest_sha256' in r}
    actions=[json.loads(x) for x in (raw/'out/actions.jsonl').read_text().splitlines()]
    missing=sorted({h for x in actions for h in x.get('project_sha256',x.get('loaded_project_sha256',{})).values() if h not in sources})
    for p in D.glob('*.py'):ast.parse(p.read_text(),filename=str(p))
    result={'rows':dict(collections.Counter(r['status'] for _,r in rows)),
            'failed':[{'file':n,'error':r.get('error')} for n,r in rows if r['status']!='PASS'],
            'released':life['released'],'cleanup_failures':life.get('cleanup_failures'),'units':load(raw/'closed-units.json'),
            'health':health,'receipt_manifest_bindings':bindings,'missing_project_source_hashes':missing,'python_syntax':'PASS'}
    assert all(bindings.values()),bindings
    assert not missing,missing
    (D/'closure.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
if __name__=='__main__':main()
