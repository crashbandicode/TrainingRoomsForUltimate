"""rc.14 wind-articles (11-15): Windboxes alone (the checklist's sixth item) at Eden speed 25 %, stills per take, on the diag build
with the `[Legend] object` probe lines (diag only: a tracked article's or item's live slots, `nr=` the no-reaction byte, `kb=` the
knockback words, and its wind areas).

- w-krool: King K. Rool's Blunderbuss held (the vacuum: a pull windbox, plus grab boxes) and Crownerang (a small windbox on his arm
  while he throws). Fighter slots: the check is the stills and a drawn-pill line.
- w-articles: wind areas that run in an ARTICLE's script, drawn since this lane (the four AREA_WIND_2ND* hooks track it): Samus's bomb,
  Ness's PK Fire pillar, Palutena's Explosive Flame. The check: per take, a `[Legend] object ... areas=<n>` line with n > 0 and a
  drawn-pill line.
- w-items: the settle rows. Sheik's Burst Grenade and R.O.B.'s Gyro are items (ITEM_KIND_EXPLOSIONBOMB, ITEM_KIND_ROBOTGYRO): the
  item scan tracks one with a live slot or a wind area, and the probe lines say what it holds. The check: the stills and that the
  item was tracked (`[Hitbox] item tracked` or a `[Legend] object` line); what the lines show is the finding, not a gate.
Kinds from src/model/swap/trio.rs: Samus 0x03, Ness 0x0a, Sheik 0x10, R.O.B. 0x2d, Palutena 0x36, King K. Rool 0x45."""
import re
import time
from pathlib import Path

import fx
import hotslots as hs
from gna import _begin, _finish
from hblegend import ACTIONS, _fighter
from hblfinal import _burst, _speed, _steps

SAMUS, NESS, SHEIK, ROB, PALUTENA, KROOL = 0x03, 0x0a, 0x10, 0x2d, 0x36, 0x45
WIND_ONLY = [0, 0, 0, 0, 0, 1]
KEEP = re.compile(r'\[Legend\] object|\[Hitbox\] (orbs (drawn|pass)|item tracked)|\[CharacterRoster\] HUD refresh owner=0')
DRAWN = re.compile(r'\[Hitbox\] orbs drawn n=(\d+)')
AREAS = re.compile(r'\[Legend\] object id=0x[0-9a-f]+ status=-?\d+ slots=\d+ areas=([1-9]\d*)')
TRACKED = re.compile(r'\[Hitbox\] item tracked|\[Legend\] object id=')


def setup(ctx):
    _, out = _begin(ctx)
    fx._live_set(ctx, hitbox_vis=WIND_ONLY, mash_state=[0] * ACTIONS)
    return out


def _take(ctx, out, tag, move, at, need):
    _burst(ctx, out, tag, move, at, keep=KEEP)
    out[tag]['expected_stills'] = len(at)
    out[tag]['need'] = need


def _problems(out):
    """Every take: its stills present and non-empty. `drawn`: a drawn-pill line with n > 0. `areas`: an object line with areas > 0.
    `tracked`: an item tracked."""
    problems = []
    for tag, take in out.items():
        if not (isinstance(take, dict) and 'shots' in take):
            continue
        stills = [Path(s) for s in take['shots']]
        if len(stills) != take.get('expected_stills') or not all(s.is_file() and s.stat().st_size > 0 for s in stills):
            problems.append('%s: a still is missing' % tag)
        lines = take.get('lines', [])
        take['drawn_pills'] = [int(m.group(1)) for t in lines for m in [DRAWN.search(t)] if m]
        take['object_lines'] = [t for t in lines if '[Legend] object' in t][:24]
        need = take.get('need', ())
        if 'drawn' in need and not any(n > 0 for n in take['drawn_pills']):
            problems.append('%s: no "[Hitbox] orbs drawn n=<k>" line with k > 0' % tag)
        if 'areas' in need and not any(AREAS.search(t) for t in lines):
            problems.append('%s: no "[Legend] object ... areas=<n>" line with n > 0' % tag)
        if 'tracked' in need and not any(TRACKED.search(t) for t in lines):
            problems.append('%s: the item was never tracked' % tag)
    return problems


def krool(ctx):
    _, out = _begin(ctx)
    _fighter(ctx, KROOL, out, 'krool_hud')
    _take(ctx, out, 'krool-vacuum', 'nspecial-hold', _steps(1.5, 7.0, 6), ('drawn',))
    _take(ctx, out, 'krool-crownerang', 'sspecial', _steps(0.8, 2.6, 6), ('drawn',))
    return _finish(ctx, 'windart-krool', out, _problems(out))


def articles(ctx):
    _, out = _begin(ctx)
    _fighter(ctx, SAMUS, out, 'samus_hud')
    _take(ctx, out, 'samus-bomb', 'dspecial', _steps(2.0, 10.0, 6), ('drawn', 'areas'))
    _fighter(ctx, NESS, out, 'ness_hud')
    _take(ctx, out, 'ness-pkfire', 'sspecial', _steps(1.0, 7.0, 6), ('drawn', 'areas'))
    _fighter(ctx, PALUTENA, out, 'palutena_hud')
    _take(ctx, out, 'palutena-flame', 'sspecial', _steps(1.0, 7.0, 6), ('drawn', 'areas'))
    return _finish(ctx, 'windart-articles', out, _problems(out))


def items(ctx):
    _, out = _begin(ctx)
    _fighter(ctx, SHEIK, out, 'sheik_hud')
    _take(ctx, out, 'sheik-grenade', 'sspecial', _steps(1.0, 12.0, 8), ('tracked',))
    _fighter(ctx, ROB, out, 'rob_hud')
    _take(ctx, out, 'rob-gyro', 'dspecial', _steps(1.0, 12.0, 8), ('tracked',))
    return _finish(ctx, 'windart-items', out, _problems(out))


def restore(ctx):
    out = {'speed': _speed(ctx, 100)}
    fx._live_set(ctx, hitbox_vis=[0, 0, 0, 0, 0, 0], mash_state=[0] * ACTIONS)
    return out
