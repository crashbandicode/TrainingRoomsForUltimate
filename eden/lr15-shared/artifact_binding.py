import hashlib,json,subprocess
from pathlib import Path
base=Path('/home/intpa/smash-training-lobby');build=base/'artifacts/trainingrooms-pkg6-rc9/build3';raw=base/'artifacts/trainingrooms-lead-astra4/rc9/lr15';dst=base/'TrainingRoomsForUltimate-work/eden/lr15-shared'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
handoff=json.loads((build/'HANDOFF.json').read_text());nro=build/'diag/libtraining_rooms.nro'
assert sha(nro)==handoff['diag_nro_sha256']
config=json.loads((raw/'config.json').read_text());instance=config['instances'][0]
assert instance['plugins'][0]['sha256']==sha(nro)
buildid=nro.read_bytes()[0x40:0x60].hex()
actions=[json.loads(x) for x in (raw/'out/actions.jsonl').read_text().splitlines()]
modules=next(x['modules']['modules'] for x in actions if 'trace_wiring' in x)
loaded=next(x for x in modules if x['build_id']==buildid)
head=subprocess.check_output(['git','-C',str(base/'TrainingRoomsForUltimate-rc9'),'rev-parse','HEAD'],text=True,timeout=20).strip()
assert head==handoff['commit']
binding={'source_commit':head,'source_content_sha256_from_build_handoff':handoff['source_content_sha256'],
         'diag_nro_sha256':sha(nro),'nro_build_id':buildid,'loaded_module':loaded,
         'config_plugin_sha256':instance['plugins'][0]['sha256'],'eden':instance['app'],
         'build_handoff_sha256':sha(build/'HANDOFF.json'),'build_audit_sha256':sha(build/'AUDIT.json'),
         'scope':'NRO bytes/pin/loaded build-ID matched; source content provenance from retained Primary/pkg6 build audit, no lane rebuild.'}
for path in [raw/'artifact-binding.json',dst/'artifact-binding.json']:path.write_text(json.dumps(binding,indent=2)+'\n')
print(json.dumps(binding,indent=2))
