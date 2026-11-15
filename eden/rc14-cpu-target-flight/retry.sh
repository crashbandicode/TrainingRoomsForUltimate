#!/bin/bash
# rc.14 cpu-target-flight: rerun a held boot's failed rows on the same boot (eden_test.py run --retry), detached; the
# boot's chain.sh waits for it before Down. Usage on the Eden host: bash retry.sh <label> <n>
set -e
R=~/eden-runs/$1
cd "$R"
rm -f out/STOP-INPUT
setsid nohup /usr/bin/python3 -I ~/pb-terra-13b2e83/tools/eden-test/eden_test.py run "$R" --retry > "run-$2.out" 2>&1 < /dev/null &
sleep 2
pgrep -af 'eden_test.py run' | grep -v pgrep | cut -c1-140 || true
