import json,hashlib
from pathlib import Path
from PIL import Image,ImageDraw
root=Path('/home/intpa/smash-training-lobby/artifacts/trainingrooms-lead-astra4/rc9/lr15')
load=lambda p:json.loads((root/p).read_text())
def local(p):return root/Path(p).relative_to('/home/intpa/eden-runs/lr-15')
hats=load('out/RC-HATS-P1/hats.json');loss=load('out/RC-HATS-copy-loss/copy-loss.json');rob=load('out/RC-KIRBY-ROB.json')['results'][0]['value'];tear=load('out/RC-TEARDOWN-slow36.json')['results'][0]['value']
groups={'hats':[(str(i)+' '+str(a['kind']),a['shot']) for i,a in enumerate(hats['attempts'])],
 'loss': [('taunt loss',loss['taunt_shot']),('reacquired',loss['reacquired']['shot'])],
 'rob':[(str(i),a['settled_shot']) for i,a in enumerate(rob['cycle'])]+[('copy',rob['copy']['shot']),('away',rob['copied_departure']['away']['settled_shot']),('normal ROB',rob['copied_departure']['rob']['settled_shot'])],
 'teardown':[('continuation',tear['continuation']['settled_shot'])]}
manifest=[]
for name,items in groups.items():
    sheet=Image.new('RGB',(1280,390*((len(items)+1)//2)),(22,22,22));draw=ImageDraw.Draw(sheet)
    for i,(label,path) in enumerate(items):
        p=local(path);image=Image.open(p).convert('RGB');image.thumbnail((640,360))
        x=(i%2)*640;y=(i//2)*390;sheet.paste(image,(x,y+25));draw.text((x+8,y+5),label,fill='white')
        manifest.append({'group':name,'label':label,'path':str(p.relative_to(root)),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
    sheet.save(root/(name+'-contact.png'))
(root/'media-index.json').write_text(json.dumps(manifest,indent=2)+'\n')
