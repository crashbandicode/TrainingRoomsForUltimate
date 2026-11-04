"""Derive RC9 row data from the retained lr14 navigation manifest."""
import copy,hashlib,json
from pathlib import Path
D=Path(__file__).resolve().parent
def main():
    source=D.parent/'lr14-shared/rows.json'
    plan=json.loads(source.read_text())
    plan['scope']='lr15 single grouped RC9 diagnostic boot; native cleanup, HATS, KIRBY-ROB and BOOT'
    plan['rows']=[r for r in plan['rows'] if not r['id'].startswith(('RC-','R3-'))]
    for name,action,args in [('RC-BOOT-install','boot',{}),('RC-HATS-P1','hats',{'role':0}),('RC-HATS-copy-loss','copy_loss',{}),('RC-HATS-CPU','hats',{'role':1}),('RC-KIRBY-ROB','kirby_rob',{}),('RC-TEARDOWN-slow36','teardown',{}),('RC-BOOT-postrows','final_alloc',{})]:
        plan['rows'].append({'id':name,'start':'training','end':'training','steps':[{'run':'lead.py:dispatch','args':dict(action=action,**args),'check':True}]})
    macro=json.loads((D/'segments/hat-next.edenmacro.json').read_text())
    # Eden button bit order is bound by the recorded P1 chord: LStick=16,
    # RStick=32. Preserve every stick/trigger/motion sample field.
    macro['name']='cpu-hat-next'
    for event in macro['events']:
        if event['buttons'] & 16:event['buttons']=(event['buttons'] & ~16)|32
    path=D/'segments/cpu-hat-next.edenmacro.json';path.write_text(json.dumps(macro,indent=2)+'\n')
    plan['segments']['cpu-hat-next']={'file':'segments/'+path.name,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'cap':15}
    plan['segments']['kirby-taunt-loss']={'file':'segments/kirby-taunt-loss.edenmacro.json','recipe':[{'buttons':'down','frames':8,'gap_frames':60}],'cap':15}
    plan['deferred']=[{'id':'HATS-nonzero-Kirby-cell','reason':'Valid product layout pins Kirby physical cell0; ordinary nonzero activation is separate regression evidence.'},
        {'id':'HATS-cancel-KO','reason':'Concrete native routes and evidence gates pending; add on same boot after qualified dry run.'}]
    (D/'rows.json').write_text(json.dumps(plan,indent=2)+'\n')
if __name__=='__main__':main()
