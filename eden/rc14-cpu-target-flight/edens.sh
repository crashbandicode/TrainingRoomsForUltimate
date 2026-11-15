#!/bin/bash
# rc.14 cpu-target-flight: the Eden runs on this host and their ages in seconds (read-only).
ps -eo etimes,args | grep '/usr/bin/eden --control-socket' | grep -v grep | while read -r age rest; do
  echo "$age $(echo "$rest" | grep -o 'eden-runs/[^/ ]*' | head -1)"
done
