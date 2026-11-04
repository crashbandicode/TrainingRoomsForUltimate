"""Repeat the successful lr15 route; fail closed on missed pending drain."""
import hashlib,importlib.util,json
from pathlib import Path
HERE=Path(__file__).resolve().parent
def load(name):
    spec=importlib.util.spec_from_file_location(name,HERE/(name+'.py'))
    m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
l=load('lead');review=load('review')
def teardown(ctx,index):
    ctx.log({'project_sha256':{x.name:hashlib.sha256(x.read_bytes()).hexdigest() for x in HERE.glob('*.py')},'action':'teardown','index':index})
    result={'index':index}
    try:
        result.update(l.teardown(ctx))
        result['validated_interruption']=review.teardown(result['lines'])
        result['fresh_build']=l.b.wait(ctx,result['fresh'],r'\[BodyBank\] build owner=0 ',8)
        result['fresh_build_ready']=l.b.wait(ctx,result['fresh'],r'\[BodyBank\] ready owner=0 .*kind=0xb previous=Building',8)
        assert review.seq(result['fresh_build'])<review.seq(result['fresh_build_ready'])<=review.seq(result['continuation']['done'])
        # lead.teardown already uses steps.cycle: HUD gate then settled screenshot.
        result['settled_reentry_shot']=result['continuation']['settled_shot']
        result['continuation_lines']=l.p.lines(ctx,result['fresh'])
        return result
    finally:
        (ctx.row_dir/'teardown.json').write_text(json.dumps(result,indent=2)+'\n')
        l.p.snapshot(ctx)
