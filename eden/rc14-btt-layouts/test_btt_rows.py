"""Offline check of the btt row hooks against a small simulation of the plugin's observable behaviour (the `[Btt]` lines, the
macros' effect on the cursor and P1, the layouts file). It proves the hooks' own logic (parsing, the closed loops, the
checks and their failure messages); it does NOT prove the plugin: the model is `src/model/btt.rs` re-stated in Python from
the SPEC, so a disagreement between the two is for the Eden boot to find. Run: /usr/bin/python3 -I test_btt_rows.py
"""
import json
import sys
import tempfile
import time
from pathlib import Path

ROWS = Path(__file__).resolve().parent / 'rows'
sys.path.insert(0, str(ROWS))
import btt_rows as rows  # noqa: E402
import fx  # noqa: E402
import steps  # noqa: E402

MAX, APART = 10, 8.0


class Game:
    """Placing / Ready / Running / Done as SPEC V2.5 says, one macro at a time, 60 frames a second of simulated time."""

    def __init__(self, root, stage=0):
        self.root, self.stage = Path(root), stage
        self.log_path = self.root / 'practice-diagnostics.log'
        self.log_path.write_text('')
        self.seq, self.ms = 0, 1000
        self.phase, self.targets, self.cursor = 'Off', [], [0.0, 15.0]
        self.p1, self.facing, self.frames, self.left = [-30.0, 0.0, 0.0], 1, 0, 0
        self.pick, self.msg, self.broken, self.menu, self.row_on = None, ('', 0), set(), False, False
        self.mode_cam, self.shown = 1, None
        self.ask = ('', 0)
        self.files = self.root / 'layouts'
        self.sdmc = self.root

    # -- log ---------------------------------------------------------------------------------------------------------
    def line(self, text):
        self.seq += 1
        self.ms += 16
        with self.log_path.open('a') as f:
            f.write('%d #%d %s\n' % (self.ms, self.seq, text))

    def text(self):
        n, got = len(self.targets), len(self.broken)
        clock = lambda f: (f * 5 // 3 // 6000, f * 5 // 3 // 100 % 60, f * 5 // 3 % 100)  # noqa: E731
        m, s, c = clock(self.frames)
        if self.phase == 'Placing':
            if self.msg[1] > 0 and self.msg[0]:
                return self.msg[0]
            five = self.five()
            tail = 'A place  B undo  X start' if self.pick is None or self.pick >= len(five) else '%d/%d %s: %d' % (
                self.pick + 1, len(five), five[self.pick][1]['name'], len(five[self.pick][1]['targets']))
            return 'Place targets %d/10   %s' % (n, tail)
        if self.phase in ('Ready', 'Running'):
            return 'Targets %d/%d   %d:%02d.%02d' % (got, n, m, s, c)
        if self.phase == 'Done':
            bm, bs, bc = clock(self.best_now)
            return 'Done! %d in %d:%02d.%02d (best %d:%02d.%02d)' % (n, m, s, c, bm, bs, bc)
        return ''

    def emit(self, force=False):
        text = (self.phase, self.text())
        if text == self.shown and not force:
            return
        self.shown = text
        cur = 'None' if self.phase != 'Placing' else 'Some([%.1f, %.1f])' % tuple(self.cursor)
        self.line('[Btt] phase=%s n=%d cursor=%s p1=[%.1f, %.1f, 0.0] text="%s"' % (self.phase, len(self.targets), cur, self.p1[0], self.p1[1], text[1]))

    def camera(self, want):
        mode = 4 if want else 1
        if mode != self.mode_cam:
            self.line('[Btt] camera want=%s mode_before=%d mode_after=%d changed=true' % (str(want).lower(), self.mode_cam, mode))
            self.mode_cam = mode

    # -- time --------------------------------------------------------------------------------------------------------
    def advance(self, frames):
        for _ in range(frames):
            if self.msg[1] > 0 and self.phase == 'Placing':
                self.msg = (self.msg[0], self.msg[1] - 1)
            if self.phase == 'Running':
                self.frames += 1
                if self.frames % 3 == 0:
                    self.emit(True)
            if self.phase == 'Done':
                self.left -= 1
                if self.left <= 0:
                    self.ready()
            self.emit()
        self.camera(self.phase != 'Off' and not self.menu)

    def ready(self):
        self.phase, self.broken, self.frames = 'Ready', set(), 0

    # -- the rows and the menu -----------------------------------------------------------------------------------------
    def set_rows(self, **v):
        if 'btt' in v:
            self.row_on = v['btt'] == [0, 1]
            if not self.row_on and self.phase != 'Off':
                self.phase = 'Off'
            self.emit(True)
            self.camera(False)

    def placing_start(self):
        self.phase, self.broken, self.cursor, self.msg = 'Placing', set(), [0.0, self.p1[1] + 15.0], ('', 0)
        self.emit(True)

    def menu_close(self):
        self.menu = False
        if self.row_on:
            self.placing_start() if (not self.targets or self.place_next) else self.ready()
            self.place_next = False
        self.emit(True)

    place_next = False

    # -- stage and files (btt-5: one file per layout in layouts/, the hot five per stage) ----------------------------
    def change_stage(self, stage):
        self.stage, self.targets, self.pick = stage, [], None
        self.requests = getattr(self, 'requests', 0) + 1  # the game's `id=` is a request counter, not the stage
        self.line('[PracticeStageRequest] queued id=%d selection=Selection { stage: %d, form: Normal } stage=%d' % (self.requests, stage, stage))
        self.listed(False)

    def docs(self, stage=None):
        out = []
        for path in sorted(self.files.glob('*.json')) if self.files.is_dir() else []:
            doc = json.loads(path.read_text())
            if stage is None or doc['stage'] == stage:
                out.append((path.stem, doc))
        return sorted(out, key=lambda kv: (-kv[1]['used'], kv[0]))

    def five(self):
        return self.docs(self.stage)[:5]

    def listed(self, saved):
        mine = self.docs(self.stage)
        self.line('[Btt] layouts stage=%d files=%d hot=%d skipped=0 saved=%s first="%s"'
                  % (self.stage, len(mine), min(len(mine), 5), str(saved).lower(), mine[0][1]['name'] if mine else ''))

    def write(self, file_id, doc):
        self.files.mkdir(exist_ok=True)
        (self.files / ('%s.json' % file_id)).write_text(json.dumps(doc, ensure_ascii=False))

    def next_use(self):
        return max([d['used'] for _, d in self.docs()] + [0]) + 1

    def new_file(self, targets, stage=None):
        stage = self.stage if stage is None else stage
        name = rows.lowest_free([d['name'] for _, d in self.docs(stage)], {0: 'Battlefield', 2: 'Big Battlefield'}[stage])
        base = name.lower().replace(' · ', '-').replace(' ', '-')
        file_id, k = base, 1
        while (self.files / ('%s.json' % file_id)).exists():
            k += 1
            file_id = '%s-%d' % (base, k)
        self.write(file_id, {'schema_version': 1, 'name': name, 'stage': stage, 'used': self.next_use(), 'targets': [list(t) for t in targets]})
        return file_id, name

    def use(self, file_id, name=None):
        doc = json.loads((self.files / ('%s.json' % file_id)).read_text())
        if name:
            doc['name'] = name
        doc['used'] = self.next_use()
        self.write(file_id, doc)

    # -- macros ------------------------------------------------------------------------------------------------------
    def say(self, message, frames=120):
        self.msg = (message, frames)

    def ask_twice(self, key, message):
        yes = self.ask[0] == key and self.ask[1] > 0
        self.ask = ('', 0) if yes else (key, 60)
        if not yes:
            self.say(message, 60)
        return yes

    def play(self, name):
        frames = 20
        if name.startswith('btt-') and name[4] in 'rlud' and name[5:].isdigit():
            n = int(name[5:])
            dx = {'r': 2.0, 'l': -2.0}.get(name[4], 0.0) * n
            dy = {'u': 2.0, 'd': -2.0}.get(name[4], 0.0) * n
            if self.phase == 'Placing':
                self.cursor = [min(135.0, max(-135.0, self.cursor[0] + dx)), min(100.0, max(-10.0, self.cursor[1] + dy))]
            frames = n + 3
        elif name in ('btt-walk-r8', 'btt-walk-l8'):
            self.p1[0] += 5.0 if name.endswith('r8') else -5.0
            self.facing = 1 if name.endswith('r8') else -1
            frames = 11
        elif name in ('jab', 'ftilt', 'sh-nair', 'fsmash'):
            if self.phase == 'Ready':
                self.phase = 'Running'
            reach = {'jab': (6.0, 14.0), 'ftilt': (6.0, 18.0), 'sh-nair': (-8.0, 8.0), 'fsmash': (8.0, 26.0)}[name]
            for i, (tx, _) in enumerate(self.targets):
                if i not in self.broken and self.facing * (tx - self.p1[0]) >= reach[0] * (1 if self.facing > 0 else 1) and abs(tx - self.p1[0]) <= reach[1]:
                    self.broken.add(i)
            frames = 40
        elif name in ('btt-a', 'btt-b', 'btt-y2', 'btt-zl-right', 'btt-zl-left', 'btt-zl-y2', 'chord-save', 'chord-load', 'x-press'):
            frames = self.macro(name)
        elif name == 'reset-center':
            self.p1 = [-30.0, 0.0, 0.0]
            self.facing = 1
            if self.phase in ('Ready', 'Running', 'Done'):
                self.ready()
            frames = 80
        elif name == 'menu-open':
            self.menu = True
            frames = 19
        elif name == 'menu-close':
            self.menu_close()
            frames = 51
        elif self.phase == 'Ready' and name == 'btt-a':
            self.phase = 'Running'
        if self.phase == 'Running' and len(self.broken) == len(self.targets) and self.targets:
            self.phase, self.left = 'Done', 180
            self.best_now = min(getattr(self, 'best_prev', 10 ** 9), self.frames)
            self.best_prev = self.best_now
        self.advance(frames)

    def macro(self, name):
        if self.phase == 'Ready' and name not in ('menu-open',):
            self.phase = 'Running'
            return 5
        if self.phase != 'Placing':
            return 10
        five = self.five()
        chosen = five[self.pick or 0] if five else None
        if name == 'btt-a':
            c = self.cursor
            if len(self.targets) >= MAX:
                self.say('10 targets: B removes the last')
            elif any(((t[0] - c[0]) ** 2 + (t[1] - c[1]) ** 2) ** 0.5 < APART for t in self.targets):
                self.say('Too close to another target')
            else:
                self.targets.append((round(c[0], 1), round(c[1], 1)))
                self.msg = ('', 0)
        elif name == 'btt-b':
            if self.targets:
                self.targets.pop()
        elif name == 'btt-y2':
            self.targets = []
            self.msg = ('', 0)
        elif name == 'x-press':
            if not self.targets:
                self.say('Place at least one target')
            else:
                self.ready()
        elif name in ('btt-zl-right', 'btt-zl-left'):
            if not five:
                self.say('No saved layouts on this stage yet')
            else:
                d = 1 if name.endswith('right') else len(five) - 1
                self.pick, self.msg = (0 if self.pick is None else (self.pick + d) % len(five)), ('', 0)
        elif name == 'chord-save':
            if not self.targets:
                self.say('Place at least one target')
            else:
                _, saved = self.new_file(self.targets)
                self.pick = 0
                self.listed(True)
                self.say('Saved: ' + saved)
        elif name == 'chord-load':
            if chosen is None:
                self.say('No saved layouts on this stage yet')
            else:
                self.targets = [tuple(t) for t in chosen[1]['targets']]
                self.say('Loaded: ' + chosen[1]['name'])
                self.use(chosen[0])
                self.pick = [k for k, _ in self.five()].index(chosen[0])
                self.listed(False)
        elif name == 'btt-zl-y2':
            for _ in range(2):
                if chosen is None:
                    self.say('No saved layouts on this stage yet')
                    break
                if self.ask_twice('del', 'ZL + Y again to delete ' + chosen[1]['name']):
                    (self.files / ('%s.json' % chosen[0])).unlink()
                    self.say('Deleted: ' + chosen[1]['name'])
                    self.pick = None if not self.five() else min(self.pick or 0, len(self.five()) - 1)
                    self.listed(False)
                    break
                self.advance(15)
        return 15

    # -- the Saved layouts screen (the list, a layout's own screen, the delete question; the keyboard) ----------------------
    def panel_open(self):
        self.pane, self.cursor_list, self.sel = 'list', 0, 0

    def labels(self):
        mine = self.docs(self.stage)
        names = rows.list_order(mine)
        return names, [next(k for k, d in mine if d['name'] == n) for n in names]

    def panel_press(self, button):
        names, ids = self.labels()
        if self.pane == 'list':
            if button == 'down':
                self.cursor_list = min(self.cursor_list + 1, len(names) - 1)
            elif button == 'up':
                self.cursor_list = max(self.cursor_list - 1, 0)
            elif button == 'a' and names:
                self.pane, self.own, self.sel = 'own', ids[self.cursor_list], 0
            elif button == 'b':
                self.pane = None
        elif self.pane in ('own', 'ask'):
            if button == 'right':
                self.sel = min(self.sel + 1, 3 if self.pane == 'own' else 1)
            elif button == 'b':
                self.pane, self.sel = ('list', 0) if self.pane == 'own' else ('own', 0)
            elif button == 'a' and self.pane == 'own':
                if self.sel == 0:
                    doc = json.loads((self.files / ('%s.json' % self.own)).read_text())
                    self.targets, self.row_on, self.pane = [tuple(t) for t in doc['targets']], True, None
                    self.use(self.own)
                elif self.sel == 1:
                    self.pane, self.naming = None, self.own
                elif self.sel == 2:
                    doc = json.loads((self.files / ('%s.json' % self.own)).read_text())
                    self.new_file(doc['targets'], doc['stage'])
                    self.pane, self.cursor_list = 'list', 0
                else:
                    self.pane, self.sel = 'ask', 0
            elif button == 'a':
                if self.sel == 1:
                    (self.files / ('%s.json' % self.own)).unlink()
                    self.pane, self.cursor_list = 'list', 0
                else:
                    self.pane, self.sel = 'own', 0
            self.listed(False)

    def view_line(self):
        names, ids = self.labels()
        if self.pane == 'list':
            title, labels, sel = 'Saved layouts', names, 0
        elif self.pane == 'own':
            title, labels, sel = json.loads((self.files / ('%s.json' % self.own)).read_text())['name'], rows.OWN_ROWS, self.sel
        elif self.pane == 'ask':
            title, labels, sel = json.loads((self.files / ('%s.json' % self.own)).read_text())['name'], rows.ASK_ROWS, self.sel
        else:
            return '[NamedMacroDiag] view none visible=false'
        # As the panel reports its tiles (Eden btt5-214653c2-1): each label cut to the tile, padded to 32 tiles.
        labels = [label[:16] for label in labels] + [''] * (32 - len(labels))
        return '[NamedMacroDiag] view title=%s footer="" selected=%d labels=%s' % (
            json.dumps(title, ensure_ascii=False), sel, json.dumps(labels, ensure_ascii=False))

    pane, naming = None, None


class Runner:
    def __init__(self, out):
        self.out = Path(out)


class Ctx:
    def __init__(self, game, root):
        self.game, self.root = game, Path(root)
        self.runner = Runner(root)
        self.row_dir = self.root / 'row'
        self.row_dir.mkdir(exist_ok=True)
        self.instance = {'diag_log': str(game.log_path), 'settings': str(root / 'settings.json')}
        (root / 'settings.json').write_text(json.dumps({'menu': {'btt': [1, 0]}}))
        self.shots = 0

    def play(self, name):
        self.game.play(name)

    def check_stop(self):
        pass

    def gate(self, name):
        pass

    def call(self, *args, **kwargs):
        return {}

    def shot(self, name):
        from PIL import Image
        self.shots += 1
        path = self.row_dir / ('%s.png' % name)
        Image.new('RGB', (64, 36), (40, 40, 40) if self.game.mode_cam == 1 else (200, 120, 40)).save(path)
        return path


def install(game, ctx):
    """The menu and the live settings, as fakes that drive the same game."""
    def set_row_value(on):
        data = json.loads(Path(ctx.instance['settings']).read_text())
        data['menu']['btt'] = [0, 1] if on else [1, 0]
        Path(ctx.instance['settings']).write_text(json.dumps(data))

    fx._live_set = lambda c, **v: (set_row_value(v['btt'] == [0, 1]), game.set_rows(**v))
    steps.open_menu = lambda c: setattr(game, 'menu', True) or game.advance(5)
    steps.close_menu = lambda c: game.menu_close()
    steps.goto_page = lambda c, page: None
    focus = {'row': None}
    steps._focus_row = lambda c, title: focus.update(row=title) or True

    def press(c, button, gap=0.4):
        if game.pane is not None:
            game.panel_press(button)
        elif button == 'a' and focus['row'] == 'Saved layouts' and game.menu:
            game.panel_open()
        elif button == 'a' and focus['row'] == 'Place targets' and game.menu:
            set_row_value(True)
            game.row_on, game.place_next, game.menu = True, True, False
            game.menu_close()
        elif button == 'a' and focus['row'] == 'Change Stage' and game.menu:
            game.menu = False
            game.change_stage(rows.STAGE_IDS[game_key['key']])
    steps.press = press

    def keyboard(c, text, cap=8.0):
        if game.naming is None:
            raise RuntimeError('no keyboard open')
        game.use(game.naming, text)
        game.naming = None
        return {'path': 'sim', 'typed': text}
    rows.cov._panel_view = lambda c: game.view_line()
    rows.cov.keyboard_type = keyboard
    game_key = {'key': 'bf'}
    real_change = rows.change_stage

    def change(c, key='bf'):
        game_key['key'] = key
        return real_change(c, key)
    rows.change_stage = change
    time.sleep = lambda s: game.advance(int(s * 60))
    return game_key


def check(name, fn, ctx, *args, **kwargs):
    try:
        result = fn(ctx, *args, **kwargs)
        print('PASS', name, json.dumps(result, default=str)[:200])
        return True
    except Exception as error:  # noqa: BLE001
        print('FAIL', name, repr(error)[:400])
        return False


def mutations():
    """Each row must FAIL on a plugin that breaks what it checks, and on that check: a Save that replaces the newest file, a
    list that shows another stage's layouts, chords that do not step, a Delete that does not ask first. Each case runs on a
    fresh simulation right after it is built (the fakes patch module state, `time.sleep` included)."""
    def fresh():
        root = tempfile.mkdtemp(prefix='btt-mut-')
        g = Game(root)
        c = Ctx(g, Path(root))
        install(g, c)
        rows.change_stage(c, 'bf')
        return g, c

    def replace_newest(g):
        real_new = g.new_file

        def new_file(targets, stage=None):
            mine = g.docs(g.stage)
            if not mine:
                return real_new(targets, stage)
            file_id, doc = mine[0]
            doc['targets'], doc['used'] = [list(t) for t in targets], g.next_use()
            g.write(file_id, doc)
            return file_id, doc['name']
        g.new_file = new_file

    def other_stage(g):
        g.write('other-stage', {'schema_version': 1, 'name': 'Other stage', 'stage': 2, 'used': 50, 'targets': [[0.0, 0.0]]})
        g.labels = lambda: ([d['name'] for _, d in g.docs()], [k for k, _ in g.docs()])

    def no_step(g):
        real_macro = g.macro

        def macro(name):
            if name in ('btt-zl-right', 'btt-zl-left') and g.five():
                g.pick, g.msg = 0, ('', 0)
                return 15
            return real_macro(name)
        g.macro = macro

    def no_question(g):
        real_ask = g.ask_twice
        g.ask_twice = lambda key, message: True if key == 'del' else real_ask(key, message)

    cases = [('a Save replaces the newest file', replace_newest, rows.e41, ('a Save made', 'ZL + Down saves')),
             ('the list shows another stage', other_stage, rows.e41, ('the list',)),
             ('the chords do not step', no_step, rows.x_saves, ('the chords stepped', 'ZL + Right: not seen')),
             ('ZL + Y deletes without asking', no_question, rows.x_saves, ('no question before the delete',))]
    ok = True
    for name, mutate, fn, why in cases:
        g, c = fresh()
        mutate(g)
        try:
            fn(c, key='bf')
            print('MUTATION SURVIVED', name)
            ok = False
        except Exception as error:  # noqa: BLE001
            killed = any(w in str(error) for w in why)
            ok &= killed
            print('MUTATION %s' % ('KILLED' if killed else 'KILLED FOR ANOTHER REASON'), name, repr(error)[:160])
    return ok

def main():
    root = tempfile.mkdtemp(prefix='btt-sim-')
    game = Game(root)
    ctx = Ctx(game, Path(root))
    install(game, ctx)
    ok = True
    # Parsing.
    st = rows.parse('12 #3 [Btt] phase=Placing n=3 cursor=Some([12.0, 15.0]) p1=[-30.5, 0.0, 0.0] text="Place targets 3/10   A place  B undo  X start"')
    assert st == {'phase': 'Placing', 'n': 3, 'cursor': (12.0, 15.0), 'p1': (-30.5, 0.0, 0.0), 'text': 'Place targets 3/10   A place  B undo  X start'}, st
    assert rows.parse('[Btt] phase=Off n=0 cursor=None text=""')['cursor'] is None
    assert rows.hundredths('0', '18', '27') == 1827 and rows.count_of('Targets 1/3   0:00.51') == (1, 3)
    # E29's clock checks from a run's states (btt-4): exact within the log's sampling; each violation is caught.
    run_ = lambda r, d, after='Ready': [{'phase': 'Running', 'text': 'Targets 2/3   0:0%d.%02d' % divmod(r, 100)},
                                        {'phase': 'Done', 'text': 'Done! 3 in 0:0%d.%02d (best 0:0%d.%02d)' % (divmod(d, 100) + divmod(d, 100))},
                                        {'phase': after, 'text': 'Targets 0/3   0:00.00'}]
    assert rows.clock_stop_problems(run_(398, 408), True) == []
    assert rows.clock_stop_problems(run_(391, 408), True) == []
    assert rows.clock_stop_problems(run_(390, 408), True)[0].startswith('(b)')
    assert rows.clock_stop_problems(run_(409, 408), True)[0].startswith('(a)')
    assert rows.clock_stop_problems(run_(398, 408, 'Running'), True)[0].startswith('(c)')
    wrong_best = run_(398, 408)
    wrong_best[1]['text'] = 'Done! 3 in 0:04.08 (best 0:03.00)'
    assert rows.clock_stop_problems(wrong_best, True)[0].startswith('(d)') and rows.clock_stop_problems(wrong_best, False) == []
    ctx.game.line('noise')
    sequence = [('btt-st-bf', rows.change_stage, {'key': 'bf'}), ('btt-E26m', rows.e26_menu, {}), ('btt-E26', rows.e26, {}),
                ('btt-E28', rows.e28, {}), ('btt-E29', rows.e29, {}), ('btt-E41', rows.e41, {'key': 'bf'}), ('btt-E43', rows.e43, {'key': 'bf'}),
                ('btt-E31a', rows.e31, {'key': 'bf'}), ('btt-G3', lambda c: (rows.g3_prep(c), rows.g3_action(c))[1], {})]
    # E42 needs the seeded layout (the fixture's content) in the simulated profile before the stage opens.
    import make_btt_fixture as fixture
    game.files.mkdir(exist_ok=True)
    (game.files / ('%s.json' % fixture.FILE_ID)).write_text(json.dumps(fixture.build(), ensure_ascii=False))
    sequence += [('btt-st-bbf', rows.change_stage, {'key': 'bbf'}), ('btt-E42', rows.e42, {'key': 'bbf'}), ('btt-E31b', rows.e31, {'key': 'bbf'}),
                 ('btt-X-stages', rows.x_stages, {'first': 'bf', 'second': 'bbf'}), ('btt-X-saves', rows.x_saves, {'key': 'bf'}),
                 ('btt-off', rows.btt_off, {})]
    for name, fn, kwargs in sequence:
        ok &= check(name, fn, ctx, **kwargs)
    # The checks fail when the plugin does what the rows forbid (mutations of the simulation).
    ok &= mutations()
    print('ALL PASS' if ok else 'SOME FAILED')
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
