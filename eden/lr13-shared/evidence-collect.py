import subprocess
from pathlib import Path
b=Path('/home/intpa/smash-training-lobby/artifacts/trainingrooms-lead-astra4/rc8/lr13-shared')
def run(args,cap=60):subprocess.run(args,check=True,timeout=cap)
run(['scp','/tmp/opencode/lr13-export.py','memento-claw-yoga-ubuntu:/home/intpa/tr-lr13-export.py'],20)
run(['ssh','-T','memento-claw-yoga-ubuntu','/usr/bin/python3 -B -I /home/intpa/tr-lr13-export.py'],40)
run(['scp','memento-claw-yoga-ubuntu:/home/intpa/tr-lr13-state-evidence.tgz',str(b)],90)
run(['scp','-r','memento-claw-yoga-ubuntu:/home/intpa/tr-lr13-inputs',str(b/'final-inputs')],120)
run(['/usr/bin/python3','-B','-I','/tmp/opencode/lr13-close.py'],60)
