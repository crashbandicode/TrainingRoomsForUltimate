"""Claim ledger helper for the Ally flow (the ledger lives on the Yoga). Host-aware as pkg6's harness (stream/pkg6 9360dfa):
the host a claim is ON comes from its "on <host>" phrase, so a line names ONLY the host it claims.

    python3 -I ledger.py open ally                  # open claims (by other owners) that concern the Ally
    python3 -I ledger.py claim ally "<text>"        # refused while another owner's claim on that host is open
    python3 -I ledger.py release ally "<text>"
"""
import calendar
import re
import subprocess
import sys
import time

OWNER = "Claude 0caa72b8/pkg5-eden"
LEDGER = "/tmp/deluxe-surface-1007/OWNER-COORDINATION.md"
SSH = ["/usr/bin/ssh", "-o", "BatchMode=yes", "-i", "/home/intpa/.ssh/id_rsa", "memento-claw-yoga-ubuntu"]
STALE_HOURS = 12
HOSTS = ("surface", "yoga", "ally")


def text():
    return subprocess.run([*SSH, f"cat {LEDGER}"], capture_output=True, text=True, check=True).stdout


def open_claims(host):
    state = {}
    now = time.time()
    for line in text().splitlines():
        m = re.search(r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2})Z (CLAIMED|RELEASED):? (\S+ \S+)", line)
        if not m:
            continue
        stamped = calendar.timegm(time.strptime(m.group(1), "%Y-%m-%d %H:%M"))
        names = re.findall(r"\bon (?:the )?(surface|yoga|ally)\b", line, re.I) or re.findall(r"\b(surface|yoga|ally)\b", line, re.I)
        concerns = not names or host in {h.lower() for h in names}
        state[m.group(3)] = m.group(2) == "CLAIMED" and (now - stamped) / 3600 < STALE_HOURS and concerns
    return [k for k, v in state.items() if v and k != OWNER]


def write(kind, host, body):
    assert host in HOSTS
    others = [h for h in HOSTS if h != host and re.search(rf"\b{h}\b", body, re.I)]
    assert not others, f"the line must name only {host}, it also names {others}"
    assert re.search(rf"\bon (the )?{host}\b", body, re.I), f'the line needs the phrase "on {host}"'
    stamp = time.strftime("%Y-%m-%d %H:%M", time.gmtime())
    line = f"\n{stamp}Z {kind}: {OWNER} {body}\n"
    subprocess.run([*SSH, f"cat >> {LEDGER}"], input=line, text=True, check=True)
    return line.strip()


if __name__ == "__main__":
    op, host = sys.argv[1], sys.argv[2]
    if op == "open":
        print(open_claims(host))
    elif op == "claim":
        busy = open_claims(host)
        if busy:
            sys.exit(f"refused, open claims on {host}: {busy}")
        print(write("CLAIMED", host, sys.argv[3]))
    elif op == "release":
        print(write("RELEASED", host, sys.argv[3]))
