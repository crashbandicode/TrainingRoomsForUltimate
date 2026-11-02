"""TrainingRooms package-2 Ally adapter: a copy of the Ally owned-rig adapters (W20's w20.py, the m5q wa.py; lineage W8/W5/W11 and the Windows port in
research/named-macros/win) with this stream's owned root C:\\Eden\\trainingrooms-pkg2, owner text and extra ops. Runs ON the Ally with the
Windows Python 3.12 (standard library only; Pillow is there for image checks).

  wa_s2.py launch PHASE             bounded Job launcher (run from a scheduled task, console-less)
  wa_s2.py ops PHASE BASE64JSON     list of ops, each reply appended to runs\\PHASE\\ops.jsonl, JSON printed
  wa_s2.py kill PHASE               end the run through its Job object (Eden hangs at power-off on this rig)
Ops (as W20's: call, status, sleep, gshot, button, hold, stick, play, mark, waitlog, waitlog_opt, count, waitstatus) plus:
  ["shot", name]                    ui.screenshot target=screen (the whole 1920x1080 screen) -> runs\\PHASE\\shots\\name.png
  ["macro", name_or_path]           play macros\\<name>.edenmacro.json (or an absolute path) and wait for it to finish
  ["plug", relpath, regex, n]       the last n lines of sdmc\\ultimate\\<relpath> matching regex (the plugin's own logs)
  ["waitplug", relpath, regex, s]   wait until a line of that file matches (after the lines present at op start), error on timeout
  ["settings", localpath]           write that settings file as sdmc\\ultimate\\TrainingRooms\\diag-settings-next.json (tmp + rename)
  ["mute"]                          settings.set core.volume 0, settings.save, settings.get (the operator rule: muted before the game boots)
  ["load"]                          game.load of the pinned xci
"""
from pathlib import Path
import base64, hashlib, json, os, re, sys, time

# The rig root: TR_ALLY_ROOT, else the folder above this file (it lives in <root>\tr-pkg2\, ally_conf.py). Restart rules 2026-11-02: on the
# Ally only the lent rig C:\Eden\c1162-w5-ally may run; the owned root C:\Eden\trainingrooms-pkg2 makes Windows prompt for the firewall.
O = Path(os.environ.get('TR_ALLY_ROOT') or Path(__file__).resolve().parent.parent); PROFILE = O / 'profile'; USER = PROFILE / 'user'
OWNER = 'trainingrooms-pkg2-0caa72b8-sonnet55'
EDEN_SHA = '45af318c93cbc00a2794421ee884ffbcb9dbec75bbdbdeef27af30d61cd2cbad'
DURATION = 3600
MEM_GIB = 8


def sha(p):
    with Path(p).open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def retry_manifest(fn, *args, **kwargs):
    for attempt in range(21):
        try:
            return fn(*args, **kwargs)
        except PermissionError:
            if attempt == 20:
                raise
            time.sleep(.01)


def modules():
    sys.path.insert(0, str(O / 'runner'))
    import winplat, winjob, winaccept, winlaunch
    if not hasattr(winplat, '_park_read_manifest'):
        winplat._park_read_manifest = winplat.read_manifest
        winplat.read_manifest = lambda *a, **k: retry_manifest(winplat._park_read_manifest, *a, **k)
    winplat.configure(O, USER); winplat.EDEN = PROFILE / 'eden.exe'; winplat.EDEN_SHA256 = EDEN_SHA
    winplat.EDEN_WORKDIR = PROFILE
    return winplat, winjob, winaccept, winlaunch


def owned():
    assert (O / 'CLAIM').read_text().strip() == OWNER


def launch(phase):
    owned(); p, j, a, l = modules(); m = j.system_memory()
    assert not p.processes(), p.processes()
    assert m['available_bytes'] > 10 * 1024 ** 3 and m['commit_available_bytes'] > 8 * 1024 ** 3, m
    assert sha(PROFILE / 'eden.exe') == EDEN_SHA and not (O / 'runs' / phase).exists()
    for key in list(os.environ):
        if key == 'EDEN_NSO_LAN_CONFIG' or key.startswith('EDEN_LAN_'):
            os.environ.pop(key)
    os.environ['EDEN_WIN_ROOT'] = str(O); os.environ['EDEN_WIN_PROFILE'] = str(USER)
    os.chdir(PROFILE)
    with (O / (phase + '-launcher.log')).open('x') as log:
        sys.stdout = log; sys.stderr = log
        try:
            raise SystemExit(l.main(['--phase', phase, '--eden', str(PROFILE / 'eden.exe'), '--memory-gib', str(MEM_GIB),
                                     '--runtime', str(DURATION), '--normal-stop', str(DURATION - 180),
                                     '--startup-timeout', '180', '--hang-timeout', '90', '--stall-timeout', '0',
                                     '--guard-available-gib', '3']))
        except BaseException:
            import traceback; traceback.print_exc(); raise


def kill(phase):
    owned(); p, j, a, l = modules()
    run = json.loads((O / 'runs' / phase / 'run.json').read_text())
    pids = [x for x in (run.get('pid'), run.get('launcher_pid')) if x]
    h, name = p.open_job(run['unit'], pids, j.JOB_OBJECT_QUERY | j.JOB_OBJECT_TERMINATE)
    print('unit', run['unit'], 'job', name, 'pids before', j.pids(h) if h else None)
    if h:
        j.terminate(h, 137); j.close(h)
    for _ in range(100):
        if not j.process_alive(run['pid']):
            break
        time.sleep(0.2)
    print('alive after:', j.process_alive(run['pid']))


def call(phase, method, params=None, timeout=30):
    owned(); p, j, a, l = modules()
    r = json.loads(retry_manifest((O / 'runs' / phase / 'run.json').read_text))
    assert r['state'] == 'ready' and not r.get('ended_at') and p.process_alive(r['pid'])
    assert r['command'][0] == str(PROFILE / 'eden.exe') and r['memory_max_bytes'] == MEM_GIB * 1024 ** 3
    return p.call(phase, method, params or {}, timeout=timeout)


def finish(phase, seconds=90):
    end = time.monotonic() + seconds
    try:
        while True:
            status = call(phase, 'macro.status'); assert not status.get('error'), status
            if status['state'] == 'finished':
                return status
            assert time.monotonic() < end, status
            time.sleep(.2)
    finally:
        call(phase, 'macro.stop'); call(phase, 'input.reset')


def play(phase, name):
    path = O / 'macros' / (name + '.edenmacro.json'); assert path.is_file()
    st = call(phase, 'status'); assert st['running'] and not st['modal'], st
    assert call(phase, 'macro.status')['state'] in ('idle', 'finished')
    call(phase, 'macro.play', {'path': str(path)})
    return finish(phase)


def ops(phase, items):
    run = O / 'runs' / phase; shots = run / 'shots'; shots.mkdir(exist_ok=True)
    receipt = run / 'ops.jsonl'; out = []
    clog = run / 'console.log'
    mark = {'pos': clog.stat().st_size if clog.exists() else 0}

    def rec(e):
        e['at'] = time.time()
        with receipt.open('a') as f:
            f.write(json.dumps(e) + '\n')
        out.append(e)

    try:
        for op in items:
            k = op[0]
            if k == 'call':
                r = call(phase, op[1], op[2] if len(op) > 2 else {})
                rec({'op': op, 'result': r if len(json.dumps(r)) < 1500 else str(r)[:1500]})
            elif k == 'status':
                rec({'op': op, 'result': call(phase, 'status')})
            elif k == 'play':
                st = play(phase, op[1]); rec({'op': op, 'status': {x: st.get(x) for x in ('state', 'frame', 'duration_frames')}})
            elif k == 'sleep':
                time.sleep(op[1])
            elif k == 'mark':
                mark['pos'] = clog.stat().st_size if clog.exists() else 0
                rec({'op': op, 'pos': mark['pos']})
            elif k == 'waitlog':
                pat = re.compile(op[1]); end = time.monotonic() + (op[2] if len(op) > 2 else 60); hit = None
                pos = mark['pos']   # offset of the first byte not yet examined as part of a complete line
                while hit is None:
                    with clog.open('rb') as f:
                        f.seek(pos); data = f.read()
                    off = 0
                    while True:
                        nl = data.find(b'\n', off)
                        if nl < 0:
                            break
                        line = data[off:nl].decode('utf-8', 'replace'); off = nl + 1
                        if pat.search(line):
                            hit = line; break
                    pos += off   # the next waitlog starts right after the hit line (or after the last complete line)
                    if hit is None:
                        if time.monotonic() > end:
                            rec({'op': op, 'error': 'timeout'})
                            raise SystemExit('waitlog timeout: ' + op[1])
                        time.sleep(0.25)
                mark['pos'] = pos
                rec({'op': op, 'line': hit[:300]})
            elif k == 'waitlog_opt':
                # W20: like waitlog, but a timeout is recorded and the batch goes on (a line that may already have passed)
                pat = re.compile(op[1]); end = time.monotonic() + (op[2] if len(op) > 2 else 60); hit = None
                pos = mark['pos']; timed_out = False
                while hit is None and not timed_out:
                    with clog.open('rb') as f:
                        f.seek(pos); data = f.read()
                    off = 0
                    while True:
                        nl = data.find(b'\n', off)
                        if nl < 0:
                            break
                        line = data[off:nl].decode('utf-8', 'replace'); off = nl + 1
                        if pat.search(line):
                            hit = line; break
                    pos += off
                    if hit is None:
                        if time.monotonic() > end:
                            timed_out = True
                        else:
                            time.sleep(0.25)
                mark['pos'] = pos
                rec({'op': op, 'line': (hit[:300] if hit else None), 'timeout': timed_out})
            elif k == 'waitstatus':
                # tm-m5q-eden-a: ["waitstatus", key, value, timeout_s] polls Eden `status` until result[key] == value (e.g. loading_complete true)
                end = time.monotonic() + (op[3] if len(op) > 3 else 120); st = {}
                while True:
                    st = call(phase, 'status')
                    if st.get(op[1]) == op[2]:
                        break
                    if time.monotonic() > end:
                        rec({'op': op, 'error': 'timeout', 'last': {op[1]: st.get(op[1])}})
                        raise SystemExit('waitstatus timeout: ' + op[1])
                    time.sleep(0.5)
                rec({'op': op, 'ok': True})
            elif k == 'count':
                pat = re.compile(op[1])
                n = sum(1 for line in clog.read_text(errors='replace').splitlines() if pat.search(line))
                rec({'op': op, 'count': n})
            elif k == 'gshot':
                # Eden's own GIF recorder (renderer read-back, unaffected by windows covering the desktop)
                path = shots / (op[1] + '.gif'); fps = op[2] if len(op) > 2 else 2
                width = op[3] if len(op) > 3 else 1280; dur = op[4] if len(op) > 4 else 600
                t0 = time.time()
                call(phase, 'recording.start', {'path': str(path), 'fps': fps, 'width': width, 'duration_ms': dur})
                st = {}
                for _ in range(int(dur / 100) + 150):
                    st = call(phase, 'recording.status')
                    if st.get('state') in ('finished', 'error', 'idle'):
                        break
                    time.sleep(0.1)
                rec({'op': op, 'path': str(path), 't0': t0, 'state': st.get('state'), 'frames': st.get('frames'),
                     'error': st.get('error')})
            elif k == 'button':
                call(phase, 'input.button', {'button': op[1], 'state': 'tap', 'duration_ms': op[2] if len(op) > 2 else 100,
                                              'player': 0, 'lease_ms': max(1000, (op[2] if len(op) > 2 else 100) + 300)})
                rec({'op': op}); time.sleep(op[3] if len(op) > 3 else 0.25)
            elif k == 'hold':
                for b in op[1]:
                    call(phase, 'input.button', {'button': b, 'state': 'press', 'player': 0, 'lease_ms': op[2] + 400})
                time.sleep(op[2] / 1000)
                for b in op[1]:
                    call(phase, 'input.button', {'button': b, 'state': 'release', 'player': 0})
                rec({'op': op}); time.sleep(op[3] if len(op) > 3 else 0.3)
            elif k == 'stick':
                call(phase, 'input.stick', {'stick': op[1], 'x': op[2], 'y': op[3], 'player': 0,
                                            'lease_ms': max(1000, op[4] + 300)})
                time.sleep(op[4] / 1000)
                call(phase, 'input.stick', {'stick': op[1], 'x': 0, 'y': 0, 'player': 0, 'lease_ms': 1000})
                rec({'op': op}); time.sleep(op[5] if len(op) > 5 else 0.25)
            elif k == 'shot':
                path = shots / (op[1] + '.png')
                r = call(phase, 'ui.screenshot', {'path': str(path), 'target': 'screen'}, timeout=60)
                rec({'op': op, 'path': str(path), 'result': r if len(json.dumps(r)) < 400 else str(r)[:400]})
            elif k == 'macro':
                name = op[1]
                path = Path(name) if (':' in name or name.startswith('/')) else O / 'macros' / (name + '.edenmacro.json')
                assert path.is_file(), path
                st = call(phase, 'status'); assert st['running'] and not st['modal'], st
                call(phase, 'macro.play', {'path': str(path)})
                st = finish(phase, op[2] if len(op) > 2 else 120)
                rec({'op': op, 'sha256': sha(path)[:16], 'status': {x: st.get(x) for x in ('state', 'frame', 'duration_frames')}})
            elif k == 'plug':
                f = USER / 'sdmc' / 'ultimate' / op[1]
                pat = re.compile(op[2]); got = [x for x in (f.read_text(errors='replace').splitlines() if f.exists() else []) if pat.search(x)]
                rec({'op': op, 'count': len(got), 'lines': got[-(op[3] if len(op) > 3 else 5):]})
            elif k == 'waitplug':
                f = USER / 'sdmc' / 'ultimate' / op[1]; pat = re.compile(op[2]); end = time.monotonic() + (op[3] if len(op) > 3 else 60)
                before = len(f.read_text(errors='replace').splitlines()) if f.exists() else 0; hit = None
                while hit is None:
                    for x in (f.read_text(errors='replace').splitlines()[before:] if f.exists() else []):
                        if pat.search(x):
                            hit = x; break
                    if hit is None:
                        if time.monotonic() > end:
                            rec({'op': op, 'error': 'timeout'}); raise SystemExit('waitplug timeout: ' + op[2])
                        time.sleep(0.25)
                rec({'op': op, 'line': hit[:300]})
            elif k == 'settings':
                sdir = USER / 'sdmc' / 'ultimate' / 'TrainingRooms'; sdir.mkdir(parents=True, exist_ok=True)
                tmp = sdir / '.diag-settings-next.tmp'; tmp.write_bytes(Path(op[1]).read_bytes()); os.replace(tmp, sdir / 'diag-settings-next.json')
                rec({'op': op, 'sha256': sha(Path(op[1]))[:16]})
            elif k == 'mute':
                rec({'op': op, 'set': call(phase, 'settings.set', {'key': 'core.volume', 'value': 0}), 'save': call(phase, 'settings.save'),
                     'get': call(phase, 'settings.get', {'key': 'core.volume'})})
            elif k == 'load':
                import winplat as _wp
                rec({'op': op, 'result': call(phase, 'game.load', {'path': str(_wp.GAME)}, timeout=90)})
            else:
                raise SystemExit('unknown op ' + k)
    finally:
        try:
            call(phase, 'input.reset')
        except Exception as e:
            out.append({'input_reset_error': str(e)})
        print(json.dumps(out)[-5000:])


if __name__ == '__main__':
    op, phase = sys.argv[1:3]
    if op == 'launch':
        launch(phase)
    elif op == 'ops':
        ops(phase, json.loads(base64.b64decode(sys.argv[3])))
    elif op == 'kill':
        kill(phase)
    else:
        raise SystemExit('unknown operation')
