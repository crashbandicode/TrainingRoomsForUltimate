set -e
# wind-articles (11-15), copied from rc14-hide-cpu-tag/up-hidetag.sh, itself copied from integration-3 up-final.sh (wa-base per label, owner): the rc.14 final-pass boot on the Yoga (one Eden). Adapted from worker A-3's yoga-up-eggs.sh (itself from
# rc12-boot4/remote_up.sh): the card set and settings of A-3's stage, the merged rows folder (Break the Targets, Eggs-ercise, Hide CPU,
# the notice, the other lanes' rows), the seeded layouts file of btt-E42, the final diag NRO. Placeholders __L__ __PORT__ __DISP__
# __AUTH__ are filled by stage-final.ps1. Run it only when the primary has assigned the slot.
PY=/usr/bin/python3
T=~/pb-terra-13b2e83/tools/eden-test/eden_test.py
L=__L__
I=~/eden-runs/inputs-$L
PORT=__PORT__
DISPNUM=__DISP__
WAITQ=__WAITQ__
check() {
  N=$(pgrep -af '/usr/bin/eden --control-socket' | grep -v pgrep | grep -o 'eden-runs/[^/ ]*' | sort -u | wc -l)
  echo "edens running: $N"
  if [ "$N" -ge 2 ]; then echo "HOST BUSY"; return 3; fi
  ps -eo pid,etimes,args | grep '/usr/bin/eden --control-socket' | grep -v grep | awk '{print $1, $2}' > /tmp/int3-edens.txt || true
  cat /tmp/int3-edens.txt
  if awk '$2 < 600 {f=1} END {exit !f}' /tmp/int3-edens.txt; then echo "LOAD PHASE: another Eden is under 10 min old"; return 7; fi
  return 0
}
# WAITQ=1 (a second plan on a slot, started by the first one's chain): wait up to 30 min for a quiet host instead of refusing
TRIES=0
while true; do
  rc=0; check || rc=$?
  if [ "$rc" -eq 0 ]; then break; fi
  if [ "$WAITQ" = 1 ] && [ "$TRIES" -lt 60 ]; then TRIES=$((TRIES+1)); echo "waiting 30 s ($rc)"; sleep 30; continue; fi
  exit $rc
done
RC=$(pgrep -c -x rustc || true); if [ "$RC" -gt 0 ]; then echo "HOST COMPILING (the lifecycle checks nice too)"; fi
if ss -ltn | grep -q ":$PORT "; then echo "PORT $PORT BUSY"; exit 4; fi
if [ -e /tmp/.X11-unix/X$DISPNUM ]; then echo "DISPLAY :$DISPNUM BUSY"; exit 5; fi
test ! -e ~/eden-runs/$L
rm -rf $I && mkdir -p $I && tar -xf ~/eden-runs/$L.tar -C $I && rm -f ~/eden-runs/$L.tar
echo $PORT > $I/rows-hang/gdb-port.txt
mkdir -p ~/eden-runs/wa-base-$L && cp ~/eden-runs/$L-base.json ~/eden-runs/wa-base-$L/config.json && rm -f ~/eden-runs/$L-base.json
$PY - <<'EOF'
import hashlib, json, os
from pathlib import Path
p = Path(os.path.expanduser('~/eden-runs/wa-base-__L__/config.json'))
c = json.loads(p.read_text())
c['host'], c['hostname'] = '__HOSTKEY__', '__HOSTNAME__'
__LEDGER__
c['runner'] = {'path': os.path.expanduser('~/pb-terra-13b2e83/tools/eden-runner/runner.py'), 'sha256': c['runner']['sha256']}
def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
missing = []
for label, path, want in [('runner', c['runner']['path'], c['runner']['sha256']),
                          ('display_helper', c['display_helper']['path'], c['display_helper']['sha256'])]:
    if not Path(path).exists(): missing.append(label + ' ' + path)
    elif sha(path) != want: missing.append(label + ' sha ' + path)
if not Path(c['playbook']['root']).exists(): missing.append('playbook ' + c['playbook']['root'])
inst = c['instances'][0] if isinstance(c['instances'], list) else c['instances']
for path in [inst['app']['path'], inst['game'], inst['source']]:
    if not Path(path).exists(): missing.append(path)
for f in inst['files']:
    if not Path(f['path']).exists(): missing.append('file ' + f['path'])
print('missing', missing)
if missing: raise SystemExit(6)
p.write_text(json.dumps(c, indent=1) + '\n')
EOF
$PY $T new --from ~/eden-runs/wa-base-$L/config.json --label $L --rows $I/rows-hang --plugin $I/inputs/plugins/libtraining_rooms.nro --owner 'Claude wind-articles' --defer-load
R=~/eden-runs/$L
$PY - "$R" "$I" <<'EOF'
import hashlib, json, sys
from pathlib import Path
run, inp = Path(sys.argv[1]), Path(sys.argv[2])
p = run / 'config.json'
c = json.loads(p.read_text())
inst = c['instances'][0] if isinstance(c['instances'], list) else c['instances']
def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()
plugins = []
for nro in sorted((inp / 'inputs' / 'plugins').glob('*.nro')):
    plugins.append({'path': str(nro), 'sha256': sha(nro), 'name': nro.name})
cursor = Path.home() / 'trainingrooms-eden-pkg6/nro/aid/libssbu_tile_cursor.nro'
plugins.append({'path': str(cursor), 'sha256': sha(cursor)})
inst['plugins'] = plugins
drop = ('user/sdmc/ultimate/TrainingRooms/', 'user/sdmc/ultimate/ssbu_online_deluxe_rewrite/config.ini')
files = [f for f in inst['files'] if not f['to'].startswith(drop)]
manifest = json.loads((inp / 'inputs' / 'manifest.json').read_text())
for f in manifest['files']:
    src = inp / 'inputs' / 'files' / f['rel']
    files.append({'path': str(src), 'sha256': sha(src), 'to': 'user/sdmc/ultimate/' + f['rel']})
inst['files'] = files
c.pop('plugin_swaps', None)
c.pop('final', None)
__LEDGER__
c['runtime'] = 5400  # 25 row minutes plus the boot (MACROS rule 8: a batch boot sets 5400 or more)
inst['display'] = ':__DISP__'
c['authorization'] = ('__AUTH__')
p.write_text(json.dumps(c, indent=1) + '\n')
print('plugins', len(plugins), 'files', len(files), 'runtime', c['runtime'])
print('tr', [x['sha256'][:12] for x in plugins if x['path'].endswith('libtraining_rooms.nro')])
print('fixture', [f['to'] for f in files if f['to'].endswith('seeded-big-battlefield.json')])
EOF
# cycle-fix's boot fixture: derive the run's pinned TrainingRooms settings.json (Ready Fighters 6 etc.) before Up, as its own boot did
if [ -f $I/rows-hang/cf_fixture.py ]; then $PY $I/rows-hang/cf_fixture.py $R $I/rows-hang/fixture.json; fi
cat > $R/chain.sh <<EOS
#!/bin/bash
R=$R
PY=/usr/bin/python3
T=$T
log() { echo "\$(date -u +%H:%M:%SZ) \$*" >> \$R/chain.log; }
UNTIL=\$(( \$(date +%s) + 1500 ))
log "up start"; \$PY \$T up \$R > \$R/up.out 2>&1; rc=\$?; log "up rc=\$rc"
if [ \$rc -eq 0 ]; then
  log "runner start"; \$PY \$T run \$R > \$R/run-1.out 2>&1; log "runner rc=\$?"
  log "hold until \$(date -u -d @\$UNTIL +%H:%M:%SZ) or RELEASE"
  while [ ! -f \$R/RELEASE ] && [ \$(date +%s) -lt \$UNTIL ]; do sleep 15; done
  while pgrep -f "eden_test.py run \$R" > /dev/null; do sleep 10; done
fi
for f in \$R/state/run-*/*/user/sdmc/ultimate/TrainingRooms/practice-diagnostics*.log; do [ -f "\$f" ] && cp "\$f" \$R/out/ ; done
for f in \$R/work/profiles/profile-diag/user/sdmc/ultimate/TrainingRooms/targets/*.json; do [ -f "\$f" ] && cp "\$f" \$R/out/ ; done
for f in \$R/work/profiles/profile-diag/user/sdmc/ultimate/TrainingRooms/layouts/*.json; do [ -f "\$f" ] && cp "\$f" \$R/out/ ; done
log "down start"; \$PY \$T down \$R > \$R/down.out 2>&1; log "down rc=\$?"
__THEN__
EOS
chmod +x $R/chain.sh
setsid nohup $R/chain.sh > /dev/null 2>&1 < /dev/null &
sleep 3
cat $R/chain.log
