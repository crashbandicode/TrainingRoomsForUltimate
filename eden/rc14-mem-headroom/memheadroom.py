"""rc.14 mem-headroom (2026-11-15): the give-back row, one short targeted boot (the primary's grant).

mh_pyra_pause: the operator's Loading settings (Ready Fighters 8, Ready Fighter Bodies 3, Cycle Preload 5, Ready Hats 5)
and P1 Kirby (a lane of Kirby alone) with his five hats; P1 on Kirby, so the created helper entries fill with his hats
(P-HOT §5.1); a pause frees the groups that left the cycles. The CPU's lane is Mii Gunner > Diddy Kong, then Pyra / Mythra
joins it (Mii Gunner > Pyra > Diddy Kong, a2-pyra-keep-all's lane), which fits Ready Fighters 8 beside P1's two bodies. The Pyra / Mythra group is held for memory by the helpers' room: when the process
would still hold it, a diag memory reserve (`diag-memory-reserve.req`, `process_memory::DIAG_RESERVE`) leaves it
`SHORT` bytes short, a shortfall the idle helpers cover. Pass, in order:
- the group held for memory (`[GroupSwap] held owner=1 ... kinds=[5a, 5b]`);
- the give-back marks idle helpers (`[GroupSwap] give-back owner=1 ... marked=`) and the notice says "load after you pause";
- one Training pause (Plus, Plus): the pause retires them (`[GroupSwap] give-back retire ... retired=true`), then the
  Pyra / Mythra group builds (`background built owner=1 ... ok=true ... kinds=[5a, 5b]`);
- `[MemoryCensus]` lines; no StallWatch; no Unmapped access in the row (`trace.unmapped_access`), z-boot reads the console.
mh_restore: the reserve 0, the remembered values (cycle-fix's restore)."""
import re
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import hotslots as hs  # noqa: E402
import gna_p2_fs as fs  # noqa: E402
import fx  # noqa: E402
import cyclefix as cf  # noqa: E402
import afull as af  # noqa: E402
import steps  # noqa: E402
from gna import _begin, _mark, _shot, _finish  # noqa: E402

PYRA = 0x5a
MB = 1 << 20
# The Pyra / Mythra group's admission need (Eden smoke2r-2 #6769: "< 237076480 bytes"; memory.rs: the set's cost with
# the 12.5 % margin and the 16 MiB floor).
AEGIS_NEED = 237076480
SHORT = 24 * MB
RESERVE = 'diag-memory-reserve.req'
CREATED_READY = re.compile(r'\[CharacterSources\] ready slot=([4-7]) kind=(0x[0-9a-f]+)')
HELD = re.compile(r'\[GroupSwap\] held owner=1 group=\d+ reason=process headroom .*kinds=\[5a, 5b\]')
MARKED = re.compile(r'\[GroupSwap\] give-back owner=1 group=\d+ shortfall=(\d+) marked=(0b[01]+)')
NOTICE = re.compile(r'\[PreloadStatus\] state=\w+ .*text="[^"]*load after you pause\)"')
RETIRED = re.compile(r'\[GroupSwap\] give-back retire slot=(\d) entry=\S+ kind=(0x[0-9a-f]+) retired=(true|false)')
AT_PAUSE = re.compile(r'\[GroupSwap\] give-back at pause marked=(0b[01]+) idle=(0b[01]+) cycle_waits=(true|false) '
                      r'install=(true|false) retired=(0b[01]+)')
BUILT = re.compile(r'\[GroupSwap\] background built owner=1 group=\d+ ok=true reason=\S+ kinds=\[5a, 5b\]')
CENSUS = re.compile(r'\[MemoryCensus\] ')
CENSUS_CREATED = re.compile(r'([4-7]):(0x[0-9a-f]+) ready')
PLAN = re.compile(r'\[GroupSwap\] plan owner=1 group=\d+ kinds=\[5a, 5b\]')
HEADROOM_ANY = re.compile(r'headroom=(?:Some\()?(\d+)\)?(?:->(?:Some\()?(\d+))?')


def _reserve(ctx, bytes_):
    """Write `diag-memory-reserve.req` and wait for the plugin's `state diag_memory_reserve` line."""
    seq = hs.diag_mark(ctx)
    path = Path(ctx.instance['diag_log']).with_name(RESERVE)
    tmp = path.with_name(RESERVE + '.tmp')
    tmp.write_text('%d\n' % bytes_)
    tmp.replace(path)
    return fs._line(ctx, seq, r'\[TrainingRooms\] state diag_memory_reserve bytes=%d' % bytes_, 10)


def _unmapped(ctx):
    got = ctx.call('trace.unmapped_access', {})
    try:
        return int(got.get('total')) if isinstance(got, dict) else None
    except (TypeError, ValueError):
        return None


def _headroom(ctx):
    """The newest process headroom the plugin printed, bytes: any `headroom=` (census, ready, prefetch, memory, a free or a
    purge, whose `->` value is the one after it)."""
    for _, _, t in reversed(hs.diag_lines(ctx)):
        m = HEADROOM_ANY.search(t)
        if m:
            return int(m.group(2) or m.group(1))
    return None


def _wait(ctx, seq, regex, cap):
    found = hs.wait_diag(ctx, seq, regex.pattern, cap)
    return found[2][:400] if found else None


def mh_pyra_pause(ctx):
    fs._remember_original(ctx)
    t0, out = _begin(ctx)
    problems = []
    cf._diag_cycle(ctx, 'off')
    # A retry starts from no reserve (a reserve stays in the plugin until it is written again).
    out['reserve_cleared'] = _reserve(ctx, 0)
    out['unmapped_before'] = _unmapped(ctx)
    seq0 = hs.diag_mark(ctx)
    # 1. The operator's settings, P1 Kirby with his hats (a short P1 lane, so Pyra / Mythra's two bodies fit Ready
    #    Fighters 8 and are planned, not cold), the CPU without Pyra yet.
    # The cycle rows' base values (`_preload_base`: Group Swap and every swap group on, both preload sources, Cycle Preload
    # 5, Ready Hats 5, Bodies 3); boot mh-14ae232f-y1's first pass ran on the card fixture's Group Swap Off.
    fx._live_set(ctx, **fs._preload_base(player_roster=fs._roster([fs.KIRBY]), player_kirby_hats=fs._roster(fs.HATS),
                                         cpu_roster=fs._roster([fs.GUNNER, fs.DIDDY]), cycle_preload_ready_fighters=fs.RF(8)))
    out['settle0'] = fs._settled(ctx, seq0, 120)
    out['to_kirby'] = af._to(ctx, 0, fs.KIRBY, 4)
    out['p1_now'] = fs._live_kinds(ctx).get(0)
    # The CPU's first swap builds its bank ("swap the CPU to load 1 more"): its groups are planned only then.
    out['cpu_to_gunner'] = af._to(ctx, 1, fs.GUNNER, 4)
    out['cpu_now'] = fs._live_kinds(ctx).get(1)
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    # A pause frees the groups that left the cycles, so the later hold finds no parked group (parked groups go first).
    steps.press(ctx, 'plus', 3.0)
    steps.press(ctx, 'plus', 2.0)
    time.sleep(3.0)
    out['preclean'] = [t[:200] for t in af._since(ctx, seq0) if '] pause frees ' in t][:6]
    # 2. The created helpers fill with Kirby's hats (up to 90 s for at least two). Ready lines of the whole diag window: a
    #    helper loaded before this row (a retry) prints none again.
    deadline = time.monotonic() + 90
    created = {}
    while time.monotonic() < deadline:
        ctx.check_stop()
        created = {m.group(1): m.group(2) for _, _, t in hs.diag_lines(ctx) for m in [CREATED_READY.search(t)] if m}
        if len(created) >= 2:
            time.sleep(5.0)
            created = {m.group(1): m.group(2) for _, _, t in hs.diag_lines(ctx) for m in [CREATED_READY.search(t)] if m}
            break
        time.sleep(1.0)
    out['census_filled'] = [t[:600] for t in af._since(ctx, seq0) if CENSUS.search(t)][-2:]
    # Boot mh-14ae232f-y1: the ready lines rotate out of the diag window within minutes; the newest census lists the
    # created helpers too (`created=[4:0x18 ready, ...]`).
    for t in [t for _, _, t in hs.diag_lines(ctx) if CENSUS.search(t)][-1:]:
        for slot, kind in CENSUS_CREATED.findall(t.split('created=[', 1)[-1].split(']', 1)[0]):
            created.setdefault(slot, kind)
    out['created_ready'] = created
    if len(created) < 2:
        problems.append('precondition: the created helpers did not fill (ready slots %s)' % created)
    # 3. Short of the Pyra / Mythra group's room by SHORT: the helpers hold it (a reserve when the process would not).
    out['attempts'] = []
    for attempt in range(2):
        # Boot mh-14ae232f-y1 retry 5: the newest headroom line was stale and the group fit at once. The second attempt
        # drops Pyra, pauses (its free prints the headroom after it) and adds it again against that value.
        headroom = _headroom(ctx)
        reserve = max(0, (headroom or 0) - AEGIS_NEED + SHORT)
        record = {'headroom_before': headroom, 'reserve': reserve, 'reserve_line': _reserve(ctx, reserve)}
        seq = hs.diag_mark(ctx)
        fx._live_set(ctx, cpu_roster=fs._roster([fs.GUNNER, PYRA, fs.DIDDY]))
        record['held'] = _wait(ctx, seq, HELD, 60)
        record['early_build'] = [t[:300] for t in af._since(ctx, seq) if BUILT.search(t)][:1]
        out['attempts'].append(record)
        if record['held'] or attempt:
            break
        fx._live_set(ctx, cpu_roster=fs._roster([fs.GUNNER, fs.DIDDY]))
        time.sleep(2.0)
        steps.press(ctx, 'plus', 3.0)
        steps.press(ctx, 'plus', 2.0)
        time.sleep(3.0)
    out.update({k: out['attempts'][-1][k] for k in ('headroom_before', 'reserve', 'reserve_line', 'held', 'early_build')})
    out['marked'] = _wait(ctx, seq, MARKED, 30)
    out['notice'] = _wait(ctx, seq, NOTICE, 30)
    out['plan'] = [t[:240] for t in af._since(ctx, seq) if PLAN.search(t)][:2]
    _shot(ctx, out, 'shot_before_pause', 'mh-before-pause')
    if not out['held']:
        problems.append('precondition: the Pyra / Mythra group was never held for memory (built early: %s)' % out['early_build'])
    if not out['marked']:
        problems.append('no give-back mark for the held group')
    if not out['notice']:
        problems.append('the notice never said "load after you pause"')
    # 4. One pause: the marked helpers go, then the group builds.
    pause = hs.diag_mark(ctx)
    steps.press(ctx, 'plus', 3.0)
    steps.press(ctx, 'plus', 2.0)
    out['at_pause'] = _wait(ctx, pause, AT_PAUSE, 20)
    out['retired'] = [t[:300] for t in af._since(ctx, pause) if RETIRED.search(t)]
    out['built'] = _wait(ctx, pause, BUILT, 120)
    out['settle_after'] = fs._settled(ctx, pause, 90)
    _shot(ctx, out, 'shot_after', 'mh-after-pause')
    lines = af._since(ctx, seq0)
    # A census follows a `[BodyBank] ready` (a swap); a pass with no swap after its mark has none of its own (retry 4 of boot
    # mh-14ae232f-y1), so the newest ones of the diag window count.
    out['census'] = [t[:600] for _, _, t in hs.diag_lines(ctx) if CENSUS.search(t)][-6:]
    out['stalls'] = [t[:200] for t in lines if 'StallWatch' in t][-3:]
    out['bad'] = af._bad(lines)
    out['unmapped_after'] = _unmapped(ctx)
    _mark(t0, out, 'done')
    m = AT_PAUSE.search(out['at_pause'] or '')
    if not m or m.group(5) == '0b0':
        problems.append('the pause retired no helper: %s' % out['at_pause'])
    if not any(RETIRED.search(t) and RETIRED.search(t).group(3) == 'true' for t in out['retired']):
        problems.append('no retire succeeded: %s' % out['retired'])
    if not out['built']:
        problems.append('the Pyra / Mythra group did not build after the pause (settle %s)' % out['settle_after'])
    if not out['census']:
        problems.append('no [MemoryCensus] line')
    if out['stalls']:
        problems.append('StallWatch: %s' % out['stalls'][-1])
    if out['unmapped_before'] is None or out['unmapped_after'] != out['unmapped_before']:
        problems.append('Unmapped accesses during the row: %s -> %s' % (out['unmapped_before'], out['unmapped_after']))
    problems += ['bad line: %s' % t for t in out['bad']]
    return _finish(ctx, 'mh-pyra-pause', out, problems)


def mh_restore(ctx):
    _reserve(ctx, 0)
    cf._diag_cycle(ctx, 'off')
    return cf.cf_restore(ctx)
