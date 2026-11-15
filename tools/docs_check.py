"""Offline docs gates for README.md and docs/**/*.md: links, anchors, orphan pages and version claims.

Operator order 2026-11-15: "automated checks for stale links/outdated version references and docs as gates". Run by
`tests/docs_gates.rs` on every `cargo test` (`python3 -I tools/docs_check.py [--root DIR] [--verbose]`). It reads only the
files under the root: no network, no clock, no git, so every host gives the same answer. Exit 0: clean; 1: problems
(printed one per line as `rule path:line: message`); 2: it could not run (no root, unreadable file).
`tools/check_external_links.py` is the network check (a release step); it reuses `external_links` below.

Rules (a problem's rule name is the first word of its line):

  link-missing     a relative link or image whose target file or folder does not exist. The case must match too, also
                   on a host whose file system ignores it, because the forge's does not. A target that leaves the repo
                   (`../..`) or starts with `/` (a path on someone's machine, not on the forge) is `link-missing` too.
  link-anchor      `#anchor` (alone, or after a .md file) with no heading or `<a id/name>` of that name in the target.
  link-ref         `[text][label]` or `[label][]` whose `[label]: target` is not defined in the same file.
  link-syntax      a `](` that does not form a link (the forge would print it as plain text).
  orphan           a .md page under docs/ that no chain of links from README.md or docs/README.md reaches. A link to a
                   folder does not reach the pages in it; the folders of project records are listed in `ORPHAN_OK`.
  version-line     docs/CURRENT-RELEASE.md has no `Current release: rc.N` line, or more than one.
  version-name     an entry point (`MUST_NAME`: the README's top, CURRENT-RELEASE.md, the Getting started guide and the
                   Status section of TECHNICAL.md) does not name the current release, or the README's top does not link
                   to docs/CURRENT-RELEASE.md.
  version-stale    a sentence that says "newest", "latest", "currently", "current release/build/version", "this
                   release/build/version" or "up to date" and names another release (rc.N or rcN, also 0.2.0-rc.N).
  allowlist-stale  an entry of the lists below that matches no file, or whose sentence is gone.

Anchors follow the slug rules of the forge's Markdown renderer (Forgejo). They are NOT GitHub's: GitHub drops punctuation
(`L+R+A Reset` is `lra-reset` there), the forge turns it into a hyphen (`l-r-a-reset`). The rule, read from the forge's own
HTML on 2026-11-15 (the pairs are in tools/test_docs_check.py's `FORGE_SLUGS`): the heading's text with its inline Markdown
removed, lower-cased; every run of characters that are not letters, digits or `_` (spaces, `&`, `+`, `.`, `'`, `/`, `:`,
`·`, `—`, `-` itself) becomes one `-`; no `-` at either end; a second heading with the same slug gets `-1`, a third `-2`.
The forge prints every id with the prefix `user-content-`; the links in the docs are written without it. Non-ASCII
letters are kept lower-cased (not seen in this repo's headings, so untested).

What is not checked: external URLs (see above), anchors in files that are not Markdown (a `#L10` of a source file),
indented code blocks (a link inside one is read like any other), and what a page says outside the rules above.
"""
import argparse
import bisect
import fnmatch
import html
import os
import posixpath
import re
import sys
import unicodedata
from typing import NamedTuple
from urllib.parse import unquote, urlsplit

CURRENT_PAGE = "docs/CURRENT-RELEASE.md"
CURRENT_LINE = re.compile(r"^Current release: (rc\.\d+)\s*$", re.M)
INDEXES = ("README.md", "docs/README.md")

# Pages that are history, not a statement about today's build: they keep the name of the build they describe, so the
# version rules (`version-stale`, `version-name`) skip them. Links and orphans are still checked. Player pages (docs/guide,
# docs/menu and the top-level references) are never listed here as a folder.
HISTORICAL = [
    ("docs/RELEASE-NOTES-rc*.md", "the release notes of a past release describe that release"),
    ("docs/DECISIONS.md", "the record of what was decided and when"),
    ("docs/decisions/*", "decision records and their mock-ups, dated"),
    ("docs/design/*", "design notes of the build they were written for"),
    ("docs/provenance/*", "where carried code and assets came from, per build"),
    ("docs/test-provenance/*", "which tests covered which package, per build"),
    ("docs/menu-mockup/*", "mock-ups of a menu before it was built"),
    ("docs/menu-redesign.md", "the redesign proposal, dated"),
    ("docs/ROADMAP.md", "planned work as of its date"),
    ("docs/MEDIA.md", "where each clip was recorded: every row names the build it was shot on"),
    ("docs/FEATURE-LEDGER.md", "test evidence per build"),
    ("docs/CODE-REDUCTION-rc10.md", "a measurement of rc.10"),
    ("docs/COVERAGE-rc9.md", "a measurement of rc.9"),
    ("docs/TEST-CUT-LEDGER-rc9.md", "a measurement of rc.9"),
    ("docs/SIZE-PLAN.md", "the size plan and its history"),
    ("docs/resource-costs.md", "a measurement"),
    ("docs/ASSET-REPRODUCTION-R15.md", "a record of how assets were reproduced"),
    ("docs/MII.md", "developer notes on the Mii work, per build"),
]

# Pages no link chain from README.md or docs/README.md reaches, on purpose (docs/README.md links these folders).
ORPHAN_OK = [
    ("docs/COMBOS.md", "a 'Moved' stub kept at the old path: older release pages and source comments cite it"),
    ("docs/FRAME-STRIP.md", "a 'Moved' stub kept at the old path: older release pages and source comments cite it"),
    ("docs/RECORDING.md", "a 'Moved' stub kept at the old path: older release pages and source comments cite it"),
    ("docs/decisions/*", "project records: docs/README.md links the folder"),
    ("docs/design/*", "project records: docs/README.md links the folder"),
    ("docs/provenance/*", "project records: docs/README.md links the folder"),
    ("docs/test-provenance/*", "project records: docs/README.md links the folder"),
    ("docs/menu-mockup/*", "project records: docs/README.md links the folder"),
]

# Sentences that say "newest" or "current" about something other than the build, or are quoted from a past state.
# (path, text the sentence contains, reason). An entry whose text is gone fails as `allowlist-stale`.
CLAIM_OK = [
    (
        "docs/RESTRICTIONS.md",
        "Replay Mii loading is unavailable in this build",
        "the quoted refusal text itself says 'this build'; the row only names rc.10, the release that removed the switch",
    ),
]

# Pages and sections that must name the current release: (path, heading text or None for the whole page, or "top" for
# the part before the first heading after the title).
MUST_NAME = [
    ("README.md", "top"),
    (CURRENT_PAGE, None),
    ("docs/guide/getting-started.md", None),
    ("docs/TECHNICAL.md", "Status"),
]

CLAIM_WORDS = re.compile(
    r"\b(?:newest|latest|currently|current (?:release|build|version|rc|candidate)|this (?:release|build|version)|up to date)\b",
    re.I,
)
RELEASE_TOKEN = re.compile(r"(?<![A-Za-z0-9])rc\.?(\d+)(?:\.\d+)*(?![A-Za-z0-9])", re.I)


class Problem(NamedTuple):
    rule: str
    path: str
    line: int
    text: str

    def __str__(self):
        return f"{self.rule} {self.path}:{self.line}: {self.text}"


class Config(NamedTuple):
    historical: list = HISTORICAL
    orphan_ok: list = ORPHAN_OK
    claim_ok: list = CLAIM_OK
    must_name: list = MUST_NAME


class Link(NamedTuple):
    line: int
    target: str


# --- reading and masking ----------------------------------------------------------------------------------------------


def markdown_files(root):
    """README.md and every .md under docs/, as sorted POSIX paths relative to the root."""
    found = []
    if os.path.isfile(os.path.join(root, "README.md")):
        found.append("README.md")
    for folder, dirs, files in os.walk(os.path.join(root, "docs")):
        dirs[:] = sorted(d for d in dirs if not d.startswith("."))
        for name in sorted(files):
            if name.endswith(".md"):
                found.append(os.path.relpath(os.path.join(folder, name), root).replace(os.sep, "/"))
    return sorted(found)


def read(root, rel):
    with open(os.path.join(root, rel), encoding="utf-8") as handle:
        return handle.read().replace("\r\n", "\n")


def _blank(text):
    """The text with everything but newlines turned into spaces, so offsets and line numbers survive."""
    return re.sub(r"[^\n]", " ", text)


FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})")


def mask_blocks(text):
    """Fenced code blocks and HTML comments blanked out, line numbers kept."""
    out, fence = [], None
    for line in text.split("\n"):
        match = FENCE.match(line)
        if fence is None and match:
            fence = match.group(1)
            out.append(_blank(line))
        elif fence is not None:
            out.append(_blank(line))
            if match and match.group(1)[0] == fence[0] and len(match.group(1)) >= len(fence) and not line.strip(" `~"):
                fence = None
        else:
            out.append(line)
    return re.sub(r"<!--.*?-->", lambda m: _blank(m.group(0)), "\n".join(out), flags=re.S)


CODE_SPAN = re.compile(r"(?<!`)(`+)(?!`)((?:[^\n]|\n(?!\s*\n))+?)(?<!`)\1(?!`)")


def mask_inline_code(text):
    return CODE_SPAN.sub(lambda m: _blank(m.group(0)), text)


def line_of(starts, offset):
    return bisect.bisect_right(starts, offset)


def line_starts(text):
    starts = [0]
    for match in re.finditer(r"\n", text):
        starts.append(match.end())
    return starts


# --- links ------------------------------------------------------------------------------------------------------------


def parse_destination(text, i):
    """The link destination that starts at text[i] (just after `(`) and the index after the closing `)`, or None."""
    n = len(text)
    while i < n and text[i] in " \t\n":
        i += 1
    if i < n and text[i] == "<":
        end = text.find(">", i + 1)
        if end < 0 or "\n" in text[i:end]:
            return None
        dest, i = text[i + 1 : end], end + 1
    else:
        depth, j = 0, i
        while j < n:
            c = text[j]
            if c == "\\" and j + 1 < n:
                j += 2
                continue
            if c in " \t\n":
                break
            if c == "(":
                depth += 1
            elif c == ")":
                if depth == 0:
                    break
                depth -= 1
            j += 1
        dest, i = text[i:j], j
    while i < n and text[i] in " \t\n":
        i += 1
    if i < n and text[i] in "\"'(":
        close = {'"': '"', "'": "'", "(": ")"}[text[i]]
        end = text.find(close, i + 1)
        if end < 0:
            return None
        i = end + 1
        while i < n and text[i] in " \t\n":
            i += 1
    if i < n and text[i] == ")":
        return dest, i + 1
    return None


HTML_ATTR = re.compile(r"""<(?:img|a|source|video|audio)\b[^>]*?\s(?:src|href)\s*=\s*(?:"([^"]*)"|'([^']*)')""", re.I)
REF_DEF = re.compile(r"^ {0,3}\[([^\]\n]+)\]:[ \t]*(<[^>\n]*>|\S+)", re.M)
REF_USE = re.compile(r"\]\[([^\]\n]*)\]")


class Parsed(NamedTuple):
    links: list  # Link: inline links, images, HTML src/href, reference definitions
    refs: list  # (line, label) of every [text][label] use
    defs: set  # labels defined
    bad: list  # (line, text) of `](` that is no link


def parse_links(text):
    """Every link of one page: from the text with code blocks, comments and inline code blanked out."""
    plain = mask_inline_code(mask_blocks(text))
    starts = line_starts(plain)
    links, bad = [], []
    for match in re.finditer(r"\]\(", plain):
        parsed = parse_destination(plain, match.end())
        line = line_of(starts, match.start())
        if parsed is None:
            bad.append((line, plain[match.start() : match.start() + 40].split("\n")[0]))
        else:
            links.append(Link(line, parsed[0].strip()))
    for match in HTML_ATTR.finditer(plain):
        links.append(Link(line_of(starts, match.start()), (match.group(1) or match.group(2)).strip()))
    defs = set()
    for match in REF_DEF.finditer(plain):
        defs.add(match.group(1).strip().lower())
        links.append(Link(line_of(starts, match.start()), match.group(2).strip("<>")))
    refs = [(line_of(starts, m.start()), m.group(1).strip().lower()) for m in REF_USE.finditer(plain)]
    return Parsed(links, refs, defs, bad)


def external_links(root):
    """(url, path, line) of every absolute http(s) URL in the docs: link targets, and bare URLs (the forge links those)."""
    found = []
    for rel in markdown_files(root):
        text = read(root, rel)
        plain = mask_inline_code(mask_blocks(text))
        starts = line_starts(plain)
        seen = set()
        for link in parse_links(text).links:
            if re.match(r"https?://", link.target, re.I):
                found.append((link.target, rel, link.line))
                seen.add(link.target)
        for match in re.finditer(r"https?://[^\s<>\"'\]]+", plain):
            url = match.group(0)
            while url and url[-1] in ".,;:!?)":
                url = url[:-1]
            if url not in seen and "](" + url not in plain[max(0, match.start() - 2) : match.end() + 1]:
                found.append((url, rel, line_of(starts, match.start())))
    return found


# --- headings and slugs -----------------------------------------------------------------------------------------------


def inline_text(raw):
    """A heading's text as the reader sees it: images and links reduced to their text, code and emphasis marks dropped."""
    text = re.sub(r"!\[([^\]]*)\]\([^)]*\)", r"\1", raw)
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = re.sub(r"<[^>]+>", "", text)
    text = text.replace("`", "")
    text = re.sub(r"\*+", "", text)
    text = re.sub(r"(?<![A-Za-z0-9])_+|_+(?![A-Za-z0-9])", "", text)
    return html.unescape(text)


def slug(raw):
    """The heading's id on the forge, without its `user-content-` prefix: lower-case; every run of characters that are not
    letters, digits or `_` becomes one `-`; no `-` at either end."""
    out = []
    for ch in inline_text(raw).lower():
        if unicodedata.category(ch)[0] in "LMN" or ch == "_":
            out.append(ch)
        elif not out or out[-1] != "-":
            out.append("-")
    return "".join(out).strip("-")


ATX = re.compile(r"^ {0,3}(#{1,6})[ \t]+(.*?)(?:[ \t]+#+)?[ \t]*$")
SETEXT = re.compile(r"^ {0,3}(=+|-+)[ \t]*$")
NOT_PARAGRAPH = re.compile(r"^\s*(?:[|>#*+-]|\d+[.)]\s)|^\s*$")


def headings(text):
    """(line, level, raw text) of every heading, ATX and setext, outside code blocks and comments."""
    lines = mask_blocks(text).split("\n")
    found = []
    for number, line in enumerate(lines, 1):
        atx = ATX.match(line)
        if atx:
            found.append((number, len(atx.group(1)), atx.group(2)))
            continue
        under = SETEXT.match(line)
        if under and number > 1 and not NOT_PARAGRAPH.match(lines[number - 2]) and not ATX.match(lines[number - 2]):
            found.append((number - 1, 1 if under.group(1)[0] == "=" else 2, lines[number - 2].strip()))
    return found


def anchors_of(text):
    """Every anchor name a page defines: heading slugs (with -1, -2 for repeats) and <a id/name>."""
    seen, names = {}, set()
    for _, _, raw in headings(text):
        base = slug(raw)
        count = seen.get(base, 0)
        seen[base] = count + 1
        names.add(base if count == 0 else f"{base}-{count}")
    for match in re.finditer(r"""<a\b[^>]*?\s(?:id|name)\s*=\s*["']([^"']+)["']""", mask_blocks(text), re.I):
        names.add(match.group(1))
    return names


# --- the tree ---------------------------------------------------------------------------------------------------------


class Tree:
    def __init__(self, root):
        self.root = root
        self.files = markdown_files(root)
        self._texts, self._anchors, self._listing = {}, {}, {}

    def text(self, rel):
        if rel not in self._texts:
            self._texts[rel] = read(self.root, rel)
        return self._texts[rel]

    def anchors(self, rel):
        if rel not in self._anchors:
            self._anchors[rel] = anchors_of(self.text(rel))
        return self._anchors[rel]

    def _names(self, folder):
        if folder not in self._listing:
            try:
                self._listing[folder] = set(os.listdir(os.path.join(self.root, folder)))
            except OSError:
                self._listing[folder] = set()
        return self._listing[folder]

    def exists(self, rel):
        """Whether the path exists with exactly this spelling, folder by folder."""
        folder = ""
        for part in rel.split("/"):
            if part not in self._names(folder):
                return False
            folder = f"{folder}/{part}" if folder else part
        return True


def resolve(source, target):
    """A link target as a repo path (`path`, `fragment`), or ('', fragment) for the same page, or None if it is not a
    relative link. A path that leaves the repo comes back as the string `..`."""
    if re.match(r"[A-Za-z][A-Za-z0-9+.-]*:", target) or target.startswith("//"):
        return None
    parts = urlsplit(target)
    path, fragment = unquote(parts.path), unquote(parts.fragment)
    if not path:
        return "", fragment
    if path.startswith("/"):
        return "/", fragment
    joined = posixpath.normpath(posixpath.join(posixpath.dirname(source), path))
    if joined == ".." or joined.startswith("../"):
        return "..", fragment
    return joined, fragment


def check_links(tree, problems, graph):
    for rel in tree.files:
        parsed = parse_links(tree.text(rel))
        for line, snippet in parsed.bad:
            problems.append(Problem("link-syntax", rel, line, f"`{snippet.strip()}` is not a link the forge can read"))
        for line, label in parsed.refs:
            if label and label not in parsed.defs:
                problems.append(Problem("link-ref", rel, line, f"no `[{label}]: target` line defines this reference"))
        for link in parsed.links:
            resolved = resolve(rel, link.target)
            if resolved is None:
                continue
            path, fragment = resolved
            if path == "/":
                problems.append(Problem("link-missing", rel, link.line, f"`{link.target}` is a path on a machine, not a link in the repo"))
                continue
            if path == "..":
                problems.append(Problem("link-missing", rel, link.line, f"`{link.target}` leaves the repository"))
                continue
            if path == "":
                target_file = rel
            elif path == ".":
                continue
            else:
                target_file = path
                if not tree.exists(target_file):
                    problems.append(Problem("link-missing", rel, link.line, f"`{link.target}`: no such file or folder"))
                    continue
            if target_file.endswith(".md") and target_file in tree.files:
                graph.setdefault(rel, set()).add(target_file)
            if fragment and target_file.endswith(".md") and os.path.isfile(os.path.join(tree.root, target_file)):
                if fragment not in tree.anchors(target_file):
                    problems.append(Problem("link-anchor", rel, link.line, f"`{link.target}`: {target_file} has no heading `#{fragment}`"))


def matches(path, patterns):
    return [(p, why) for p, why in patterns if fnmatch.fnmatchcase(path, p)]


def check_orphans(tree, config, problems, graph, used):
    reached, queue = set(), [i for i in INDEXES if i in tree.files]
    while queue:
        page = queue.pop()
        if page in reached:
            continue
        reached.add(page)
        queue.extend(sorted(graph.get(page, ())))
    for rel in tree.files:
        if rel in reached or not rel.startswith("docs/"):
            continue
        allowed = matches(rel, config.orphan_ok)
        if allowed:
            used.add(("orphan_ok", allowed[0][0]))
        else:
            problems.append(Problem("orphan", rel, 1, "no link from README.md or docs/README.md reaches this page, directly or through other pages"))


# --- versions ---------------------------------------------------------------------------------------------------------


def crate_version(root):
    path = os.path.join(root, "Cargo.toml")
    if not os.path.isfile(path):
        return ""
    match = re.search(r'^version\s*=\s*"([^"]+)"', read(root, "Cargo.toml"), re.M)
    return match.group(1) if match else ""


def current_release(tree, problems):
    """The release named by the one `Current release: rc.N` line of docs/CURRENT-RELEASE.md, or None."""
    if CURRENT_PAGE not in tree.files:
        problems.append(Problem("version-line", CURRENT_PAGE, 1, "the page does not exist: it holds the one `Current release: rc.N` line"))
        return None
    text = mask_blocks(tree.text(CURRENT_PAGE))
    found = CURRENT_LINE.findall(text)
    if len(found) != 1:
        problems.append(Problem("version-line", CURRENT_PAGE, 1, f"expected exactly one `Current release: rc.N` line, found {len(found)}"))
        return None
    return found[0]


def names_release(text, release):
    number = release.split(".")[1]
    return any(m.group(1) == number for m in RELEASE_TOKEN.finditer(text))


def section(text, heading):
    """The text of one part of a page: `top` is everything before its second heading; otherwise the section under the
    heading of that text, up to the next heading of the same or a higher level. None if the heading is missing."""
    found = headings(text)
    lines = text.split("\n")
    if heading == "top":
        end = found[1][0] - 1 if len(found) > 1 else len(lines)
        return "\n".join(lines[:end])
    for index, (number, level, raw) in enumerate(found):
        if inline_text(raw).strip() == heading:
            end = len(lines)
            for later, later_level, _ in found[index + 1 :]:
                if later_level <= level:
                    end = later - 1
                    break
            return "\n".join(lines[number - 1 : end])
    return None


def blocks(text):
    """(line, text) of each paragraph, list item and table row of a page, code blocks and comments left out."""
    out, current, start = [], [], 0
    for number, line in enumerate(mask_blocks(text).split("\n"), 1):
        starts_block = bool(re.match(r"^\s*(?:[-*+]|\d+[.)])\s|^\s*\||^#{1,6}\s", line))
        if not line.strip() or starts_block:
            if current:
                out.append((start, " ".join(current)))
            current = []
        if line.strip():
            if not current:
                start = number
            current.append(line.strip())
    if current:
        out.append((start, " ".join(current)))
    return out


def sentences(block):
    """The sentences of a block; a table row is one sentence, so "Newest | rc.13" in two cells is read together."""
    if block.lstrip().startswith("|"):
        yield block.strip()
        return
    for part in re.split(r"(?<=[.!?])\s+(?=[A-Z0-9*_\[`(<\"])", block):
        if part.strip():
            yield part.strip()


def claim_text(sentence, crate):
    """The sentence as the reader sees it: link targets and bare URLs gone, the crate version (0.2.0-rc.6) gone."""
    text = re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", sentence)
    text = re.sub(r"https?://\S+", "", text)
    text = text.replace("`", "")
    return text.replace(crate, "") if crate else text


def check_versions(tree, config, problems, used):
    release = current_release(tree, problems)
    if release is None:
        return
    number = release.split(".")[1]
    crate = crate_version(tree.root)
    for rel in tree.files:
        if matches(rel, config.historical):
            used.add(("historical", matches(rel, config.historical)[0][0]))
            continue
        for line, block in blocks(tree.text(rel)):
            for sentence in sentences(block):
                text = claim_text(sentence, crate)
                if not CLAIM_WORDS.search(text):
                    continue
                named = {m.group(1) for m in RELEASE_TOKEN.finditer(text)}
                other = sorted(named - {number}, key=int)
                if not other:
                    continue
                excused = [(p, t, why) for p, t, why in config.claim_ok if p == rel and t in sentence]
                if excused:
                    used.add(("claim_ok", excused[0][0], excused[0][1]))
                    continue
                problems.append(Problem(
                    "version-stale", rel, line,
                    f"says {CLAIM_WORDS.search(text).group(0)!r} and names rc.{other[0]}, but the current release is {release}: {sentence[:110]}",
                ))
    for rel, where in config.must_name:
        if rel not in tree.files:
            problems.append(Problem("version-name", rel, 1, "this entry point does not exist"))
            continue
        text = tree.text(rel)
        part = text if where is None else section(text, where)
        label = rel if where is None else f"{rel} ({where})"
        if part is None:
            problems.append(Problem("version-name", rel, 1, f"has no `{where}` section to name {release} in"))
        elif not names_release(claim_text(part, crate), release):
            problems.append(Problem("version-name", rel, 1, f"{label} does not name the current release, {release}"))
        if rel == "README.md" and part is not None:
            links = [link.target for link in parse_links(part).links]
            if not any((resolve(rel, t) or ("",))[0] == CURRENT_PAGE for t in links):
                problems.append(Problem("version-name", rel, 1, f"the README's top does not link to {CURRENT_PAGE}"))


def check_allowlists(tree, config, used, problems):
    for name, entries in (("historical", config.historical), ("orphan_ok", config.orphan_ok)):
        for pattern, _ in entries:
            if not any(fnmatch.fnmatchcase(rel, pattern) for rel in tree.files):
                problems.append(Problem("allowlist-stale", "tools/docs_check.py", 1, f"{name} entry `{pattern}` matches no page"))
    for path, snippet, _ in config.claim_ok:
        if path not in tree.files or snippet not in tree.text(path):
            problems.append(Problem("allowlist-stale", "tools/docs_check.py", 1, f"claim_ok entry for {path}: the text `{snippet[:50]}` is gone"))


def check(root, config=None, used=None):
    """Every problem of the tree under `root`, sorted."""
    config = config or Config()
    used = set() if used is None else used
    tree = Tree(root)
    problems, graph = [], {}
    check_links(tree, problems, graph)
    check_orphans(tree, config, problems, graph, used)
    check_versions(tree, config, problems, used)
    check_allowlists(tree, config, used, problems)
    return sorted(set(problems), key=lambda p: (p.path, p.line, p.rule, p.text))


def main(argv=None):
    parser = argparse.ArgumentParser(description="Offline checks of README.md and docs/**/*.md")
    parser.add_argument("--root", default=os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    parser.add_argument("--verbose", action="store_true", help="also list the allowlist entries that were used")
    args = parser.parse_args(argv)
    if not os.path.isdir(os.path.join(args.root, "docs")):
        print(f"no docs/ folder under {args.root}", file=sys.stderr)
        return 2
    used = set()
    try:
        problems = check(args.root, used=used)
    except (OSError, UnicodeDecodeError) as error:
        print(f"could not read the docs: {error}", file=sys.stderr)
        return 2
    for problem in problems:
        print(problem)
    pages = len(markdown_files(args.root))
    print(f"docs_check: {pages} pages, {len(problems)} problems")
    if args.verbose:
        for entry in sorted(used, key=str):
            print("allowed:", *entry)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
