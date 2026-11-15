"""Network check of every http(s) link in README.md and docs/**/*.md. A release step, not part of `cargo test`.

Run it after the release page exists (the docs link to it) and before the release is announced:

    python3 -I tools/check_external_links.py [--root DIR] [--timeout SECONDS] [--workers N] [--strict] [--skip REGEX]...
                                             [--api URL | --no-api]

It reads the same links as tools/docs_check.py (`external_links`: link targets and bare URLs, code blocks and comments
left out), asks each distinct URL once (HEAD, then GET when the server refuses HEAD or answers it with an error), prints
a report, and changes nothing. No fix is applied: a moved link is for a person to update.

Report lines, one per URL, then a summary:

  OK     200 <url>
  MOVED  200 <url> -> <final url>     (a redirect; a warning, a failure only with --strict)
  FAIL   404 <url>                     (4xx or 5xx after the GET, or no answer within the timeout)
  STALE  CURRENT-RELEASE.md says rc.14 but the forge has a release rc.15      (see below)

Each line is followed by where the URL is used (`path:line`, the first three).

The STALE check asks the forge's release list (`--api`, default: this repository's Forgejo API) and fails when it holds a
release `v0.2.0-rc.N` with N above the release named by docs/CURRENT-RELEASE.md: the page was not updated for it. It
needs no login. `--no-api` skips it. The release the page names is not yet on the forge before its release is published,
so that link fails until then; that is the order the checklist gives (docs/BUILD.md, "Release checklist").

Exit: 0 every link answered (and nothing stale); 1 a failure, a stale release page, or a moved link with --strict;
2 it could not run (no docs, an unreadable file, a bad option).
"""
import argparse
import concurrent.futures
import json
import os
import pathlib
import re
import sys
import urllib.error
import urllib.request

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))  # python3 -I leaves the script's folder out
import docs_check  # noqa: E402

DEFAULT_API = "https://git.syntaxbyexample.com/api/v1/repos/intpa/TrainingRoomsForUltimate/releases?limit=50"
USER_AGENT = "TrainingRooms-docs-link-check"
RELEASE_TAG = re.compile(r"^v\d+\.\d+\.\d+-rc\.(\d+)$")


def distinct(links):
    """[(url, [(path, line), ...])] in order of first use; the fragment is not asked for, so `#a` and `#b` are one URL."""
    found = {}
    for url, path, line in links:
        found.setdefault(url.split("#")[0], []).append((path, line))
    return list(found.items())


def ask(url, method, timeout):
    request = urllib.request.Request(url, method=method, headers={"User-Agent": USER_AGENT, "Accept": "*/*"})
    with urllib.request.urlopen(request, timeout=timeout) as answer:
        if method == "GET":
            answer.read(1024)  # enough to know the body arrives; the page itself is not needed
        return answer.status, answer.geturl()


def check_url(url, timeout):
    """(kind, status, final url or error): kind is OK, MOVED or FAIL."""
    last = None
    for method in ("HEAD", "GET"):
        try:
            status, final = ask(url, method, timeout)
        except urllib.error.HTTPError as error:
            last = (error.code, f"HTTP {error.code}")
            error.close()
            continue
        except (urllib.error.URLError, OSError, ValueError) as error:
            last = (0, str(getattr(error, "reason", error)))
            continue
        moved = final.rstrip("/") != url.rstrip("/")
        return ("MOVED" if moved else "OK"), status, final
    return "FAIL", last[0], last[1]


def newer_releases(tags, current):
    """The release numbers on the forge above the current one (`rc.14` -> 14), sorted."""
    number = int(current.split(".")[1])
    return sorted({int(m.group(1)) for t in tags if (m := RELEASE_TAG.match(t)) and int(m.group(1)) > number})


def forge_tags(api, timeout):
    request = urllib.request.Request(api, headers={"User-Agent": USER_AGENT, "Accept": "application/json"})
    with urllib.request.urlopen(request, timeout=timeout) as answer:
        return [item["tag_name"] for item in json.load(answer) if not item.get("draft")]


def run(root, timeout=15, workers=8, strict=False, skip=(), api=DEFAULT_API, out=sys.stdout):
    """Check the tree under `root`; the report goes to `out`; the return value is the exit code."""
    skip = [re.compile(p) for p in skip]
    links = [(u, p, n) for u, p, n in docs_check.external_links(root) if not any(s.search(u) for s in skip)]
    urls = distinct(links)
    problems = []
    tree = docs_check.Tree(root)
    release = docs_check.current_release(tree, problems)
    if release is None:
        print("no `Current release: rc.N` line in docs/CURRENT-RELEASE.md", file=out)
        return 2
    with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, workers)) as pool:
        results = list(pool.map(lambda item: check_url(item[0], timeout), urls))
    counts = {"OK": 0, "MOVED": 0, "FAIL": 0}
    for (url, uses), (kind, status, detail) in zip(urls, results):
        counts[kind] += 1
        arrow = f" -> {detail}" if kind == "MOVED" else (f"  ({detail})" if kind == "FAIL" else "")
        print(f"{kind:<6} {status:>3} {url}{arrow}", file=out)
        for path, line in uses[:3]:
            print(f"         {path}:{line}", file=out)
        if len(uses) > 3:
            print(f"         and {len(uses) - 3} more", file=out)
    stale = []
    if api:
        try:
            stale = newer_releases(forge_tags(api, timeout), release)
        except (urllib.error.URLError, OSError, ValueError, KeyError) as error:
            if isinstance(error, urllib.error.HTTPError):
                error.close()
            counts["FAIL"] += 1
            print(f"FAIL     0 {api}  (the release list could not be read: {getattr(error, 'reason', error)})", file=out)
    for number in stale:
        print(f"STALE  docs/CURRENT-RELEASE.md says {release} but the forge has a release rc.{number}", file=out)
    print(f"check_external_links: {len(urls)} URLs: {counts['OK']} ok, {counts['MOVED']} moved, {counts['FAIL']} failed, "
          f"{len(stale)} stale (current release {release})", file=out)
    failed = counts["FAIL"] or stale or (strict and counts["MOVED"])
    return 1 if failed else 0


def main(argv=None):
    parser = argparse.ArgumentParser(description="Network check of the external links in README.md and docs/")
    parser.add_argument("--root", default=str(pathlib.Path(__file__).resolve().parent.parent))
    parser.add_argument("--timeout", type=float, default=15.0, help="seconds per request (default 15)")
    parser.add_argument("--workers", type=int, default=8)
    parser.add_argument("--strict", action="store_true", help="a moved link (a redirect) fails too")
    parser.add_argument("--skip", action="append", default=[], metavar="REGEX", help="do not ask URLs matching this")
    parser.add_argument("--api", default=DEFAULT_API, help="the forge's release list (JSON)")
    parser.add_argument("--no-api", action="store_true", help="do not look for a newer release")
    args = parser.parse_args(argv)
    if not os.path.isdir(os.path.join(args.root, "docs")):
        print(f"no docs/ folder under {args.root}", file=sys.stderr)
        return 2
    try:
        return run(args.root, args.timeout, args.workers, args.strict, args.skip, None if args.no_api else args.api)
    except (OSError, UnicodeDecodeError, re.error) as error:
        print(f"could not run: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
