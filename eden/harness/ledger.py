"""Claim ledger: parse, open claims per host, claim, release, queue head, fairness (one module for every stream).

Playbook origin: the coordination-file block of tools/eden-solo/solo.ps1 (`$coord`, the `(CLAIMED|RELEASED): (\\S+ \\S+)`
parse, "Add-Content ... CLAIMED:" and "RELEASED:" lines), playbook ae87443, read together with
docs/EDEN.md "Queue runs" and README "Rules that bite". Replaces eden/ally/ledger.py, the `open_claims`/`ledger`
functions of the nine tr_eden copies and the helpers in /tmp/integrator3 (ledger.py, surface_free.py,
claim_when_free.py).

What it adds to solo.ps1's parse (all seen in the real ledger):
  * stamps `YYYY-MM-DD HH:MMZ` and `YYYY-MM-DD HH:MM:SSZ` (before, a line with seconds was never read, so its
    RELEASED never closed a claim);
  * `CLAIMED:`, `CLAIMED again:`, `CLAIMED (build only, ...):`, `RELEASED (not started):` and so on;
  * owners of every agent: `Claude 0caa72b8/pkg6-eden`, `Codex 01a1...-79c4/imgui-nvn-sol`, `OpenCode ...`,
    `Claude tangent ssbu-refresh-clock-t2` (the owner key is the agent and its id word);
  * host awareness: a claim is ON the host its "on <host>" phrase names; without one, every host the line names;
    without any, every host (conservative). A RELEASED closes the owner's claim on the hosts it names (all
    hosts when it names none);
  * the open claims of foreign owners block regardless of age until a later RELEASED;
    `stale_hours` only separates old claims in the report, never grants admission;
  * fairness: after your RELEASED wait `fair_seconds` before claiming the same host again;
  * the queue file's head item per host (advisory: a free-text file);
  * the queue file's interleave rule (binding, 2026-11-02): where `imgui-nvn-sol` is queued it takes the host after
    each TrainingRooms run and before the next TrainingRooms claim, so `claim` refuses until its CLAIMED follows
    our last RELEASED on that host (`parallel_turn`).

    ledger.py open | free | claim "<text>" | release "<text>" | queue | wait <cap seconds> ["<claim text>"]
"""
import calendar
import re
import subprocess
import sys
import time
from collections import namedtuple
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import params  # noqa: E402

LINE = re.compile(r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2})(?::(\d{2}))?Z (CLAIMED|RELEASED)(?: again| \([^)]*\))?:? (.*)$")
# Two entries written onto one line (a writer that did not start on a new line) are two events.
JOINED = re.compile(r"(?<=\S)(?=\d{4}-\d{2}-\d{2} \d{2}:\d{2}(?::\d{2})?Z (?:CLAIMED|RELEASED))")
ON = re.compile(r"\bon (?:the )?(surface|yoga|ally)\b", re.I)
ANY = re.compile(r"\b(surface|yoga|ally)\b", re.I)
MODEL = re.compile(r"^(claude|gpt|grok|fable|opus|sonnet)-[\w.-]+$", re.I)
ALL = frozenset(("surface", "yoga", "ally"))
Event = namedtuple("Event", "kind when owner hosts text")
Claim = namedtuple("Claim", "owner host when eden text")


def owner_of(rest):
    """`Claude 0caa72b8/f6 (f6 stream) ...` -> `Claude 0caa72b8/f6`; the agent plus its first id word (a word with
    a digit or a slash that is not a model name; `Codex worker ssbu-gnw-judge-eden` has none: the next word)."""
    words = rest.split()
    if len(words) < 2:
        return rest.strip()
    for word in words[1:4]:
        word = word.strip("(),:;")
        if (any(c.isdigit() for c in word) or "/" in word) and not MODEL.match(word):
            return f"{words[0]} {word}"
    return f"{words[0]} {words[1].strip('(),:;')}"


def hosts_of(rest, release=False):
    if release:
        return frozenset(m.lower() for m in ANY.findall(rest)) or ALL
    # Historical claims include explicit non-use and references to a separately
    # claimed run. Those clauses describe other ownership, not this claim.
    # Remove only the evidenced forms, retaining every affirmative host mention
    # elsewhere (including real multi-host claims). Releases remain conservative.
    rest = re.sub(r"\b(?:surface|yoga|ally)\s+not used\b", "", rest, flags=re.I)
    rest = re.sub(r"\bmy Eden run\s+[\w-]+\s+is on (?:the )?(?:surface|yoga|ally)\b",
                  "", rest, flags=re.I)
    named = {m.lower() for m in ANY.findall(rest)}
    return frozenset(m.lower() for m in ON.findall(rest)) or frozenset(named) or ALL


def is_eden(rest):
    return bool(re.search(r"\bEden\b", rest)) and not re.search(r"no Eden|compile only|build only", rest, re.I)


def parse(text):
    """CLAIMED / RELEASED events in file order (the file is append only: later in the file is later)."""
    events = []
    for line in (part for raw in text.splitlines() for part in JOINED.split(raw)):
        m = LINE.match(line.strip())
        if not m:
            continue
        stamp = calendar.timegm(time.strptime(m.group(1), "%Y-%m-%d %H:%M")) + int(m.group(2) or 0)
        kind, rest = m.group(3), m.group(4)
        events.append(Event(kind, stamp, owner_of(rest), hosts_of(rest, kind == "RELEASED"), rest))
    return events


def open_claims(text, host, exclude=(), now=None, stale_hours=12):
    """(open, stale): the claims on `host` whose owner has no later RELEASED, minus `exclude` (own owners);
    `stale` are those at least `stale_hours` old. Both lists block admission until RELEASED."""
    now = time.time() if now is None else now
    held = {}
    for e in parse(text):
        for h in e.hosts:
            key = (e.owner, h)
            if e.kind == "CLAIMED":
                held[key] = Claim(e.owner, h, e.when, is_eden(e.text), e.text)
            else:
                held.pop(key, None)
    mine = {exclude} if isinstance(exclude, str) else set(exclude)
    live = [c for (o, h), c in held.items() if h == host and o not in mine]
    fresh = [c for c in live if (now - c.when) / 3600 < stale_hours]
    return fresh, [c for c in live if c not in fresh]


def since_release(text, owner, host, now=None):
    """Seconds since `owner`'s last RELEASED naming `host` (None when it never released here)."""
    now = time.time() if now is None else now
    last = [e.when for e in parse(text) if e.kind == "RELEASED" and e.owner == owner and host in e.hosts]
    return now - max(last) if last else None


def queue_items(text, host):
    """The items of the queue file's section for `host` (a `## ...` heading naming it)."""
    items, inside = [], False
    for line in text.splitlines():
        if line.startswith("## "):
            inside = host in line.lower()
        elif inside:
            m = re.match(r"^\s*(?:-|\d+\.)\s+(.*)$", line)
            if m:
                items.append(m.group(1).strip())
    return items


def queue_head(text, host):
    """First real item: notes in parentheses (`(imgui-nvn-sol took the Yoga ...)`) do not count."""
    return next((i for i in queue_items(text, host) if not i.startswith("(")), None)


# The parallel projects' owners, matched by name prefix in the owner key and the queue line (tm-f8-terra also
# covers tm-f8-terra-ally and tm-f8-terra-eden; likewise tm-f8-sol, -ally, -eden).
PARALLEL = ("imgui-nvn-sol", "tm-f8-terra", "tm-f8-sol")


def is_parallel(owner):
    return any(name in owner for name in PARALLEL)


def parallel_turns(text, queue, host, now=None, grace=180):
    """The queue file's interleave rule (lead, 2026-11-02, binding on every stream), for each parallel owner: on a
    host where it is queued, it takes the host after each TrainingRooms compile or Eden run and before the next
    TrainingRooms claim. One reason to wait per owner that is queued for `host` (a queue line that does not say it
    took the host or is satisfied) and has no CLAIMED on `host` after the last TrainingRooms RELEASED there (every
    owner that is not a parallel one counts as ours). The turn is `grace` seconds long (lead, 2026-11-02: an idle parallel line
    must not block a host forever): once that long has passed since our last RELEASED with no CLAIMED from it, the next
    TrainingRooms claim may go ahead; the parallel owner keeps priority for the turn after, because the next TrainingRooms
    release starts a new grace."""
    now = time.time() if now is None else now
    if not queue:
        return []
    events = [e for e in parse(text) if host in e.hosts]
    mine = [i for i, e in enumerate(events) if e.kind == "RELEASED" and not is_parallel(e.owner)]
    if not mine:
        return []
    if now - events[mine[-1]].when >= grace:
        return []
    items = queue_items(queue, host)
    later = events[mine[-1] + 1:]
    return [f"{name} is queued for the {host} and has not had it since the last TrainingRooms release there"
            for name in PARALLEL
            if any(name in i and not re.search(r"\btook\b|satisfied", i) for i in items)
            and not any(e.kind == "CLAIMED" and name in e.owner for e in later)]


def parallel_turn(text, queue, host, now=None, grace=180):
    """The first reason from `parallel_turns`, or None."""
    return next(iter(parallel_turns(text, queue, host, now, grace)), None)

def stamp(now=None):
    return time.strftime("%Y-%m-%d %H:%M", time.gmtime(now)) + "Z"


def format_line(p, kind, body, now=None):
    """`<stamp> CLAIMED: <owner> <body>`; the body must name `p.host` as "on <Host>" and no other host (so the
    host-aware parse reads the line right). The stamp has no seconds: older parsers read only that form."""
    title = p.info["title"]
    others = [h for h in ALL if h != p.host and re.search(rf"\b{h}\b", body, re.I)]
    if others:
        raise SystemExit(f"the line must name only {title}, it also names {others}")
    if not re.search(rf"\bon (the )?{title}\b", body, re.I):
        raise SystemExit(f'the line needs the phrase "on {title}"')
    return f"{stamp(now)} {kind}: {p.owner} {body}"


def _ssh(p, *words, tries=4):
    """ssh to the ledger host; a dropped connection is retried (a claim helper must not die on one failed read)."""
    for attempt in range(tries):
        r = subprocess.run(["/usr/bin/ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=15", p.ledger_ssh, *words],
                           capture_output=True, text=True)
        if r.returncode == 0:
            return r
        time.sleep(3 * (attempt + 1))
    raise SystemExit(f"ledger host {p.ledger_ssh} unreachable ({r.returncode}): {r.stderr.strip()[:200]}")


def local_ledger(p):
    """The ledger file lives on the Yoga: read it directly there, over ssh from any other machine (whatever host
    the claim is for)."""
    return not p.ledger_ssh or params.machine() == "yoga"


def read(p, path=None):
    path = path or p.ledger
    if local_ledger(p):
        return Path(path).read_text(encoding="utf-8", errors="replace") if Path(path).exists() else ""
    return _ssh(p, f"Get-Content -Raw {path}").stdout


def append(p, line):
    text = f"\n{line}\n"
    if local_ledger(p):
        with Path(p.ledger).open("a", encoding="utf-8") as f:
            f.write(text)
        return
    # The ledger lives on the Yoga: ship the line as a file, append it there (the way s2_eden did).
    name = f"/tmp/tr-ledger-{p.prefix}-{int(time.time())}.txt"
    Path(name).write_text(text, encoding="utf-8")
    try:
        subprocess.run(["/usr/bin/scp", "-q", "-o", "BatchMode=yes", name, f"{p.ledger_ssh}:{name}"], check=True)
        _ssh(p, f"Add-Content -Path {p.ledger} -Value (Get-Content -Raw {name}) -NoNewline; Remove-Item {name}")
    finally:
        Path(name).unlink(missing_ok=True)


def queue_text(p):
    path = Path(p.queue)
    return path.read_text(encoding="utf-8") if p.queue and path.exists() else ""


def busy(p, text=None, now=None, queue=None):
    """Why this stream may not claim its host now: a list of reasons (empty: free)."""
    text = read(p) if text is None else text
    queue = queue_text(p) if queue is None else queue
    now = time.time() if now is None else now
    me = owner_of(p.owner)
    fresh, stale = open_claims(text, p.host, exclude=me, now=now, stale_hours=p.stale_hours)
    why = [f"({'Eden' if c.eden else 'compile'}) {c.text[:110]}" for c in fresh + stale]
    waited = since_release(text, me, p.host, now)
    if waited is not None and waited < p.fair_seconds:
        why.append(f"fairness: own RELEASED {waited:.0f} s ago, wait {p.fair_seconds - waited:.0f} s more")
    return why + ([] if is_parallel(p.owner) else parallel_turns(text, queue, p.host, now, p.parallel_grace))


def claim(p, body, now=None, text=None, queue=None):
    params.need(p, "owner")
    why = busy(p, text, now, queue)
    if why:
        raise Busy(why)
    line = format_line(p, "CLAIMED", body, now)
    append(p, line)
    return line


def release(p, body, now=None):
    params.need(p, "owner")
    line = format_line(p, "RELEASED", body, now)
    append(p, line)
    return line


class Busy(Exception):
    """The host is not free; `.args[0]` lists the reasons."""


def advice(p):
    """Queue head for this host and whether it names this stream (advisory only)."""
    try:
        head = queue_head(Path(p.queue).read_text(encoding="utf-8"), p.host)
    except FileNotFoundError:
        return None, True
    mine = bool(head) and re.match(rf"^[\s*]*{re.escape(p.stream_name)}\b", head, re.I) is not None
    return head, mine


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    p = params.load()
    op = argv[0] if argv else "open"
    if op == "open":
        fresh, stale = open_claims(read(p), p.host, owner_of(p.owner), stale_hours=p.stale_hours)
        for c in fresh:
            print(f"OPEN on {p.host} ({'Eden' if c.eden else 'compile'}) {c.text[:130]}")
        for c in stale:
            print(f"OPEN on {p.host} (age >= {p.stale_hours:g} h, blocking until RELEASED) {c.owner}: {c.text[:80]}")
        sys.exit(3 if fresh or stale else 0)
    elif op == "free":
        why = busy(p)
        print("\n".join(why) or f"{p.host} free")
        sys.exit(3 if why else 0)
    elif op == "claim":
        try:
            print(claim(p, argv[1]))
        except Busy as exc:
            print("refused:\n  " + "\n  ".join(exc.args[0]))
            sys.exit(3)
    elif op == "release":
        print(release(p, argv[1]))
    elif op == "queue":
        head, mine = advice(p)
        print(f"queue head on {p.host}: {head} ({'this stream' if mine else 'NOT this stream'})")
    elif op == "wait":
        # Interruptible checks of at most 60 s (stream rules); prints one line per change.
        end, last = time.time() + float(argv[1]), None
        while True:
            why = busy(p)
            if not why:
                print(f"{stamp()} {p.host} free", flush=True)
                if len(argv) > 2:
                    print(claim(p, argv[2]))
                return
            if why != last:
                print(f"{stamp()} waiting: {why[0][:110]}", flush=True)
                last = why
            if time.time() > end:
                sys.exit(3)
            time.sleep(min(60, max(1, end - time.time())))
    else:
        sys.exit(f"usage: ledger.py open|free|claim <text>|release <text>|queue|wait <cap s> [<claim text>]")


if __name__ == "__main__":
    main()
