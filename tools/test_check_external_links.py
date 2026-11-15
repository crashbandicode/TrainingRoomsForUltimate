"""Host tests of tools/check_external_links.py against a server on 127.0.0.1 (no outside network).

Run by `tests/docs_gates.rs` (`python3 -I tools/test_check_external_links.py`). The local server answers a fixed set of
paths: 200, 404, a redirect, HEAD refused with 405 (GET fine), HEAD with an error but GET fine, and a slow one.
"""
import contextlib
import http.server
import io
import json
import pathlib
import sys
import threading
import time
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))  # python3 -I leaves the script's folder out
import check_external_links as c  # noqa: E402
import test_docs_check as fixtures  # noqa: E402


class Handler(http.server.BaseHTTPRequestHandler):
    hits = []

    def log_message(self, *args):
        pass

    def respond(self, code, body=b"ok", headers=()):
        self.send_response(code)
        for name, value in headers:
            self.send_header(name, value)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def handle_any(self):
        try:
            self.answer()
        except (BrokenPipeError, ConnectionResetError):
            pass  # the client gave up (the timeout test)

    def answer(self):
        Handler.hits.append((self.command, self.path))
        path = self.path.split("#")[0]
        if path == "/ok":
            self.respond(200)
        elif path == "/gone":
            self.respond(404)
        elif path == "/moved":
            self.respond(301, headers=[("Location", "/ok")])
        elif path == "/no-head":
            self.respond(405 if self.command == "HEAD" else 200)
        elif path == "/head-error":
            self.respond(500 if self.command == "HEAD" else 200)
        elif path == "/slow":
            time.sleep(1)
            self.respond(200)
        elif path == "/api":
            self.respond(200, json.dumps(Handler.api).encode(), [("Content-Type", "application/json")])
        else:
            self.respond(404)

    do_GET = do_HEAD = handle_any
    api = []


class Server:
    def __enter__(self):
        Handler.hits = []
        self.httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.base = f"http://127.0.0.1:{self.httpd.server_address[1]}"
        threading.Thread(target=self.httpd.serve_forever, kwargs={"poll_interval": 0.02}, daemon=True).start()
        return self

    def __exit__(self, *exc):
        self.httpd.shutdown()
        self.httpd.server_close()


def tree(base, *paths):
    links = "\n".join(f"[l{i}]({base}{p})" for i, p in enumerate(paths))
    return fixtures.build(dict(fixtures.CLEAN, **{"README.md": fixtures.CLEAN["README.md"] + "\n" + links + "\n"}))


def report(root, **options):
    out = io.StringIO()
    code = c.run(root, out=out, **options)
    return code, out.getvalue()


class Statuses(unittest.TestCase):
    def test_200_is_ok_and_the_exit_is_0(self):
        with Server() as s:
            code, text = report(tree(s.base, "/ok"), api=None)
        self.assertEqual(code, 0, text)
        self.assertIn("OK", text)
        self.assertIn("1 ok, 0 moved, 0 failed", text)

    def test_404_fails_with_the_page_that_uses_it(self):
        with Server() as s:
            code, text = report(tree(s.base, "/ok", "/gone"), api=None)
        self.assertEqual(code, 1, text)
        self.assertIn(f"FAIL   404 {s.base}/gone", text)
        self.assertIn("README.md:", text)
        self.assertIn("1 ok, 0 moved, 1 failed", text)

    def test_a_redirect_is_a_warning_and_fails_only_with_strict(self):
        with Server() as s:
            root = tree(s.base, "/moved")
            code, text = report(root, api=None)
            strict, _ = report(root, api=None, strict=True)
        self.assertEqual(code, 0, text)
        self.assertIn(f"MOVED", text)
        self.assertIn(f"-> {s.base}/ok", text)
        self.assertEqual(strict, 1)

    def test_a_server_that_refuses_head_is_asked_again_with_get(self):
        with Server() as s:
            code, text = report(tree(s.base, "/no-head", "/head-error"), api=None)
        self.assertEqual(code, 0, text)
        self.assertEqual(sorted(m for m, p in Handler.hits if p == "/no-head"), ["GET", "HEAD"])

    def test_a_link_that_is_not_answered_in_time_fails(self):
        with Server() as s:
            code, text = report(tree(s.base, "/slow"), api=None, timeout=0.3)
        self.assertEqual(code, 1, text)
        self.assertIn("FAIL", text)

    def test_a_refused_connection_fails(self):
        code, text = report(tree("http://127.0.0.1:9", "/x"), api=None, timeout=2)
        self.assertEqual(code, 1, text)

    def test_one_url_is_asked_once_however_often_it_is_used(self):
        with Server() as s:
            code, text = report(tree(s.base, "/ok", "/ok#a", "/ok#b"), api=None)
        self.assertEqual(code, 0, text)
        self.assertEqual(len([h for h in Handler.hits if h[0] == "HEAD" and h[1] == "/ok"]), 1)
        self.assertIn("1 URLs", text)

    def test_skip_leaves_a_url_alone(self):
        with Server() as s:
            code, text = report(tree(s.base, "/gone"), api=None, skip=["/gone$"])
        self.assertEqual(code, 0, text)
        self.assertEqual(Handler.hits, [])

    def test_nothing_is_changed(self):
        with Server() as s:
            root = tree(s.base, "/gone")
            before = (pathlib.Path(root) / "README.md").read_text()
            report(root, api=None)
        self.assertEqual((pathlib.Path(root) / "README.md").read_text(), before)


class StaleRelease(unittest.TestCase):
    def test_a_newer_release_on_the_forge_makes_the_page_stale(self):
        Handler.api = [{"tag_name": "v0.2.0-rc.15", "draft": False}, {"tag_name": "v0.2.0-rc.14", "draft": False}]
        with Server() as s:
            code, text = report(tree(s.base, "/ok"), api=f"{s.base}/api")
        self.assertEqual(code, 1, text)
        self.assertIn("STALE  docs/CURRENT-RELEASE.md says rc.14 but the forge has a release rc.15", text)

    def test_the_current_release_and_older_ones_are_not_stale(self):
        Handler.api = [{"tag_name": "v0.2.0-rc.14", "draft": False}, {"tag_name": "v0.2.0-rc.13", "draft": False}, {"tag_name": "v0.2.0-rc.3.1", "draft": False}]
        with Server() as s:
            code, text = report(tree(s.base, "/ok"), api=f"{s.base}/api")
        self.assertEqual(code, 0, text)

    def test_a_draft_is_not_a_release(self):
        Handler.api = [{"tag_name": "v0.2.0-rc.15", "draft": True}]
        with Server() as s:
            code, text = report(tree(s.base, "/ok"), api=f"{s.base}/api")
        self.assertEqual(code, 0, text)

    def test_an_unreadable_release_list_fails(self):
        with Server() as s:
            code, text = report(tree(s.base, "/ok"), api=f"{s.base}/nothing-here")
        self.assertEqual(code, 1, text)
        self.assertIn("the release list could not be read", text)

    def test_newer_releases_reads_tags_only_of_the_rc_pattern(self):
        self.assertEqual(c.newer_releases(["v0.2.0-rc.15", "v0.2.0-rc.16", "v0.2.0-rc.14", "v1.0.0", "v0.2.0-rc.3.1", "nightly"], "rc.14"), [15, 16])


class Options(unittest.TestCase):
    def test_a_missing_current_release_line_is_a_usage_error(self):
        root = fixtures.build({"README.md": "# x\n", "docs/README.md": "# d\n"})
        code, text = report(root, api=None)
        self.assertEqual(code, 2, text)

    def test_main_exits_2_without_a_docs_folder(self):
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(c.main(["--root", fixtures.tempfile.mkdtemp()]), 2)


if __name__ == "__main__":
    unittest.main()
