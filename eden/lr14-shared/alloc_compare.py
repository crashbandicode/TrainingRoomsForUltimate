"""Extract labelled allocation counters without conflating companion or RSS data."""
import hashlib,json,re,sys
from pathlib import Path
def extract(path):
    raw=path.read_bytes();rows=[]
    for number,line in enumerate(raw.decode(errors='replace').splitlines(),1):
        if not any(x in line for x in ('[TrainingModpack]','[TrainingRooms]','plugin=training_rooms/','[HookAlloc]','[Alloc]','[Hitbox]')):continue
        counters=dict(re.findall(r'\b([A-Za-z_]*alloc[A-Za-z_]*)=(\d+|unmetered)\b',line))
        if counters:rows.append({'line':number,'counters':counters,'context':line[-1800:]})
    return {'path':str(path),'sha256':hashlib.sha256(raw).hexdigest(),'rows':rows}
if __name__=='__main__':
    print(json.dumps([extract(Path(x)) for x in sys.argv[1:]],indent=2))
