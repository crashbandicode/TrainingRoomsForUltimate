"""Rustfmt-normalized implementation code lines in Rust: no blank lines, no comments, no tests, no diag-only code.

Provenance: the primary's tool /home/intpa/smash-training-lobby/tools/sloc4.py (2026-11-02, sha256
6dcb208c203b4adcbd8c06c67347726e62c0582bd254723e3a4b6a1dd030fc19), copied 2026-11-02 by the slim stream with its
counting rules unchanged, so totals stay comparable with the fork's count. It replaces the earlier copy of sloc3.py.
Normalized measurement provenance: operator tools/sloc-fmt.ps1 SHA256 ace31db0d654530e3e4faf719dadaef557003510bb4edabed59a5fa42fe77898.
CLI and size_gate format each file via pinned stable rustfmt stdin (edition2021, skip_children=true), then apply
unchanged measure() rules. Scripts/tools are excluded from normalized implementation totals. No source is modified.
Changes: vendored code under third_party/ is excluded and reported on its own line; a `tree` mode counts the working
tree's tracked and untracked-not-ignored files (what tools/size_gate.py gates); depth 0 lists single files; the
counting is a function (`measure`) that size_gate.py imports.

Rules (2026-11-14; operator: diagnostic code that won't ship is not counted, then everything in rc.14, which the
primary read as the yes for test and simulator code through the same rule): `cfg_scan` reads every `#[cfg(...)]`
predicate (`all`, `any`, `not`, `name`, `name = "value"`) with one parser and evaluates it in three values against
what is known of the release build (cargo skyline build --release, default features): `test` is off, feature
`simulator` is off, and the diag features are off (`diag`, every root feature that turns it on, and the path crates'
features only those turn on, read from the Cargo.toml files). Every other name or value (target_os, deluxe-sdk, ...)
is unknown, and an unknown predicate is counted. An item, statement, field, variant, match arm or parameter whose
predicate is false is dropped from its attributes to its end (token trees and Rust's end rules, not a text scan), and
reported by why it is off: test (false once `test` is off), host-only simulator (false once `simulator` is off too) or
diag (false once the diag features are off too). A file reached only through such `mod name;` declarations is dropped
whole with its child modules, and so is a file whose inner `#![cfg(...)]` is false. `#[cfg_attr]` and `cfg!(...)`
drop nothing. A `macro_rules!` template (after each `=>`) is read as items and statements, since every expansion
carries its cfgs (sloc4's text match dropped test items there too); a macro call's body is opaque and stays
counted. Excluded as before: files under tests/, benches/,
examples/; // and /* */ comments; blank lines. sloc4 matched only `#[cfg(test)]`, `#[cfg(all(test, ...))]` and
`#[cfg(any(test, feature = "simulator"))]` and ended an item at its first ';' or '{' (so `const X: [T; N] = ...`,
`-> [T; N] {` and `use a::{b};` were partly counted). measure_normalized checks every file that lost code: what is
left still parses (pinned rustfmt), and each dropped item owns whole lines in rustfmt's layout; otherwise it raises
(size_gate exit 2).
usage: python3 -I sloc.py git <repo> <rev> [depth]  |  python3 -I sloc.py dir <path> [depth]  |  python3 -I sloc.py tree <repo> [depth]"""
import collections
import pathlib
import posixpath
import re
import subprocess
import sys

try:
    import tomllib
except ImportError:  # Python < 3.11; size_gate.py reports it
    tomllib = None

TEST_DIRS = ('tests/', 'benches/', 'examples/')
VENDORED_DIRS = ('third_party/',)
CATEGORIES = ('test', 'sim', 'diag')  # most excluded first: a file reached from several categories takes the last


def strip_comments(text):
    out, i, n, depth, in_str = [], 0, len(text), 0, False
    while i < n:
        c = text[i]
        if depth:
            if text.startswith('/*', i):
                depth += 1; i += 2
            elif text.startswith('*/', i):
                depth -= 1; i += 2
            else:
                if c == '\n':
                    out.append('\n')
                i += 1
            continue
        if in_str:
            out.append(c)
            if c == '\\' and i + 1 < n:
                out.append(text[i + 1]); i += 2; continue
            if c == '"':
                in_str = False
            i += 1
            continue
        if text.startswith('//', i):
            j = text.find('\n', i)
            i = n if j < 0 else j
            continue
        if text.startswith('/*', i):
            depth = 1; i += 2; continue
        if c == '"' and not (i > 0 and text[i - 1] == "'" and i + 1 < n and text[i + 1] == "'"):
            in_str = True
        out.append(c)
        i += 1
    return ''.join(out)


def nonblank(text):
    return sum(1 for line in text.splitlines() if line.strip())


# --- cfg predicates -------------------------------------------------------------------------------------------------

class CfgSyntax(ValueError):
    pass


def parse_cfg(nodes):
    """The predicate in `cfg(...)`'s argument (its token nodes): ('all'|'any', [p...]), ('not', [p]) or ('atom', name,
    value-or-None). Raises CfgSyntax when it is not one predicate."""
    preds = _cfg_list(nodes)
    if len(preds) != 1:
        raise CfgSyntax(f'cfg takes one predicate, found {len(preds)}')
    return preds[0]


def _cfg_list(nodes):
    preds, i = [], 0
    while i < len(nodes):
        pred, i = _cfg_pred(nodes, i)
        preds.append(pred)
        if i < len(nodes):
            if not _is(nodes[i], ','):
                raise CfgSyntax(f'expected "," in a cfg list, found {_text(nodes[i])!r}')
            i += 1
    return preds


def _cfg_pred(nodes, i):
    t = nodes[i]
    if not (isinstance(t, Tok) and t.kind == 'ident'):
        raise CfgSyntax(f'expected a cfg name, found {_text(t)!r}')
    if t.text in ('all', 'any', 'not') and i + 1 < len(nodes) and _group(nodes[i + 1], '('):
        args = _cfg_list(nodes[i + 1].children)
        if t.text == 'not' and len(args) != 1:
            raise CfgSyntax('not() takes one predicate')
        return (t.text, args), i + 2
    if i + 1 < len(nodes) and _is(nodes[i + 1], '='):
        if i + 2 >= len(nodes) or not (isinstance(nodes[i + 2], Tok) and nodes[i + 2].kind == 'str'):
            raise CfgSyntax(f'cfg {t.text} = needs a string')
        return ('atom', t.text, _unquote(nodes[i + 2].text)), i + 3
    return ('atom', t.text, None), i + 1


def evaluate(pred, known):
    """Three-valued: True, False or None (unknown). known(name, value) answers an atom the same way."""
    op = pred[0]
    if op == 'atom':
        return known(pred[1], pred[2])
    if op == 'not':
        value = evaluate(pred[1][0], known)
        return None if value is None else not value
    values = [evaluate(p, known) for p in pred[1]]
    if op == 'all':
        return False if False in values else (True if all(v is True for v in values) else None)
    return True if True in values else (False if all(v is False for v in values) else None)


def knowledge(features_off=frozenset(), flags_off=('test',)):
    """What is known of the release build: these flags and features are off; nothing else is known."""
    def known(name, value):
        if value is None:
            return False if name in flags_off else None
        if name == 'feature' and value in features_off:
            return False
        return None
    return known


class Release:
    """The release build's known-off features per crate, from the manifests ({repo path: Cargo.toml text}).

    The diag features are `diag` and every root feature that turns it on; a path crate's diag features are the ones
    only those turn on. The release build is the default features (cargo skyline build --release)."""

    def __init__(self, manifests=None):
        self.diag = {'': frozenset({'diag'})}
        manifests = manifests or {}
        if 'Cargo.toml' in manifests and tomllib is not None:
            self._read(manifests)

    def _read(self, manifests):
        root = tomllib.loads(manifests['Cargo.toml'])
        features = root.get('features', {})

        def closure(names, table):
            seen, deps, stack = set(), set(), list(names)
            while stack:
                name = stack.pop()
                if name in seen:
                    continue
                seen.add(name)
                for entry in table.get(name, []):
                    if entry.startswith('dep:'):
                        continue
                    if '/' in entry:
                        crate, feature = entry.split('/', 1)
                        deps.add((crate.rstrip('?'), feature))
                    else:
                        stack.append(entry)
            return seen, deps

        family = {name for name in features if 'diag' in closure([name], features)[0]}
        family.add('diag')
        shipped, shipped_deps = closure(['default'] if 'default' in features else [], features)
        if family & shipped:
            raise ValueError(f'the default features turn on diag features: {sorted(family & shipped)}')
        _, family_deps = closure(sorted(family), features)
        self.diag = {'': frozenset(family)}
        for name, spec in root.get('dependencies', {}).items():
            if not (isinstance(spec, dict) and 'path' in spec):
                continue
            crate_dir = posixpath.normpath(spec['path'])
            text = manifests.get(posixpath.join(crate_dir, 'Cargo.toml'))
            if text is None:
                continue
            table = tomllib.loads(text).get('features', {})
            wanted = set(spec.get('features', []))
            if spec.get('default-features', True) and 'default' in table:
                wanted.add('default')
            wanted |= {f for crate, f in shipped_deps if crate == name}
            on = closure(sorted(wanted), table)[0]
            diag_on = closure(sorted(f for crate, f in family_deps if crate == name), table)[0]
            self.diag[crate_dir] = frozenset(diag_on - on)

    def crate(self, path):
        """The crate directory a source file belongs to ('' for the root)."""
        best = ''
        for crate_dir in self.diag:
            if crate_dir and (path == crate_dir or path.startswith(crate_dir + '/')) and len(crate_dir) > len(best):
                best = crate_dir
        return best

    def decider(self, path):
        """category(predicate) for a file: 'test', 'sim' or 'diag' when the release build leaves it out, else None."""
        levels = (('test', knowledge()),
                  ('sim', knowledge({'simulator'})),
                  ('diag', knowledge({'simulator'} | self.diag[self.crate(path)])))

        def decide(pred):
            for category, known in levels:
                if evaluate(pred, known) is False:
                    return category
            return None
        return decide


# --- Rust tokens and token trees ------------------------------------------------------------------------------------

TOKEN = re.compile(r'''
    (?P<ws>\s+)
  | (?P<lcomment>//[^\n]*)
  | (?P<bcomment>/\*)
  | (?P<rawstr>(?:br|cr|r)(?P<hashes>\#*)")
  | (?P<str>[bc]?")
  | (?P<char>b?'(?:[^'\\\n]|\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]{1,8}\}|[^\n]))')
  | (?P<lifetime>'(?:r\#)?[A-Za-z_][A-Za-z0-9_]*)
  | (?P<ident>(?:r\#)?[A-Za-z_][A-Za-z0-9_]*)
  | (?P<num>[0-9][0-9_]*(?:\.[0-9][0-9_]*)?(?:[eE][+-]?[0-9_]+)?[A-Za-z0-9_]*)
  | (?P<punct>::|->|=>|==|!=|<=|>=|&&|\|\||\.\.=|\.\.\.|\.\.|<<=|>>=|<<|>>|[-+*/%^&|]=|[^\s])
''', re.X)
OPEN, CLOSE = {'(': ')', '[': ']', '{': '}'}, {')', ']', '}'}
KEYWORDS = frozenset('''as async await break const continue crate dyn else enum extern false fn for if impl in let loop
    match mod move mut pub ref return self static struct super trait true type unsafe use where while yield'''.split())
ITEM_HEADS = frozenset(('fn', 'mod', 'impl', 'trait', 'enum', 'struct'))
SEMI_HEADS = frozenset(('use', 'let', 'static', 'type'))


class Tok:
    __slots__ = ('kind', 'text', 'start', 'end')

    def __init__(self, kind, text, start, end):
        self.kind, self.text, self.start, self.end = kind, text, start, end


class Group:
    __slots__ = ('delim', 'children', 'start', 'end')

    def __init__(self, delim, start):
        self.delim, self.children, self.start, self.end = delim, [], start, start


def _is(node, text):
    return isinstance(node, Tok) and node.text == text and node.kind != 'str'


def _group(node, delim):
    return isinstance(node, Group) and node.delim == delim


def _text(node):
    return node.text if isinstance(node, Tok) else node.delim + '...'


def _unquote(text):
    return text[text.index('"') + 1:text.rindex('"')]


def tokenize(text, comments=None):
    """Rust tokens of text; the spans of its comments go to `comments` when it is a list."""
    tokens, i, n = [], 0, len(text)
    while i < n:
        m = TOKEN.match(text, i)
        kind = m.lastgroup if m.lastgroup != 'hashes' else 'rawstr'
        if kind == 'ws':
            i = m.end()
            continue
        if kind == 'lcomment':
            if comments is not None:
                comments.append((i, m.end()))
            i = m.end()
            continue
        if kind == 'bcomment':
            depth, j = 1, m.end()
            while depth and j < n:
                if text.startswith('/*', j):
                    depth += 1; j += 2
                elif text.startswith('*/', j):
                    depth -= 1; j += 2
                else:
                    j += 1
            if comments is not None:
                comments.append((i, j))
            i = j
            continue
        if kind == 'rawstr':
            close = '"' + m.group('hashes')
            j = text.find(close, m.end())
            if j < 0:
                raise ValueError(f'unterminated raw string at offset {i}')
            tokens.append(Tok('str', text[i:j + len(close)], i, j + len(close)))
            i = j + len(close)
            continue
        if kind == 'str':
            j = m.end()
            while j < n and text[j] != '"':
                j += 2 if text[j] == '\\' else 1
            if j >= n:
                raise ValueError(f'unterminated string at offset {i}')
            tokens.append(Tok('str', text[i:j + 1], i, j + 1))
            i = j + 1
            continue
        tokens.append(Tok(kind, m.group(), i, m.end()))
        i = m.end()
    return tokens


def token_tree(tokens):
    root = Group('', 0)
    stack = [root]
    for t in tokens:
        if t.kind == 'punct' and t.text in OPEN:
            g = Group(t.text, t.start)
            stack[-1].children.append(g)
            stack.append(g)
        elif t.kind == 'punct' and t.text in CLOSE:
            if len(stack) == 1 or OPEN[stack[-1].delim] != t.text:
                raise ValueError(f'unbalanced {t.text!r} at offset {t.start}')
            stack.pop().end = t.end
        else:
            stack[-1].children.append(t)
    if len(stack) != 1:
        raise ValueError(f'unclosed {stack[-1].delim!r} at offset {stack[-1].start}')
    root.end = tokens[-1].end if tokens else 0
    return root


# --- the extent of a cfg-gated element ------------------------------------------------------------------------------

def _find(nodes, i, pred):
    while i < len(nodes):
        if pred(nodes[i]):
            return i
        i += 1
    return -1


def _semi_end(nodes, i):
    j = _find(nodes, i, lambda node: _is(node, ';'))
    return len(nodes) if j < 0 else j + 1


def _block_like_end(nodes, k):
    """End of `if`/`match`/`while`/`for`/`loop`/`unsafe {}`/`{}` starting at k (exclusive), or -1."""
    while True:
        t = nodes[k]
        if isinstance(t, Group):
            return k + 1 if t.delim == '{' else -1
        word = t.text
        start = k + 1
        if word in ('if', 'while') and start < len(nodes) and _is(nodes[start], 'let'):
            start = _find(nodes, start, lambda node: _is(node, '='))
        elif word == 'for':
            start = _find(nodes, start, lambda node: _is(node, 'in'))
        if start < 0:
            return -1
        body = _find(nodes, start, lambda node: _group(node, '{'))
        if body < 0:
            return -1
        if word == 'if' and body + 1 < len(nodes) and _is(nodes[body + 1], 'else') and body + 2 < len(nodes):
            k = body + 2
            continue
        return body + 1


def _item_end(nodes, j):
    """(end, kind) of the item or statement starting at j (after its attributes) in an items/stmts sequence."""
    n, k = len(nodes), j
    if k < n and _is(nodes[k], 'pub'):
        k += 1
        if k < n and _group(nodes[k], '('):
            k += 1
    while k < n:
        t = nodes[k]
        nxt = nodes[k + 1] if k + 1 < n else None
        if _group(t, '{') and not _continues(nodes, k + 1):
            return _optional(nodes, k + 1, ';'), 'block'
        if not isinstance(t, Tok):
            break
        if t.kind == 'lifetime' and _is(nxt, ':'):
            k += 2
            continue
        word = t.text
        if word in ('if', 'match', 'while', 'for', 'loop') or (word in ('unsafe', 'const', 'async') and _group(nxt, '{')) \
                or (word == 'async' and _is(nxt, 'move')):
            if word == 'async' and _is(nxt, 'move'):
                k += 1
            end = _block_like_end(nodes, k + (1 if word in ('unsafe', 'const', 'async') else 0))
            if end < 0 or _continues(nodes, end):
                break
            return _optional(nodes, end, ';'), 'block'
        if word in ('unsafe', 'async', 'default', 'auto') or (word == 'const' and nxt is not None and _text(nxt) in (
                'fn', 'unsafe', 'async', 'extern')):
            k += 1
            continue
        if word == 'extern':
            if _is(nxt, 'crate'):
                return _semi_end(nodes, k), 'use'
            k += 2 if isinstance(nxt, Tok) and nxt.kind == 'str' else 1
            if k < n and _group(nodes[k], '{'):
                return k + 1, 'extern'
            continue
        if word == 'macro_rules' and _is(nxt, '!'):
            body = _find(nodes, k + 2, lambda node: isinstance(node, Group))
            if body < 0:
                break
            return (_optional(nodes, body + 1, ';') if nodes[body].delim == '{' else _semi_end(nodes, body)), 'macro'
        if word in ITEM_HEADS or (word == 'union' and isinstance(nxt, Tok) and nxt.kind == 'ident'):
            end = _find(nodes, k + 1, lambda node: _is(node, ';') or _group(node, '{'))
            return (len(nodes) if end < 0 else end + 1), word
        if word in SEMI_HEADS or word == 'const':
            return _semi_end(nodes, k), word
        if t.kind == 'ident':  # a macro call: path ! group
            m = k
            while m + 2 < n and _is(nodes[m + 1], '::') and isinstance(nodes[m + 2], Tok):
                m += 2
            if m + 2 < n and _is(nodes[m + 1], '!') and isinstance(nodes[m + 2], Group):
                if nodes[m + 2].delim == '{':
                    return _optional(nodes, m + 3, ';'), 'macro'
                return _semi_end(nodes, m + 3), 'macro'
        break
    return _semi_end(nodes, j), 'expr'


def _optional(nodes, i, text):
    return i + 1 if i < len(nodes) and _is(nodes[i], text) else i


def _comma_end(nodes, j):
    k = j
    if k < len(nodes) and _is(nodes[k], 'move'):
        k += 1
    if k < len(nodes) and _is(nodes[k], '|'):  # closure parameters hold commas
        bar = _find(nodes, k + 1, lambda node: _is(node, '|'))
        if bar >= 0:
            k = bar + 1
    end = _find(nodes, k, lambda node: _is(node, ','))
    return len(nodes) if end < 0 else end + 1


def _arm_end(nodes, j):
    arrow = _find(nodes, j, lambda node: _is(node, '=>') or _is(node, ','))
    if arrow < 0 or _is(nodes[arrow], ','):
        return _comma_end(nodes, j)
    b = arrow + 1
    if b < len(nodes) and (_group(nodes[b], '{') or (isinstance(nodes[b], Tok) and nodes[b].text in (
            'if', 'match', 'while', 'for', 'loop')) or (_is(nodes[b], 'unsafe') and b + 1 < len(nodes) and _group(
            nodes[b + 1], '{'))):
        end = _block_like_end(nodes, b + (1 if _is(nodes[b], 'unsafe') else 0))
        if end > 0 and not _continues(nodes, end):
            return _optional(nodes, end, ',')
    return _comma_end(nodes, b)


def _continues(nodes, i):
    """A block-like expression ends its statement or arm unless a method call or `?` goes on from it."""
    return i < len(nodes) and (_is(nodes[i], '.') or _is(nodes[i], '?'))


def _attributes(nodes, i):
    """Outer attributes at i: (index after them, [attribute groups])."""
    attrs = []
    while i + 1 < len(nodes) and _is(nodes[i], '#') and _group(nodes[i + 1], '['):
        attrs.append(nodes[i + 1])
        i += 2
    return i, attrs


def _attr_cfg(group):
    """The cfg predicate of an attribute group `[cfg(...)]`, 'unknown' for a cfg that does not parse, or None."""
    c = group.children
    if len(c) == 2 and _is(c[0], 'cfg') and _group(c[1], '('):
        try:
            return parse_cfg(c[1].children)
        except CfgSyntax:
            return 'unknown'
    return None


def _attr_path(group):
    c = group.children
    if len(c) == 3 and _is(c[0], 'path') and _is(c[1], '=') and isinstance(c[2], Tok) and c[2].kind == 'str':
        return _unquote(c[2].text)
    return None


class Walker:
    """Finds the regions of a file a false cfg removes (outermost only) and the `mod name;` declarations, each with the
    category that removes it (None: compiled in the release build)."""

    def __init__(self, decide):
        self.decide = decide
        self.regions = []      # (start, end, category, context)
        self.mods = []         # (name, path attribute or None, category or None)
        self.opaque_cfgs = 0   # false cfgs inside macro bodies, which stay counted

    def category(self, attrs):
        found = None
        for attr in attrs:
            pred = _attr_cfg(attr)
            if pred is None or pred == 'unknown':
                continue
            category = self.decide(pred)
            if category is not None and (found is None or CATEGORIES.index(category) < CATEGORIES.index(found)):
                found = category
        return found

    def file(self, root):
        """Walks the file; returns the category of a false inner #![cfg] at its top (the whole file), else None."""
        return self.sequence(root.children, 'items', None, top=True)

    def sequence(self, nodes, kind, inherited, top=False):
        i = 0
        if kind in ('items', 'stmts'):
            inner = []
            while i + 2 < len(nodes) and _is(nodes[i], '#') and _is(nodes[i + 1], '!') and _group(nodes[i + 2], '['):
                inner.append(nodes[i + 2])
                i += 3
            category = self.category(inner)
            if category is not None:
                if not top and inherited is None:
                    self.regions.append((nodes[0].start, nodes[-1].end, category, 'inner'))
                self.sequence(nodes[i:], kind, category)  # its `mod` declarations take the category
                return category
        while i < len(nodes):
            j, attrs = _attributes(nodes, i)
            if j >= len(nodes):
                break
            if kind in ('items', 'stmts'):
                end, head = _item_end(nodes, j)
            elif kind == 'arms':
                end, head = _arm_end(nodes, j), 'arm'
            else:
                end, head = _comma_end(nodes, j), 'expr'
            end = max(end, j + 1)
            own = self.category(attrs)
            if own is not None and inherited is None:
                self.regions.append((nodes[i].start, nodes[end - 1].end, own, kind))
            here = inherited if inherited is not None else own
            if head == 'mod' and end - j >= 2:
                self.mod(nodes, j, end, attrs, here)
            self.element(nodes[j:end], head, here)
            i = end
        return None

    def mod(self, nodes, j, end, attrs, category):
        k = _find(nodes, j, lambda node: _is(node, 'mod'))
        if 0 <= k < end - 1 and isinstance(nodes[k + 1], Tok) and nodes[k + 1].kind == 'ident' and _is(nodes[end - 1], ';'):
            path = next((p for p in map(_attr_path, attrs) if p), None)
            self.mods.append((nodes[k + 1].text, path, category))

    def element(self, nodes, head, inherited):
        if head in ('fn', 'struct', 'enum', 'union', 'impl', 'trait', 'mod', 'extern', 'use') and nodes and _group(
                nodes[-1], '{'):
            for node in nodes[:-1]:
                if isinstance(node, Group):
                    self.group(node, 'commas', inherited)
            body = {'fn': 'stmts', 'struct': 'commas', 'enum': 'commas', 'union': 'commas', 'use': 'commas'}.get(
                head, 'items')
            self.group(nodes[-1], body, inherited)
            return
        if head == 'macro':
            if nodes and _is(nodes[0], 'macro_rules'):
                self.macro_rules(nodes, inherited)
            else:
                self.opaque(nodes)
            return
        self.walk_nodes(nodes, inherited)

    def macro_rules(self, nodes, inherited):
        """A `macro_rules!` template (the group after each `=>`) is items and statements that every expansion
        carries, cfg attributes included; the matchers stay opaque."""
        body = next((node for node in nodes[2:] if isinstance(node, Group)), None)
        if body is None:
            return
        children = body.children
        for idx, node in enumerate(children):
            if not isinstance(node, Group):
                continue
            if idx and _is(children[idx - 1], '=>'):
                self.group(node, 'stmts', inherited)
            else:
                self.opaque([node])

    def walk_nodes(self, nodes, inherited):
        """Expression context: classify each group by what precedes it."""
        pending, pattern, arrow = None, False, False
        for idx, node in enumerate(nodes):
            prev = nodes[idx - 1] if idx else None
            if isinstance(node, Tok):
                word = node.text if node.kind != 'str' else ''
                nxt = nodes[idx + 1] if idx + 1 < len(nodes) else None
                if word in ('if', 'while', 'match'):
                    pending, pattern = word, word != 'match' and _is(nxt, 'let')
                elif word == 'for' and not _is(nxt, '<'):
                    pending, pattern = 'for', True
                elif word == 'in' and pending == 'for':
                    pattern = False
                elif word == '=' and pattern and pending in ('if', 'while'):
                    pattern = False
                elif word == '->':
                    arrow = True
                elif word in ('=', ',', ';', '=>'):
                    arrow = False
                continue
            if prev is not None and _is(prev, '!'):
                self.opaque([node])
                continue
            if idx >= 1 and _is(prev, '#') or idx >= 2 and _is(prev, '!') and _is(nodes[idx - 2], '#'):
                continue
            if node.delim != '{':
                self.group(node, 'commas', inherited)
                continue
            if pending and not pattern:
                kind, pending = ('arms' if pending == 'match' else 'stmts'), None
            elif pattern:
                kind = 'commas'
            elif arrow:
                kind, arrow = 'stmts', False
            elif isinstance(prev, Tok) and prev.kind == 'ident' and (prev.text not in KEYWORDS or prev.text == 'Self') \
                    or _is(prev, '>') or _is(prev, '>>'):
                kind = 'commas'
            else:
                kind = 'stmts'
            self.group(node, kind, inherited)

    def group(self, group, kind, inherited):
        self.sequence(group.children, kind, inherited)

    def opaque(self, nodes):
        stack = list(nodes)
        while stack:
            node = stack.pop()
            if isinstance(node, Group):
                c = node.children
                if node.delim == '[' and len(c) == 2 and _is(c[0], 'cfg') and _group(c[1], '('):
                    try:
                        if self.decide(parse_cfg(c[1].children)) is not None:
                            self.opaque_cfgs += 1
                    except CfgSyntax:
                        pass
                stack.extend(c)


def cfg_scan(text, decide):
    """(regions, mods, file category, opaque false cfgs) of one file; see Walker."""
    walker = Walker(decide)
    tokens = tokenize(text)
    file_category = walker.file(token_tree(tokens)) if tokens else None
    return walker.regions, walker.mods, file_category, walker.opaque_cfgs


def without_comments(text):
    """text with its comments (doc comments too) removed, line breaks kept: what a dropped item's doc comment leaves."""
    spans = []
    tokenize(text, spans)
    out, pos = [], 0
    for start, end in spans:
        out.append(text[pos:start])
        out.append('\n' * text.count('\n', start, end))
        pos = end
    out.append(text[pos:])
    return ''.join(out)


def cut(text, regions):
    out, pos = [], 0
    for start, end, *_ in sorted(regions):
        if start < pos:
            continue
        out.append(text[pos:start])
        pos = end
    out.append(text[pos:])
    return ''.join(out)


# --- files ----------------------------------------------------------------------------------------------------------

def _own_dir(path):
    d, base = posixpath.dirname(path), posixpath.basename(path)
    return d, (d if base in ('mod.rs', 'lib.rs', 'main.rs') else posixpath.join(d, base[:-3]))


def declared_children(path, mods):
    """Yield (child_path, category) for the `mod x;` declarations cfg_scan found in a file."""
    d, own_dir = _own_dir(path)
    for name, attr_path, category in mods:
        if attr_path:
            yield posixpath.normpath(posixpath.join(d, attr_path)), category
        else:
            yield posixpath.join(own_dir, name + '.rs'), category
            yield posixpath.join(own_dir, name, 'mod.rs'), category


def group(path, depth):
    if depth == 0:
        return path
    parts = path.split('/')
    return '/'.join(parts[:depth]) if len(parts) > depth else '/'.join(parts[:-1]) or '(root)'


def in_dirs(name, dirs):
    return any(f'/{d}' in f'/{name}' for d in dirs)


def is_manifest(name):
    return posixpath.basename(name) == 'Cargo.toml' and not in_dirs(name, TEST_DIRS + VENDORED_DIRS)


def files_git(repo, rev):
    names = subprocess.run(['git', '-C', str(repo), 'ls-tree', '-r', '--name-only', rev],
                           capture_output=True, text=True, check=True).stdout.split('\n')
    return {name: subprocess.run(['git', '-C', str(repo), 'show', f'{rev}:{name}'], capture_output=True,
                                 check=True).stdout.decode('utf-8', 'replace')
            for name in names if name.endswith('.rs') and not in_dirs(name, TEST_DIRS) or is_manifest(name)}


def files_dir(root):
    root = pathlib.Path(root)
    files = {}
    for p in list(root.rglob('*.rs')) + list(root.rglob('Cargo.toml')):
        rel = p.relative_to(root).as_posix()
        if rel.startswith('target/') or '/target/' in rel or (rel.endswith('.rs') and in_dirs(rel, TEST_DIRS)) \
                or (rel.endswith('.toml') and not is_manifest(rel)):
            continue
        files[rel] = p.read_text('utf-8', 'replace')
    return files


def files_tree(repo):
    """The working tree as git sees it (tracked plus untracked-not-ignored); a plain directory walk without git."""
    repo = pathlib.Path(repo)
    try:
        names = subprocess.run(['git', '-C', str(repo), 'ls-files', '--cached', '--others', '--exclude-standard'],
                               capture_output=True, text=True, check=True).stdout.split('\n')
    except (OSError, subprocess.CalledProcessError):
        return files_dir(repo)
    return {n: (repo / n).read_text('utf-8', 'replace') for n in names
            if (n.endswith('.rs') and not in_dirs(n, TEST_DIRS) or is_manifest(n)) and (repo / n).is_file()}


Measure = collections.namedtuple('Measure', 'impl sim tests vendored diag excluded')
Measure.__doc__ = """impl: implementation lines per group; sim, tests, diag: lines left out of the count; vendored:
third_party/ lines; excluded: {path: {category: lines}} for every file that lost some."""


def cfg_status(sources, scans):
    """{path: category or None} of every file from its `mod` declarations: the most compiled of the declarations that
    reach it, where a declaration's category is the least compiled of its own cfg and its declaring file's; a file no
    declaration reaches has its own inner #![cfg]'s category."""
    reached = collections.defaultdict(list)
    for path, (_, mods, _, _) in scans.items():
        for child, category in declared_children(path, mods):
            if child in sources:
                reached[child].append((path, category))
    rank = {category: i for i, category in enumerate(CATEGORIES)}
    rank[None] = len(CATEGORIES)
    status = {path: (CATEGORIES[0] if path in reached else scans[path][2]) for path in sources}
    for _ in range(len(sources) + 1):  # the least fixed point: every reached file starts at the least compiled
        changed = False
        for path, decls in reached.items():
            best = max(min(rank[own], rank[status[parent]]) for parent, own in decls)
            best = min(best, rank[scans[path][2]])
            new = CATEGORIES[best] if best < len(CATEGORIES) else None
            if new != status[path]:
                status[path], changed = new, True
        if not changed:
            break
    return status


def measure(files, depth, check=None, release=None):
    """Count the Rust files of `files` ({repo path: text}; Cargo.toml manifests in it say which features are diag).

    Every cfg goes through the evaluator (cfg_scan): a file whose category is test, sim or diag is left out whole; in
    the other files each outermost gated region is cut, and its lines go to the region's category (counted in that
    order, so the three numbers add up to what left the count). check(path, kept_text, regions), when given, is called
    for each file that lost part of its code."""
    release = release or Release({k: v for k, v in files.items() if k.endswith('Cargo.toml')})
    sources = {k: v for k, v in files.items() if k.endswith('.rs')}
    scans = {}
    for path, text in sources.items():
        try:
            scans[path] = cfg_scan(text, release.decider(path))
        except ValueError as error:
            raise ValueError(f'{path}: {error}') from error
    status = cfg_status(sources, scans)
    impl, lost, excluded, vendored = collections.Counter(), collections.Counter(), {}, 0
    for path, text in sources.items():
        kept = nonblank(strip_comments(text))
        if status[path] is not None:
            lost[status[path]] += kept
            excluded[path] = {status[path]: kept}
            continue
        regions, per = scans[path][0], {}
        for i, category in enumerate(CATEGORIES):
            inside = [r for r in regions if r[2] in CATEGORIES[:i + 1]]
            now = nonblank(strip_comments(cut(text, inside))) if inside else kept
            if kept - now:
                per[category] = kept - now
                lost[category] += kept - now
            kept = now
        if per:
            excluded[path] = per
        if regions and check is not None:
            check(path, cut(text, regions), regions)
        if in_dirs(path, VENDORED_DIRS):
            vendored += kept
        else:
            impl[group(path, depth)] += kept
    return Measure(impl, lost['sim'], lost['test'], vendored, lost['diag'], excluded)


# Operator's sloc-fmt.ps1, 2026-11-02: edition 2021 and stable formatter.
# Format isolated stdin so measurement never edits source or follows module paths.
# The operator's final comparison excludes scripts (tools/ and scripts/).
NORMALIZED_BASIS = "rustfmt-sloc4"
RUSTFMT_VERSION = "rustfmt 1.9.0-stable (48a229ceae 2026-09-01)"


def _rustfmt():
    rustup = pathlib.Path.home() / ".cargo/bin/rustup"
    command = [str(rustup), "run", "1.98.1", "rustfmt"]
    version = subprocess.run(command + ["--version"], capture_output=True, text=True,
                             check=True, timeout=30).stdout.strip()
    if version != RUSTFMT_VERSION:
        raise ValueError(f"normalization requires {RUSTFMT_VERSION}; found {version}")
    return command + ["--edition", "2021", "--config", "skip_children=true", "--emit", "stdout"]


def normalized_files(files):
    command = _rustfmt()
    result = {}
    for name, text in files.items():
        if in_dirs(name, VENDORED_DIRS) or name.startswith(("tools/", "scripts/")):
            continue
        if not name.endswith('.rs'):
            result[name] = text
            continue
        formatted = subprocess.run(command, input=text, capture_output=True, text=True, timeout=30)
        if formatted.returncode:
            raise ValueError(f"rustfmt failed for {name}: {formatted.stderr.strip()}")
        result[name] = formatted.stdout
    return result


def layout_check(path, kept, regions, text):
    """In rustfmt's layout an item, statement or match arm owns whole lines: a dropped one starts its line and ends
    its last line, and no line inside it starts a new element at the attribute's indentation."""
    for start, end, category, context in regions:
        if context not in ('items', 'stmts', 'arms'):
            continue
        where = f'{path}: the {category}-only element at line {text.count(chr(10), 0, start) + 1}'
        line_start = text.rfind('\n', 0, start) + 1
        indent = text[line_start:start]
        tail_end = text.find('\n', end)
        if indent.strip() or text[end:tail_end if tail_end >= 0 else len(text)].strip():
            raise ValueError(f'{where} does not own whole lines')
        strings = [(t.start - start, t.end - start) for t in tokenize(text[start:end]) if t.kind == 'str']
        offset, seen_item = 0, False
        for line in text[start:end].split('\n'):
            stripped, own = line.strip(), line[:len(line) - len(line.lstrip())]
            if offset and stripped and not any(a < offset < b for a, b in strings):
                if len(own) < len(indent):
                    raise ValueError(f'{where} reaches out of its block')
                if own == indent and not stripped.startswith(('}', ')', ']', 'where', '{', 'else', '|', '.', '?')):
                    if seen_item:
                        raise ValueError(f'{where} runs into the next element')
                    seen_item = not stripped.startswith('#')
            offset += len(line) + 1


def parses(command, text):
    """rustfmt parses the code (formatting complaints, `error[internal]`, are not parse errors)."""
    result = subprocess.run(command + ['--color', 'never'], input=without_comments(text), capture_output=True,
                            text=True, timeout=30)
    errors = [line for line in result.stderr.splitlines() if line.startswith('error')]
    return result.returncode == 0 or bool(errors) and all(line.startswith('error[internal]') for line in errors)


def measure_normalized(files, depth):
    normalized = normalized_files(files)
    command = _rustfmt()

    def check(path, kept, regions):
        layout_check(path, kept, regions, normalized[path])
        if not parses(command, kept):
            raise ValueError(f'{path}: the code left after dropping its cfg-gated items does not parse')
    return measure(normalized, depth, check)


def main(argv):
    mode = argv[1]
    if mode == 'git':
        files, rest = files_git(argv[2], argv[3]), argv[4:]
    elif mode == 'dir':
        files, rest = files_dir(argv[2]), argv[3:]
    else:
        files, rest = files_tree(argv[2]), argv[3:]
    m = measure_normalized(files, int(rest[0]) if rest else 1)
    print(f'Basis: {NORMALIZED_BASIS}; {RUSTFMT_VERSION}; edition 2021; scripts excluded')
    for g, n in sorted(m.impl.items(), key=lambda kv: -kv[1]):
        print(f'{g:40s} {n:8d}')
    print(f'{"TOTAL (implementation, no tests)":40s} {sum(m.impl.values()):8d}')
    print(f'{"host-only simulator (not counted)":40s} {m.sim:8d}')
    print(f'{"test code excluded (files + gated items)":40s} {m.tests:8d}')
    print(f'{"diag-only code (not in the release build)":40s} {m.diag:8d}')
    print(f'{"third_party/ (vendored, not counted)":40s} {m.vendored:8d}')


if __name__ == '__main__':
    main(sys.argv)
