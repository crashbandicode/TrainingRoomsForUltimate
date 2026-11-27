"""Offline docs gates for README.md and docs/**/*.md: links, anchors, orphan pages and version claims.

Ordered 2026-11-15: checks for stale links and outdated version references, run as gates. Run by
`tests/docs_gates.rs` on every `cargo test` (`python3 -I tools/docs_check.py [--root DIR] [--verbose]`). It reads only the
files under the root: no network and no clock, so every host gives the same answer. With a `.git`, it also reads the
tracked paths and, for skip-worktree media files absent from disk, their indexed bytes: gate hosts leave docs/media and
every .gif out of the work tree (`tracked_paths`). Exit 0: clean; 1: problems
(printed one per line as `rule path:line: message`); 2: it could not run (no root, unreadable file).
`tools/check_external_links.py` is the network check (a release step); it reuses `external_links` below.

Rules (a problem's rule name is the first word of its line):

  link-missing     a relative link or image whose target file or folder does not exist. The case must match too, also
                   on a host whose file system ignores it, because the forge's does not. A target that leaves the repo
                   (`../..`) or starts with `/` (a path on someone's machine, not on the forge) is `link-missing` too.
  link-anchor      `#anchor` (alone, or after a .md file) with no heading or `<a id/name>` of that name in the target.
  link-ref         `[text][label]` or `[label][]` whose `[label]: target` is not defined in the same file.
  link-syntax      a `](` that does not form a link (the forge would print it as plain text).
  video-src        a `<video>` must use the forge's media blob URL with the git blob id of an .mp4 in docs/media/.
  orphan           a .md page under docs/ that no chain of links from README.md or docs/README.md reaches. A link to a
                   folder does not reach the pages in it; the folders of project records are listed in `ORPHAN_OK`.
  version-line     docs/CURRENT-RELEASE.md has no `Current release: rc.N` line, or more than one.
  version-name     an entry point (`MUST_NAME`: the README's top, CURRENT-RELEASE.md, the Getting started guide and the
                   Status section of TECHNICAL.md) does not name the current release, or the README's top does not link
                   to docs/CURRENT-RELEASE.md.
  version-stale    a sentence that says "newest", "latest", "currently", "current release/build/version", "this
                   release/build/version" or "up to date" and names another release (rc.N or rcN, also 0.2.0-rc.N).
  version-cargo    the crate version in Cargo.toml (`0.2.0-rc.N`) does not name the current release: the menu footer
                   starts with it (`0.2.0-rc.14 · rc14 <commit>`), and the release tag is `v0.2.0-rc.14`. Decided
                   2026-11-15: the footer and the tag agree. The bump (Cargo.toml, Cargo.lock's own entry) goes in the
                   release's name commit with build.rs's BUILD_NAME, never before the release is the current one.
  notice-text      a "double-quoted" text that must be a notice, refusal or greyed-row reason of the plugin, but is not a
                   string literal of the code the release build compiles in src/ (or crates/): test code (a `tests/`
                   folder, `*_tests.rs`, `#[cfg(test)]`, by tools/sloc.py's evaluator, the size gate's rule), host-only
                   simulator code and diag-only code do not count, because the player never sees what only they hold.
                   On `docs/guide/troubleshooting.md` every quoted text; on any other
                   player page a quoted text in a sentence that says notice, refus*, message, help line, reason or
                   greyed; in docs/RESTRICTIONS.md the quoted texts of the Message column. A text matches a literal when
                   it is part of it, or fits its `{}` holes ("Loading 6/10" fits "Loading {}/{}"); text around a ‹name›,
                   a {hole} or an ellipsis is matched piece by piece, "Header · Body" per part, and "save (load) again"
                   as "save again" and "load again". Comments and char literals of the Rust source do not count. This
                   proves the wording exists in shipped code, not that the notice shows in the situation the page describes.
                   Texts built from pieces at run time go in `NOTICE_OK` with the reason.
  wall             a wall of text on a menu or guide page (`WALL_PAGES`): a caption (a block that starts with one `*`)
                   of more than 120 visible characters, its "Open the clip" link not counted; a paragraph or list item
                   of more than two sentences, or of more than 300 visible characters; a list item that starts with
                   **Values:**, **Default:** or **How:**. Table rows, headings, code blocks, HTML and an image's alt
                   text are not read. A page still to be rewritten is in `WALL_PENDING` with its count, which only falls.
  budget           a menu or guide page over its word budget (`BUDGET_WORDS`): more than 1,500 words on a menu page or
                   800 on a guide page, counted as the file's text split on white space, its visuals (video lines,
                   images, captions) left out; a table cell of more than 15
                   words; a caption of more than 60 visible characters; a section (the text under one heading) with
                   more than one sentence of prose; a second "Good to know" heading, or more than five rows under it;
                   a `##` section of a menu page that does not open with a clip or picture; a guide page with none.
                   A page not yet cut is in `BUDGET_PENDING` with its word count, which only falls.
  allowlist-stale  an entry of the lists below that matches no file, or whose sentence or text is gone.

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
import hashlib
import html
import os
import posixpath
import re
import subprocess
import sys
import unicodedata
from typing import NamedTuple
from urllib.parse import unquote, urlsplit

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # python3 -I leaves the script's folder out
import sloc  # noqa: E402

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

# Quoted texts that are real notices but are built from pieces at run time, so no one literal holds them (`notice-text`).
# (path, the quoted text, reason). An entry whose text is gone from the page fails as `allowlist-stale`.
NOTICE_OK = [
    ("docs/TECHNICAL.md", "Same as 格斗, nothing new saved", "the notice with a Mii's name filled in: src/model/notify.rs joins 'Same as ', the name and ', nothing new saved'"),
    ("docs/guide/cpu-behaviour.md", "Selected: Up x3, In x1", "a help line built per chip: src/model/ui/screens.rs writes 'Selected: ' then '<direction> x<weight>' for each"),
    ("docs/menu/cpu.md", "Selected: Up x2, Out x1", "the same help line, built per chip in src/model/ui/screens.rs"),
    ("docs/RESTRICTIONS.md", "Combos work in Training", "a retired restriction: the row says the string is gone from src/"),
]

# Pages whose every "double-quoted" text must be a notice text; and pages where only one table column is checked
# (page -> column number, 1 is the first): RESTRICTIONS' Message column, not the approvals in Approved.
NOTICE_PAGES = ["docs/guide/troubleshooting.md"]
NOTICE_COLUMNS = {"docs/RESTRICTIONS.md": 2}
NOTICE_WORDS = re.compile(r"notice|refus|message|help line|reason|greyed", re.I)

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


# Walls of text (`wall`): the player pages the rule reads, its limits, and the pages not yet rewritten.
# Ordered 2026-11-27 (operator, through tr-primary-2 02:33:44Z): a caption is one short line, a paragraph is at most
# two short sentences, a setting's Values / Default / How bullets are a table.
WALL_PAGES = ["docs/menu/*.md", "docs/guide/*.md"]
WALL_CAPTION = 120
WALL_SENTENCES = 2
WALL_CHARS = 300
WALL_BULLETS = re.compile(r"^\*\*(?:Values|Default|How):\*\*")
# page -> the number of `wall` problems it may still have while the pages are rewritten one by one. A page with more
# fails with every one of them listed; a page with fewer fails as `allowlist-stale`, so a number only falls and a
# finished page leaves the list. An empty list means every page is held to the rule.
WALL_PENDING = {
    # Counted on 2026-11-27 at the commit that added the rule (739 on 27 pages). Rewritten in this order: the menu pages
    # (Fighters & Stages first), then the guide.
    "docs/guide/break-the-targets.md": 3,
    "docs/guide/combos.md": 13,
    "docs/guide/controls.md": 9,
    "docs/guide/cpu-behaviour.md": 13,
    "docs/guide/eggs-ercise.md": 3,
    "docs/guide/fighter-cycles.md": 8,
    "docs/guide/frame-strip.md": 9,
    "docs/guide/game-speed.md": 6,
    "docs/guide/getting-started.md": 4,
    "docs/guide/guard-the-edge.md": 4,
    "docs/guide/juggler.md": 4,
    "docs/guide/mash-out.md": 3,
    "docs/guide/mii.md": 8,
    "docs/guide/parry-practice.md": 1,
    "docs/guide/recording.md": 76,
    "docs/guide/save-states.md": 9,
    "docs/guide/stages.md": 7,
    "docs/guide/tech-practice.md": 3,
    "docs/guide/troubleshooting.md": 2,
    "docs/guide/using-the-menu.md": 8,
    "docs/guide/waiting-rooms.md": 4,
    "docs/menu/cpu.md": 134,
    "docs/menu/display-and-controls.md": 69,
    "docs/menu/practice-games.md": 3,
    "docs/menu/saves.md": 47,
}


# Word budgets (`budget`). Ordered 2026-11-27 (operator, through tr-primary-2 03:16:29Z): the pages were too long.
# Words are counted as the order counted them: the file's text split on white space, tables and links included.
BUDGET_WORDS = [("docs/menu/*.md", 1500), ("docs/guide/*.md", 800)]
BUDGET_CELL = 15
BUDGET_CAPTION = 60
BUDGET_SENTENCES = 1
BUDGET_GOOD_ROWS = 5
# Visuals (operator, through tr-primary-2 03:32:34Z): on a menu page every `##` section opens with one embedded clip or
# picture; a guide page has at least one. A visual (its video or image line and its caption) is not counted as words.
BUDGET_VISUALS = {"docs/menu/*.md": "sections", "docs/guide/*.md": "page"}
# page -> its word count, for the pages not yet cut to the budget. Such a page is not held to the rule, but it may not
# grow, a shorter page lowers its number, and a page that meets the budget leaves the list. Empty before a release.
BUDGET_PENDING = {
    "docs/guide/break-the-targets.md": 215,
    "docs/guide/combos.md": 2192,
    "docs/guide/controls.md": 841,
    "docs/guide/cpu-behaviour.md": 1060,
    "docs/guide/eggs-ercise.md": 307,
    "docs/guide/fighter-cycles.md": 733,
    "docs/guide/frame-strip.md": 983,
    "docs/guide/game-speed.md": 367,
    "docs/guide/getting-started.md": 518,
    "docs/guide/guard-the-edge.md": 263,
    "docs/guide/juggler.md": 246,
    "docs/guide/mash-out.md": 133,
    "docs/guide/mii.md": 582,
    "docs/guide/parry-practice.md": 157,
    "docs/guide/recording.md": 6118,
    "docs/guide/save-states.md": 848,
    "docs/guide/stages.md": 584,
    "docs/guide/tech-practice.md": 230,
    "docs/guide/troubleshooting.md": 1408,
    "docs/guide/using-the-menu.md": 647,
    "docs/guide/waiting-rooms.md": 605,
    "docs/menu/cpu.md": 3764,
    "docs/menu/display-and-controls.md": 3394,
    "docs/menu/practice-games.md": 1589,
    "docs/menu/recording.md": 9606,
    "docs/menu/saves.md": 2195,
}


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
    notice_ok: list = NOTICE_OK
    notice_pages: list = NOTICE_PAGES
    notice_columns: dict = NOTICE_COLUMNS
    wall_pages: list = WALL_PAGES
    wall_pending: dict = WALL_PENDING
    budget_words: list = BUDGET_WORDS
    budget_pending: dict = BUDGET_PENDING
    budget_visuals: dict = BUDGET_VISUALS


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
        target = match.group(1) if match.group(1) is not None else match.group(2)
        links.append(Link(line_of(starts, match.start()), target.strip()))
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


def tracked_paths(root):
    """(files, folders) as git lists them: tracked, plus untracked and not ignored. None when `root` has no `.git`.

    The gate hosts build from a stage that leaves `docs/media` and every `*.gif` out of the work tree (remote-gate.ps1,
    skip-worktree), so a link to a clip must be judged by what the commit holds, not by what is on that disk."""
    if not os.path.exists(os.path.join(root, ".git")):
        return None
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    try:
        listing = subprocess.run(["git", "-C", root, "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
                                 capture_output=True, check=True, env=env, timeout=120).stdout
    except subprocess.CalledProcessError as error:
        raise OSError(f"git ls-files failed in {root}: {error.stderr.decode('utf-8', 'replace').strip()}") from error
    except subprocess.SubprocessError as error:
        raise OSError(f"git ls-files did not finish in {root}: {error}") from error
    files = {path for path in listing.decode("utf-8").split("\0") if path}
    folders = {""}
    for path in files:
        parts = path.split("/")[:-1]
        folders.update("/".join(parts[:n]) for n in range(1, len(parts) + 1))
    return files, folders


class Tree:
    def __init__(self, root):
        self.root = root
        self.files = markdown_files(root)
        self.listed = tracked_paths(root)
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
        """Whether the path exists with exactly this spelling: in git's list when the root is a git work tree (see
        `tracked_paths`), else on disk, folder by folder."""
        if self.listed is not None:
            return rel in self.listed[0] or rel in self.listed[1]
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


# --- video sources ----------------------------------------------------------------------------------------------------


VIDEO_BLOB = "https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/"
VIDEO_URL = re.compile(re.escape(VIDEO_BLOB) + r"([0-9a-fA-F]{40})")
VIDEO_TAG = re.compile(r"""<video(?=[\s/>])(?:[^>"']|"[^"]*"|'[^']*')*>""", re.I)
VIDEO_ATTRIBUTE = re.compile(r"""([^\s"'<>/=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?""")


def blob_id(data):
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def media_blobs(tree):
    """Use committed identities even in sparse checkouts; a plain export has only its file bytes."""
    if tree.listed is not None:
        env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}

        def records(*args):
            result = subprocess.run(["git", "-C", tree.root, *args], capture_output=True,
                                    check=True, env=env, timeout=120)
            return result.stdout.split(b"\0")

        try:
            sparse = {entry[2:] for entry in records("ls-files", "-v", "-z", "--", "docs/media/")
                      if entry[:2].upper() == b"S "}
            found = set()
            for entry in records("ls-tree", "-r", "-z", "HEAD", "--", "docs/media/"):
                if not entry.endswith(b".mp4"):
                    continue
                meta, path = entry.split(b"\t", 1)
                mode, _, oid = meta.split()
                if mode in (b"100644", b"100755") and (
                        path in sparse or os.path.isfile(os.path.join(tree.root, os.fsdecode(path)))):
                    found.add(oid.decode("ascii"))
            return found
        except subprocess.SubprocessError as error:
            raise OSError(f"could not read committed media in {tree.root}: {error}") from error

    found = set()
    for folder, _, files in os.walk(os.path.join(tree.root, "docs", "media")):
        for name in files:
            if not name.endswith(".mp4"):
                continue
            path = os.path.join(folder, name)
            with open(path, "rb") as handle:
                found.add(blob_id(handle.read()))
    return found


def check_video_sources(tree, problems):
    blobs = None
    for rel in tree.files:
        plain = mask_inline_code(mask_blocks(tree.text(rel)))
        starts = line_starts(plain)
        for tag in VIDEO_TAG.finditer(plain):
            attributes = VIDEO_ATTRIBUTE.finditer(tag.group(0)[len("<video"):-1])
            sources = [next((value for value in attr.groups()[1:] if value is not None), None)
                       for attr in attributes if attr.group(1).lower() == "src"]
            source = sources[0] if len(sources) == 1 else None
            url = VIDEO_URL.fullmatch(source) if source is not None else None
            line = line_of(starts, tag.start())
            if url is None:
                problems.append(Problem("video-src", rel, line, "use the media blob URL of the clip"))
                continue
            if blobs is None:
                blobs = media_blobs(tree)
            if url.group(1).lower() not in blobs:
                problems.append(Problem("video-src", rel, line,
                                        f"blob {url.group(1)} is not an .mp4 in docs/media/; use the media blob URL of the clip"))


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


def lock_version(root):
    """The version Cargo.lock records for the root package (the name in Cargo.toml), or "" without either file."""
    if not (os.path.isfile(os.path.join(root, "Cargo.toml")) and os.path.isfile(os.path.join(root, "Cargo.lock"))):
        return ""
    name = re.search(r'^name\s*=\s*"([^"]+)"', read(root, "Cargo.toml"), re.M)
    entry = re.search(rf'\[\[package\]\]\nname = "{re.escape(name.group(1))}"\nversion = "([^"]+)"', read(root, "Cargo.lock")) if name else None
    return entry.group(1) if entry else ""


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


def claim_text(sentence):
    """The sentence as the reader sees it: link targets and bare URLs gone, backticks gone."""
    text = re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", sentence)
    text = re.sub(r"https?://\S+", "", text)
    return text.replace("`", "")


def check_versions(tree, config, problems, used):
    release = current_release(tree, problems)
    if release is None:
        return
    number = release.split(".")[1]
    crate = crate_version(tree.root)
    if crate and not re.fullmatch(rf"\d+\.\d+\.\d+-rc\.{number}", crate):
        problems.append(Problem(
            "version-cargo", "Cargo.toml", 1,
            f"the crate version is {crate}, but the current release is {release}: the menu footer starts with the crate version, "
            f"so it must read 0.2.0-{release} (Cargo.lock's own entry too)",
        ))
    locked = lock_version(tree.root)
    if crate and locked and locked != crate:
        problems.append(Problem("version-cargo", "Cargo.lock", 1, f"the lock file has this package at {locked}, but Cargo.toml says {crate}"))
    for rel in tree.files:
        if matches(rel, config.historical):
            used.add(("historical", matches(rel, config.historical)[0][0]))
            continue
        for line, block in blocks(tree.text(rel)):
            for sentence in sentences(block):
                text = claim_text(sentence)
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
        elif not names_release(claim_text(part), release):
            problems.append(Problem("version-name", rel, 1, f"{label} does not name the current release, {release}"))
        if rel == "README.md" and part is not None:
            links = [link.target for link in parse_links(part).links]
            if not any((resolve(rel, t) or ("",))[0] == CURRENT_PAGE for t in links):
                problems.append(Problem("version-name", rel, 1, f"the README's top does not link to {CURRENT_PAGE}"))


# --- notice texts -----------------------------------------------------------------------------------------------------

_TOKEN = re.compile(r'//[^\n]*|/\*|r#*"|"|\'')
_STRING_END = re.compile(r'(?:[^"\\]|\\.)*"', re.S)
_CHAR = re.compile(r"'(?:\\[^']+|[^\\'])'")
_ESCAPES = {"n": "\n", "t": "\t", "\\": "\\", '"': '"', "'": "'", "0": "\0", "r": "\r"}


def _unescape(body):
    body = re.sub(r"\\\n\s*", "", body)
    body = re.sub(r"\\u\{([0-9a-fA-F_]+)\}", lambda m: chr(int(m.group(1).replace("_", ""), 16)), body)
    body = re.sub(r"\\x([0-9a-fA-F]{2})", lambda m: chr(int(m.group(1), 16)), body)
    return re.sub(r"\\(.)", lambda m: _ESCAPES.get(m.group(1), m.group(1)), body, flags=re.S)


def rust_strings(src):
    """The text of every string literal in Rust source (plain, byte and raw); comments, char literals and lifetimes are
    skipped, so a quote inside them cannot throw the pairing off."""
    out, pos, size = [], 0, len(src)
    while True:
        token = _TOKEN.search(src, pos)
        if token is None:
            return out
        start, text = token.start(), token.group(0)
        pos = token.end()
        if text.startswith("//"):
            continue
        if text == "/*":
            depth = 1
            while depth and pos < size:
                opening, closing = src.find("/*", pos), src.find("*/", pos)
                if closing < 0:
                    pos = size
                elif 0 <= opening < closing:
                    depth, pos = depth + 1, opening + 2
                else:
                    depth, pos = depth - 1, closing + 2
        elif text.startswith("r"):
            if start and (src[start - 1].isalnum() or src[start - 1] == "_"):
                continue  # the tail of a name, not a raw string
            end = src.find('"' + "#" * text.count("#"), pos)
            if end < 0:
                return out
            out.append(src[pos:end])
            pos = end + 1 + text.count("#")
        elif text == '"':
            end = _STRING_END.match(src, pos)
            if end is None:
                return out
            out.append(_unescape(src[pos : end.end() - 1]))
            pos = end.end()
        else:
            char = _CHAR.match(src, start)
            pos = char.end() if char else start + 1


def _flat(text):
    return re.sub(r"\s+", " ", text).strip()


HOLE = re.compile(r"\{[^{}]*\}")
PIECES = re.compile(r"‹[^›]*›|\{[^{}]*\}|…| · ")  # a ‹name›, a {hole}, an ellipsis, or the "Header · Body" dot
EITHER = re.compile(r"\b([a-z]+) \(([a-z]+)\)(?= [a-z]|$)")  # "save (load) again", "to save (load)"
_corpora = {}


class Corpus(NamedTuple):
    text: str  # every literal, one per line, whitespace collapsed
    patterns: list  # a literal with `{}` holes as a regex: "Loading {}/{}" fits "Loading 6/10"


def release_sources(root):
    """{path: text} of the Rust code of src/ and crates/ that the release build compiles. tools/sloc.py's cfg evaluator
    decides (the size gate's rule): a `tests/` folder, a file or module behind `#[cfg(test)]` (`*_tests.rs`,
    `mod tests`), host-only simulator code and diag-only code are left out, so a text that only they hold is no text."""
    files = {p: t for p, t in sloc.files_tree(root).items() if p.endswith("Cargo.toml") or p.startswith(("src/", "crates/"))}
    release = sloc.Release({p: t for p, t in files.items() if p.endswith("Cargo.toml")})
    sources = {p: t for p, t in files.items() if p.endswith(".rs")}
    scans = {p: sloc.cfg_scan(t, release.decider(p)) for p, t in sources.items()}
    status = sloc.cfg_status(sources, scans)
    return {p: sloc.cut(t, scans[p][0]) for p, t in sources.items() if status[p] is None}


def corpus_of(root):
    """The string literals of the code the release build compiles under `root` (see `release_sources`)."""
    if root not in _corpora:
        found = set()
        for text in release_sources(root).values():
            found.update(s for s in map(_flat, rust_strings(text)) if len(s) >= 3)
        patterns = set()
        for literal in found:
            for variant in {literal, literal.rstrip(".")}:
                if HOLE.search(variant) and len(HOLE.sub("", variant).strip()) >= 5:
                    patterns.add("(?s)" + ".*?".join(re.escape(part) for part in HOLE.split(variant)))
        _corpora[root] = Corpus("\n".join(sorted(found)), [re.compile(p) for p in sorted(patterns)])
    return _corpora[root]


def _plain(text, corpus):
    return text in corpus.text or any(p.fullmatch(text) for p in corpus.patterns)


def text_exists(quote, corpus):
    """Whether a quoted text is in the code: see the `notice-text` rule at the top of this file."""
    quote = _flat(quote)
    for variant in ([quote] if not EITHER.search(quote) else [EITHER.sub(r"\1", quote), EITHER.sub(r"\2", quote)]):
        if _plain(variant, corpus):
            continue
        parts = [part.strip(" .,:;") for part in PIECES.split(variant)]
        big = [part for part in parts if len(part) >= 6]
        if len(parts) < 2 or not big or not all(_plain(part, corpus) for part in big):
            return False
    return True


QUOTE = re.compile(r'(?<![\w)])"([^"\n]{2,160}?)"(?![\w(])')


def notice_quotes(rel, text, config):
    """(line, text) of each double-quoted text on a page that has to be a notice (the rule's scope is at the top)."""
    column = config.notice_columns.get(rel)
    for line, block in blocks(text if column is not None else mask_inline_code(text)):
        if column is not None:
            cells = block.split("|")
            scope = [cells[column]] if block.lstrip().startswith("|") and len(cells) > column + 1 else []
        else:
            scope = [s for s in sentences(block) if rel in config.notice_pages or NOTICE_WORDS.search(s)]
        for sentence in scope:
            plain = re.sub(r"https?://\S+", "", re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", sentence))
            for match in QUOTE.finditer(plain):
                yield line, match.group(1)


def check_notices(tree, config, problems, used):
    wanted = []
    for rel in tree.files:
        if matches(rel, config.historical):
            continue
        wanted += [(rel, line, quote) for line, quote in notice_quotes(rel, tree.text(rel), config)]
    if not wanted:
        return
    corpus = corpus_of(tree.root)
    for rel, line, quote in wanted:
        excused = [(p, q) for p, q, _ in config.notice_ok if p == rel and q == _flat(quote)]
        if excused:
            used.add(("notice_ok", *excused[0]))
        elif not text_exists(quote, corpus):
            problems.append(Problem("notice-text", rel, line, f'"{quote}" is not a text in src/ (a reworded notice, or one built from pieces: NOTICE_OK)'))


def walls(text):
    """(line, what is wrong) of each wall of text on a page: a long caption, a long paragraph, a settings bullet."""
    for line, block in blocks(text):
        body = re.sub(r"!\[.*?\]\([^)]*\)(?:<br>)?", "", block).strip()
        body = re.sub(r"^(?:[-*+]|\d+[.)])\s+", "", body)
        if not body or body.startswith(("|", "#", "<")):
            continue
        if body.startswith("*") and not body.startswith("**"):
            shown = claim_text(re.sub(r"\[Open [^\]]*\]\([^)]*\)", "", body)).replace("*", "").strip()
            if len(shown) > WALL_CAPTION:
                yield line, f"a caption of {len(shown)} characters (at most {WALL_CAPTION}: one short line; the clip shows the rest)"
            continue
        if WALL_BULLETS.match(body):
            yield line, "a Values / Default / How bullet (make it a table: Value | What it does)"
            continue
        shown = claim_text(body).replace("*", "").strip()
        count = len(list(sentences(shown)))
        if count > WALL_SENTENCES:
            yield line, f"a paragraph of {count} sentences (at most {WALL_SENTENCES}: put options, keys and results in a table)"
        elif len(shown) > WALL_CHARS:
            yield line, f"a paragraph of {len(shown)} characters (at most {WALL_CHARS})"


def check_walls(tree, config, problems):
    for rel in tree.files:
        if not any(fnmatch.fnmatchcase(rel, pattern) for pattern in config.wall_pages):
            continue
        found = list(walls(tree.text(rel)))
        allowed = config.wall_pending.get(rel)
        if allowed is None or len(found) > allowed:
            problems.extend(Problem("wall", rel, line, text) for line, text in found)
            if allowed is not None:
                problems.append(Problem("wall", rel, 1, f"{len(found)} walls of text; the page was pending at {allowed}, and that number only falls"))
        elif len(found) < allowed:
            fix = f"lower it to {len(found)}" if found else "remove the entry"
            problems.append(Problem("allowlist-stale", "tools/docs_check.py", 1, f"wall_pending entry for {rel} is {allowed} but the page has {len(found)}: {fix}"))
    for rel in config.wall_pending:
        if rel not in tree.files:
            problems.append(Problem("allowlist-stale", "tools/docs_check.py", 1, f"wall_pending entry `{rel}` matches no page"))


def shown_words(text):
    """The words a reader sees in a table cell or a line: link targets, HTML tags and emphasis marks gone."""
    return claim_text(re.sub(r"<[^>]+>", "", text)).replace("*", "").split()


def counted_words(text):
    """The words that count toward a page's budget: its visuals (video lines, images and caption lines) left out."""
    kept = []
    for line in text.split("\n"):
        bare = line.strip()
        if bare.startswith("<video") or re.match(r"^\*[^*\s]", bare):
            continue
        kept.append(re.sub(r"!\[[^\]]*\]\([^)]*\)(?:<br>)?", "", line))
    return len(" ".join(kept).split())


def is_visual(block):
    return block.startswith(("<video", "!["))


def budget(text, limit, visuals=None):
    """(line, what is wrong) of each way a page is over its word budget or lacks its visuals."""
    words = counted_words(text)
    if words > limit:
        yield 1, f"the page has {words} words (at most {limit})"
    found = blocks(text)
    if visuals == "page" and not any(is_visual(block) for _, block in found):
        yield 1, "the page has no clip or picture (a guide page has at least one)"
    if visuals == "sections":
        for index, (line, block) in enumerate(found):
            if block.startswith("## ") and not (index + 1 < len(found) and is_visual(found[index + 1][1])):
                yield line, "the section does not open with a clip or picture"
    for number, line in enumerate(mask_blocks(text).split("\n"), 1):
        if not line.lstrip().startswith("|") or re.match(r"^\s*\|[\s:|-]+$", line):
            continue
        for cell in line.replace("\\|", " ").strip().strip("|").split("|"):
            count = len(shown_words(cell))
            if count > BUDGET_CELL:
                yield number, f"a table cell of {count} words (at most {BUDGET_CELL})"
    heading, prose, good, rows = 1, 0, [], {}
    for line, block in blocks(text):
        body = re.sub(r"!\[.*?\]\([^)]*\)(?:<br>)?", "", block).strip()
        if block.startswith("#"):
            if prose > BUDGET_SENTENCES:
                yield heading, f"{prose} sentences of prose in this section (at most {BUDGET_SENTENCES}, before its table, clip or picture)"
            heading, prose = line, 0
            if re.search(r"good to know", block, re.I):
                good.append(line)
            continue
        if body.startswith("|"):
            if good and heading == good[-1] and not re.match(r"^\|[\s:|-]+$", body):
                rows[heading] = rows.get(heading, 0) + 1
            continue
        body = re.sub(r"^(?:[-*+]|\d+[.)])\s+", "", body)
        if not body or body.startswith("<"):
            continue
        if body.startswith("*") and not body.startswith("**"):
            shown = claim_text(re.sub(r"\[Open [^\]]*\]\([^)]*\)", "", body)).replace("*", "").strip()
            if len(shown) > BUDGET_CAPTION:
                yield line, f"a caption of {len(shown)} characters (at most {BUDGET_CAPTION})"
            continue
        prose += len(list(sentences(claim_text(body).replace("*", "").strip())))
    if prose > BUDGET_SENTENCES:
        yield heading, f"{prose} sentences of prose in this section (at most {BUDGET_SENTENCES}, before its table, clip or picture)"
    for line in good[1:]:
        yield line, "a second \"Good to know\" table (at most one on a page)"
    for line, count in rows.items():
        if count - 1 > BUDGET_GOOD_ROWS:
            yield line, f"a \"Good to know\" table of {count - 1} rows (at most {BUDGET_GOOD_ROWS})"


def check_budgets(tree, config, problems):
    for rel in tree.files:
        limit = next((words for pattern, words in config.budget_words if fnmatch.fnmatchcase(rel, pattern)), None)
        if limit is None:
            continue
        text = tree.text(rel)
        visuals = next((kind for pattern, kind in config.budget_visuals.items() if fnmatch.fnmatchcase(rel, pattern)), None)
        found, words, allowed = list(budget(text, limit, visuals)), counted_words(text), config.budget_pending.get(rel)
        if allowed is None:
            problems.extend(Problem("budget", rel, line, what) for line, what in found)
        elif words > allowed:
            problems.append(Problem("budget", rel, 1, f"the page has {words} words; it was pending at {allowed}, and that number only falls"))
        elif not found:
            problems.append(Problem("allowlist-stale", "tools/docs_check.py", 1, f"budget_pending entry for {rel}: the page meets its budget, remove the entry"))
        elif words < allowed:
            problems.append(Problem("allowlist-stale", "tools/docs_check.py", 1, f"budget_pending entry for {rel} is {allowed} but the page has {words} words: lower it to {words}"))
    for rel in config.budget_pending:
        if rel not in tree.files:
            problems.append(Problem("allowlist-stale", "tools/docs_check.py", 1, f"budget_pending entry `{rel}` matches no page"))


def check_allowlists(tree, config, used, problems):
    for name, entries in (("historical", config.historical), ("orphan_ok", config.orphan_ok)):
        for pattern, _ in entries:
            if not any(fnmatch.fnmatchcase(rel, pattern) for rel in tree.files):
                problems.append(Problem("allowlist-stale", "tools/docs_check.py", 1, f"{name} entry `{pattern}` matches no page"))
    for path, snippet, _ in config.claim_ok:
        if path not in tree.files or snippet not in tree.text(path):
            problems.append(Problem("allowlist-stale", "tools/docs_check.py", 1, f"claim_ok entry for {path}: the text `{snippet[:50]}` is gone"))
    for path, quote, _ in config.notice_ok:
        if path not in tree.files or f'"{quote}"' not in _flat(tree.text(path)):
            problems.append(Problem("allowlist-stale", "tools/docs_check.py", 1, f"notice_ok entry for {path}: the text `{quote[:50]}` is gone"))


def check(root, config=None, used=None):
    """Every problem of the tree under `root`, sorted."""
    config = config or Config()
    used = set() if used is None else used
    tree = Tree(root)
    problems, graph = [], {}
    check_links(tree, problems, graph)
    check_video_sources(tree, problems)
    check_orphans(tree, config, problems, graph, used)
    check_versions(tree, config, problems, used)
    check_notices(tree, config, problems, used)
    check_walls(tree, config, problems)
    check_budgets(tree, config, problems)
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
