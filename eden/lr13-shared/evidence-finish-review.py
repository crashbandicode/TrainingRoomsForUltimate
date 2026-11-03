import hashlib,json,shutil,datetime
from pathlib import Path
d=Path('/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work/eden/lr13-shared')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
actions=[json.loads(x) for x in (d/'out/actions.jsonl').read_text().splitlines()]
pins=json.loads((d/'tool-pins.json').read_text())
available={sha(p):p.name for p in d.glob('*.py')}
dispatches=[x for x in actions if 'loaded_project_sha256' in x]
pins['executed_hook_sources']={x['loaded_project_sha256']['steps.py']:available.get(x['loaded_project_sha256']['steps.py']) for x in dispatches}
assert all(pins['executed_hook_sources'].values())
pins['all_executed_project_sources_retained']=all(h in available for x in dispatches for h in x['loaded_project_sha256'].values())
assert pins['all_executed_project_sources_retained']
pins['last_executed_hook_sha256']=dispatches[-1]['loaded_project_sha256']['steps.py']
pins['latest_uploaded_hook_sha256']=sha(d/'steps.py')
pins['latest_uploaded_executed']=pins['latest_uploaded_hook_sha256'] in pins['executed_hook_sources']
pins['unexecuted_scope']='Latest adds 0x67ead0 installed-identity trace and MII final retry; uploaded after RC-final loaded 03a4cb33, therefore NOT executed.'
(d/'tool-pins.json').write_text(json.dumps(pins,indent=2)+'\n')
review=json.loads((d/'review.json').read_text())
review['visual_review']={
 'kirby_rob':{'status':'PASS requested settled-body/HUD clauses','sheets':['review-media/kirby-settled.png','review-media/copied-departure-special.png'],'details':'All seven roster targets have matching body/portrait. Mario-hat Kirby confirmed#1402; canonical fallback#1411; settled Falcon then ordinary R.O.B. body/portrait.'},
 'special':{'status':'PASS ordinary settled-HUD and Aegis pause clauses','sheet':'review-media/copied-departure-special.png','details':'Ordinary ROB white/grey with red lamp and matching portrait; Pyra body+portrait, native CPUs1 pause, begin#2156/end#2159/resume#2160 leaked0.'},
 'notice':{'status':'PASS P1-only and CPU-only fixtures','sheet':'review-media/notice-roles.png','details':'P1 swap1#363 visually Swap P1 once to load; CPU retry swap2#3423/draw#3426 visibly Swap the CPU once to load. Ready1/1#3461 visible in pilot-shapes and AI frames0/6/12; initial notice-total screenshot itself still showed CPU fighter applied.'},
 'pilot':{'status':'PASS arm/release and observed AI/manual response','sheets':['review-media/pilot-shapes.png','review-media/pilot-current-ai-sheet.png','review-media/tr2-jab-x4-sheet.png'],'details':'Current selected CPU(L3), close, uncontrolled Steve attacks and movement, reopen stillCPU, Player release, close, manual jab then idle. Original marker failure retained. Classifier limits string says old-binary but marker JSON explicitly binds current lr12 Player and lr13 CPU references.'},
 'hats':{'status':'FAILED five hats','details':'Initial Mario request#1401/confirm#1402 only; helper kind1 was ready but next copy#1403 notice Kirby is still changing characters. Canonical retry Mario#3197/3198 then same notice#3209; fallback-only postcheck failed too, masking earlier nested error. No kind7-ready/five confirmations.'},
 'r3':{'status':'FAILED independent producer/retirement','details':'Original row failed old pause image gate during fixture transition; gated recovery succeeded. Aegis native pause is narrower positive; renderer-unavailable independence untested.'}}
review['hud_change_gates']=[{k:x.get(k) for k in ('time','gate','on','pass','shot','mark')} for x in actions if str(x.get('gate','')).startswith('hud-settled-') and x.get('pass')]
review['artifact_integrity']={'executed_hook_sources_retained':True,'exact_manifest_bindings':pins['manifest_bindings'],'latest_uploaded_executed':pins['latest_uploaded_executed']}
review['requested_outcome']='RC NOT READY. Shutdown success, HATS five, pending teardown, complete Mii tuple and R3 independent producer/retirement remain unverified/failed.'
(d/'review.json').write_text(json.dumps(review,indent=2)+'\n')
for name in ('review','media','finish-review','export','collect','close'):
    shutil.copyfile(Path('/tmp/opencode')/('lr13-'+name+'.py'),d/('evidence-'+name+'.py'))
print(json.dumps({'executed_hooks':pins['executed_hook_sources'],'last_executed':pins['last_executed_hook_sha256'],'latest_unexecuted':pins['latest_uploaded_hook_sha256'],'stable_gates':len(review['hud_change_gates'])},indent=2))
