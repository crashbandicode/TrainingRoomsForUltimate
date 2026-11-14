# btt-3: Break the Targets Eden boot (SPEC V2.5 rows 26-31, 41-43). Piped to `bash -s` on the Eden host by stage_btt.ps1.
# Modelled on d2-btt-cam-1's up_cam.sh (it booted, loaded and reached Training with these pins). DRAFT: not run yet.
# Placeholders: __LABEL__ __NRO_SHA__ __FIX_SHA__ __BASE__ __PORT__ __DISP__ __HOST__ __OWNER__
set -e
PY=/usr/bin/python3
T=~/ssbu-mod-playbook/tools/eden-test/eden_test.py
L=__LABEL__
I=~/eden-runs/_btt3-inputs
BASE=__BASE__
PORT=__PORT__
DISP=__DISP__
# Admission (the lifecycle enforces it too): the ledger is readable, at most one other Eden here, nothing compiling un-niced,
# the port and the display free, the run folder new.
test -e /tmp/deluxe-surface-1007/OWNER-COORDINATION.md || ssh -n -o BatchMode=yes memento-claw-yoga-ubuntu test -e /tmp/deluxe-surface-1007/OWNER-COORDINATION.md
N=$(pgrep -c -f '/usr/bin/eden --control-socket' || true)
echo "edens running: $N"
if [ "$N" -ge 2 ]; then echo "HOST BUSY"; exit 3; fi
RC=$(pgrep -c -x rustc || true); if [ "$RC" -gt 0 ]; then echo "HOST COMPILING"; exit 4; fi
if ss -ltn | grep -q ":$PORT "; then echo "PORT $PORT BUSY"; exit 4; fi
if [ -e /tmp/.X11-unix/X$DISP ]; then echo "DISPLAY :$DISP BUSY"; exit 5; fi
test ! -e ~/eden-runs/$L
echo "__NRO_SHA__  $I/libtraining_rooms.nro" | sha256sum -c -
echo "__FIX_SHA__  $I/fixtures/seeded-big-battlefield.json" | sha256sum -c -
# The rows folder: shipped as a tarball, unpacked fresh for this run.
rm -rf $I/rows-$L && mkdir -p $I/rows-$L && tar -xzf $I/rows.tgz -C $I/rows-$L
echo $PORT > $I/rows-$L/gdb-port.txt
$PY -I $T new --from $BASE --label $L --rows $I/rows-$L --plugin $I/libtraining_rooms.nro --owner '__OWNER__'
R=~/eden-runs/$L
# Config: this run's display, the seeded layout file for Big Battlefield (btt-5: layouts/, one file per layout) (a file the boot did not write), the authorization.
$PY -I - "$R" "$DISP" "$I/fixtures/seeded-big-battlefield.json" "__FIX_SHA__" <<'EOF'
import json, sys
from pathlib import Path
p = Path(sys.argv[1]) / 'config.json'
c = json.loads(p.read_text())
inst = c['instances'][0] if isinstance(c['instances'], list) else c['instances']
inst['display'] = ':' + sys.argv[2]
inst.setdefault('files', []).append({'path': sys.argv[3], 'sha256': sys.argv[4],
                                     'to': 'user/sdmc/ultimate/TrainingRooms/layouts/seeded-big-battlefield.json'})
c['authorization'] = ('Break the Targets rows (SPEC V2.5, the primary lane 2026-11-14): the card set of the base config with the '
                      'TrainingRooms diag build of the d2/btt lane, one instance, Training only, muted (volume 0), the GDB stub '
                      'on before game.load. The seeded layouts/seeded-big-battlefield.json is a fixture the boot reads; Down restores the profile. '
                      'No card write; nothing pushed.')
p.write_text(json.dumps(c, indent=1) + '\n')
print('display', inst['display'], 'files', [f['to'] for f in inst['files']][-2:], 'volume', json.dumps(c.get('eden_settings'))[:120])
EOF
cat > $R/chain.sh <<EOS
#!/bin/bash
R=$R
PY=/usr/bin/python3
T=$T
log() { echo "\$(date -u +%H:%M:%SZ) \$*" >> \$R/chain.log; }
log "up start"; \$PY -I \$T up \$R > \$R/up.out 2>&1; rc=\$?; log "up rc=\$rc"
if [ \$rc -eq 0 ]; then
  log "runner start"; \$PY -I \$T run \$R > \$R/run-1.out 2>&1; log "runner rc=\$?"
fi
for f in \$R/state/run-*/*/user/sdmc/ultimate/TrainingRooms/practice-diagnostics*.log \$R/work/profiles/profile-diag/user/sdmc/ultimate/TrainingRooms/practice-diagnostics*.log; do [ -f "\$f" ] && cp "\$f" \$R/out/ ; done
for f in \$R/work/profiles/profile-diag/user/sdmc/ultimate/TrainingRooms/layouts/*.json; do [ -f "\$f" ] && cp "\$f" \$R/out/ ; done
log "down start"; \$PY -I \$T down \$R > \$R/down.out 2>&1; log "down rc=\$?"
EOS
chmod +x $R/chain.sh
setsid nohup $R/chain.sh > /dev/null 2>&1 < /dev/null &
sleep 3
cat $R/chain.log
