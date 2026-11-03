import hashlib,json,shutil
from pathlib import Path
from PIL import Image,ImageDraw
raw=Path('/home/intpa/smash-training-lobby/artifacts/trainingrooms-lead-astra4/rc8/lr13-shared/final-inputs/out')
dest=Path('/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work/eden/lr13-shared/review-media');dest.mkdir(exist_ok=True)
bindings=[]
def sheet(name,files):
    cards=[]
    for p in files:
        im=Image.open(p).convert('RGB');im.thumbnail((640,337))
        card=Image.new('RGB',(640,365),'white');card.paste(im,(0,28));ImageDraw.Draw(card).text((4,5),p.parent.name+'/'+p.name[:55],fill='black');cards.append(card)
        bindings.append({'source':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'sheet':name})
    canvas=Image.new('RGB',(1280,365*((len(cards)+1)//2)),'white')
    for i,card in enumerate(cards):canvas.paste(card,((i%2)*640,(i//2)*365))
    canvas.save(dest/name)
sheet('kirby-settled.png',sorted((raw/'RC-KIRBY-ROB').glob('*settled-kirby*.png')))
files=list((raw/'RC-HATS').glob('*hat-0.png'))+list((raw/'RC-HATS').glob('*settled-copied*.png'))+list((raw/'RC-HATS').glob('*settled-normal*.png'))
files+=list((raw/'RC-SPECIAL-ordinary').glob('*settled-ordinary-rob*.png'))
files+=list((raw/'RC-SPECIAL-aegis').glob('*settled-aegis*.png'))+list((raw/'RC-SPECIAL-aegis').glob('*r3-pause*.png'))
sheet('copied-departure-special.png',files)
pilot=json.loads((raw/'RC-final/PILOT-current-shape.json').read_text())['value']
files=[raw/'RC-final'/Path(pilot[k]['path']).name for k in ('manual_before','armed','reopened_cpu','released')]
files += [raw/'RC-final'/Path(pilot[k]).name for k in ('closed_armed','closed_released')]
sheet('pilot-shapes.png',files)
files=[raw/'RC-NOTICE-p1'/Path(json.loads((raw/'RC-NOTICE-p1.json').read_text())['results'][0]['value']['shot']).name]
notice=json.loads((raw/'RC-final/NOTICE-cpu-drawn.json').read_text())['value']
files += [raw/'RC-final'/Path(notice[k]).name for k in ('shot','ready_shot')]
sheet('notice-roles.png',files)
for name in ('pilot-current-ai.gif','tr2-jab-x4.gif'):
    path=raw/'RC-final'/name;im=Image.open(path);frames=[]
    for index in range(0,im.n_frames,max(1,im.n_frames//8)):
        im.seek(index);p=dest/(path.stem+'-'+str(index)+'.png');im.convert('RGB').save(p);frames.append(p)
    sheet(path.stem+'-sheet.png',frames)
    bindings.append({'source':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'frames':im.n_frames,'sheet':path.stem+'-sheet.png'})
for folder,pattern in [('RC-KIRBY-ROB','*settled-kirby-2d.png'),('RC-KIRBY-ROB','*settled-kirby-58.png'),('RC-HATS','*hat-0.png'),('RC-HATS','*settled-copied-kirby-away.png'),('RC-HATS','*settled-normal-rob.png'),('RC-SPECIAL-ordinary','*settled-ordinary-rob.png')]:
    for p in (raw/folder).glob(pattern):shutil.copyfile(p,dest/p.name)
(dest/'bindings.json').write_text(json.dumps(bindings,indent=2)+'\n')
