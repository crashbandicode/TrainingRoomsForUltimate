"""Host tests of tools/docs_check.py: the clean fixture passes, and each rule fails on its own seeded break.

Run by `tests/docs_gates.rs` (`python3 -I tools/test_docs_check.py`). Every test builds a small docs tree in a temporary
folder, so nothing depends on the real docs.
"""
import contextlib
import io
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))  # python3 -I leaves the script's folder out
import docs_check as d  # noqa: E402

# Heading -> the id the forge printed for it on 2026-11-15 (without its `user-content-` prefix), read from the rendered
# pages of this repository on its main branch. Each pair is one fact about the forge, not about this code.
FORGE_SLUGS = [
    ("Display & Controls", "display-controls"),
    ("L+R+A Reset", "l-r-a-reset"),
    ('"Same as …, nothing new saved"', "same-as-nothing-new-saved"),
    ("A new Mii: Add New…", "a-new-mii-add-new"),
    ("0.2.0-rc.6", "0-2-0-rc-6"),
    ("No. of CPUs: the seats are the next fighters", "no-of-cpus-the-seats-are-the-next-fighters"),
    ('Mii editor: "Same as ‹name›, nothing new saved"', "mii-editor-same-as-name-nothing-new-saved"),
    ("Corrections to the rc.5 notes", "corrections-to-the-rc-5-notes"),
    ("rc.9 (draft)", "rc-9-draft"),
    ("The D-pad chords and Online Deluxe's window (C4)", "the-d-pad-chords-and-online-deluxe-s-window-c4"),
    ("Online Deluxe's L + R + D-pad Left / Right (C10)", "online-deluxe-s-l-r-d-pad-left-right-c10"),
    ("imgui_smash 1.0.2", "imgui_smash-1-0-2"),
    ("Page 4 — Fighters & Stages", "page-4-fighters-stages"),
    ("Q1 · Number of pages", "q1-number-of-pages"),
    ("Q17 · Which chips show the game's DI / SDI visual", "q17-which-chips-show-the-game-s-di-sdi-visual"),
    ("The frame strip and the waiting room's menu", "the-frame-strip-and-the-waiting-room-s-menu"),
    ("Eggs-ercise", "eggs-ercise"),
    ("Solo Popo (original Ice Climbers)", "solo-popo-original-ice-climbers"),
    ("Save setup to library...", "save-setup-to-library"),
    ("2026-11-03: RC archive working-buffer residency", "2026-11-03-rc-archive-working-buffer-residency"),
]

CURRENT = "# Current release\n\nCurrent release: rc.14\n"

CLEAN = {
    "Cargo.toml": '[package]\nname = "training_rooms"\nversion = "0.2.0-rc.14"\n',
    "Cargo.lock": '# generated\n\n[[package]]\nname = "other"\nversion = "1.0.0"\n\n[[package]]\nname = "training_rooms"\nversion = "0.2.0-rc.14"\ndependencies = []\n',
    "README.md": (
        "# Project\n\nIntro. **Current release: [rc.14](docs/CURRENT-RELEASE.md)**, untested.\n\n## Install\n\n"
        "See [the docs](docs/README.md) and ![a picture](docs/media/pic.gif).\n"
    ),
    "docs/README.md": (
        "# Docs\n\n- [Current release](CURRENT-RELEASE.md)\n- [Getting started](guide/getting-started.md)\n"
        "- [Technical](TECHNICAL.md)\n- [Old notes](RELEASE-NOTES-rc9.md)\n"
    ),
    "docs/CURRENT-RELEASE.md": CURRENT,
    "docs/guide/getting-started.md": (
        "# Getting started\n\nThe footer reads `0.2.0-rc.14 · rc14 1a2b3c4d`: that is rc.14.\n\n"
        "## Which build\n\nSee [Technical](../TECHNICAL.md#status) and [this page](#which-build).\n"
    ),
    "docs/TECHNICAL.md": "# Technical\n\n## Status\n\nThe current release is rc.14, as the footer shows.\n\n## Other\n\nText.\n",
    "docs/RELEASE-NOTES-rc9.md": "# rc.9\n\nThe newest build is rc.9.\n",
    "docs/media/pic.gif": "GIF",
}

def cfg(**changes):
    """A config with no allowance and no notice pages: the real lists name pages the small fixtures do not have.
    No word budget either: the fixtures of the other rules add a sentence to a guide page (`Budgets` turns it on)."""
    base = dict(historical=[("docs/RELEASE-NOTES-rc*.md", "a past release")], orphan_ok=[], claim_ok=[], must_name=d.MUST_NAME,
                notice_ok=[], notice_pages=[], notice_columns={}, wall_pending={}, budget_words=[], budget_pending={},
                budget_visuals={})
    return d.Config(**dict(base, **changes))


NO_ALLOWANCE = cfg()


def build(files):
    """A temporary tree holding `files` ({relative path: text}); returns its path (the caller removes it)."""
    root = tempfile.mkdtemp(prefix="docs-check-")
    for rel, text in files.items():
        path = os.path.join(root, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="") as handle:
            handle.write(text)
    return root


def problems(files, config=NO_ALLOWANCE, **changes):
    """The rule names of every problem in CLEAN with `changes` applied ({path: text}; None deletes the file)."""
    tree = dict(CLEAN, **files, **changes)
    tree = {k: v for k, v in tree.items() if v is not None}
    root = build(tree)
    try:
        return [(p.rule, p.path, p.text) for p in d.check(root, config)]
    finally:
        for folder, _, names in os.walk(root, topdown=False):
            for name in names:
                os.remove(os.path.join(folder, name))
            os.rmdir(folder)


def rules(**changes):
    return sorted({rule for rule, _, _ in problems({}, **changes)})


class Slugs(unittest.TestCase):
    def test_every_heading_has_the_id_the_forge_printed(self):
        for heading, expected in FORGE_SLUGS:
            self.assertEqual(d.slug(heading), expected, heading)

    def test_a_repeated_heading_gets_a_number_like_the_forge(self):
        # menu/cpu.md on the forge: Shield, Shield (shield-1), Shield Tilt; saves.md: Save states, Save States (save-states-1)
        text = "# A\n\n## Shield\n\n### Shield\n\n### Shield Tilt\n\n## Save states\n\n### Save States\n\n### Shield\n"
        self.assertEqual(d.anchors_of(text), {"a", "shield", "shield-1", "shield-2", "shield-tilt", "save-states", "save-states-1"})

    def test_markup_in_a_heading_is_not_part_of_its_id(self):
        self.assertEqual(d.anchors_of("# Tools: `code`, *a* [link](x.md) and ![i](p.png)\n"), {"tools-code-a-link-and-i"})


class CleanTree(unittest.TestCase):
    def test_the_clean_fixture_has_no_problem(self):
        self.assertEqual(problems({}), [])


class Links(unittest.TestCase):
    def test_a_missing_file_fails(self):
        self.assertEqual(rules(**{"docs/guide/getting-started.md": CLEAN["docs/guide/getting-started.md"] + "\n[x](nowhere.md)\n"}), ["link-missing"])

    def test_a_missing_image_fails(self):
        self.assertEqual(rules(**{"docs/TECHNICAL.md": CLEAN["docs/TECHNICAL.md"] + "\n![x](media/none.gif)\n"}), ["link-missing"])

    def test_the_case_of_a_path_must_match(self):
        self.assertEqual(rules(**{"README.md": CLEAN["README.md"] + "\n[x](docs/Technical.md)\n"}), ["link-missing"])

    def test_a_path_on_a_machine_fails(self):
        self.assertEqual(rules(**{"README.md": CLEAN["README.md"] + "\n[x](/home/someone/notes.md)\n"}), ["link-missing"])

    def test_a_path_that_leaves_the_repository_fails(self):
        self.assertEqual(rules(**{"README.md": CLEAN["README.md"] + "\n[x](../outside.md)\n"}), ["link-missing"])

    def test_an_html_image_is_checked_too(self):
        self.assertEqual(rules(**{"README.md": CLEAN["README.md"] + '\n<img src="docs/media/none.gif">\n'}), ["link-missing"])

    def test_a_link_in_a_code_block_a_code_span_or_a_comment_is_not_read(self):
        text = "\n```md\n[x](gone.md)\n```\n\n`[y](gone.md)`\n\n<!-- [z](gone.md) -->\n"
        self.assertEqual(problems({}, **{"README.md": CLEAN["README.md"] + text}), [])

    def test_an_external_link_is_not_checked_here(self):
        self.assertEqual(problems({}, **{"README.md": CLEAN["README.md"] + "\n[x](https://example.invalid/a.md)\n"}), [])

    def test_a_percent_encoded_path_and_a_target_with_a_title_resolve(self):
        files = {"docs/a b.md": "# A\n", "README.md": CLEAN["README.md"] + '\n[x](docs/a%20b.md "title") [y](<docs/a b.md>)\n'}
        self.assertEqual([r for r, _, _ in problems(files) if r.startswith("link")], [])

    def test_a_target_with_parentheses_resolves(self):
        files = {"docs/f (1).md": "# F\n", "README.md": CLEAN["README.md"] + "\n[x](<docs/f (1).md>) [y](docs/f_(1).md)\n"}
        self.assertEqual([r for r, _, _ in problems(files) if r.startswith("link")], ["link-missing"])

    def test_a_link_that_is_no_link_fails(self):
        self.assertEqual(rules(**{"README.md": CLEAN["README.md"] + "\n[x](two words.md)\n"}), ["link-syntax"])

    def test_an_undefined_reference_fails_and_a_defined_one_passes(self):
        self.assertEqual(rules(**{"README.md": CLEAN["README.md"] + "\n[x][nope]\n"}), ["link-ref"])
        ok = CLEAN["README.md"] + "\n[x][a]\n\n[a]: docs/README.md\n"
        self.assertEqual(problems({}, **{"README.md": ok}), [])

    def test_a_reference_definition_with_a_missing_target_fails(self):
        self.assertEqual(rules(**{"README.md": CLEAN["README.md"] + "\n[a]: gone.md\n"}), ["link-missing"])


class VideoSources(unittest.TestCase):
    blob = "7634061256bdced74a25e63f3a15600361cec376"
    url = "https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/"

    def check_video(self, text):
        files = {"docs/media/clip.mp4": "fixture mp4", "README.md": CLEAN["README.md"] + "\n" + text + "\n"}
        return problems(files)

    def test_a_relative_src_fails(self):
        got = self.check_video('<video src="docs/media/clip.mp4" controls></video>')
        self.assertEqual([r for r, _, _ in got], ["video-src"])
        self.assertIn("use the media blob URL of the clip", got[0][2])

    def test_an_unknown_blob_fails_and_names_the_id(self):
        unknown = "a" * 40
        got = self.check_video(f'<video src="{self.url}{unknown}"></video>')
        self.assertEqual([r for r, _, _ in got], ["video-src"])
        self.assertIn(unknown, got[0][2])

    def test_a_blob_of_a_fixture_mp4_passes(self):
        self.assertEqual(self.check_video(f'<video src="{self.url}{self.blob}" controls></video>'), [])

    def test_code_and_comments_are_not_read(self):
        text = '```html\n<video src="missing.mp4"></video>\n```\n<!-- <video src="missing.mp4"></video> -->'
        self.assertEqual(self.check_video(text), [])

    def test_missing_empty_wrong_and_duplicate_sources_fail(self):
        for tag in ('<video controls></video>', '<video src=""></video>',
                    '<video src="https://example.invalid/clip.mp4"></video>',
                    f'<video src="{self.url}{self.blob}?download=1"></video>',
                    f'<video src="{self.url}{self.blob}" src="{self.url}{self.blob}"></video>'):
            with self.subTest(tag=tag):
                self.assertEqual([r for r, _, _ in self.check_video(tag)], ["video-src"])

    def test_single_quoted_unquoted_and_multiline_sources(self):
        for tag in (f"<video src='{self.url}{self.blob}'></video>",
                    f"<video\n controls src={self.url}{self.blob}\n></video>"):
            with self.subTest(tag=tag):
                self.assertEqual(self.check_video(tag), [])

    def test_src_inside_another_attribute_is_not_a_source(self):
        tag = f"""<video title='src="{self.url}{self.blob}"'></video>"""
        self.assertEqual([r for r, _, _ in self.check_video(tag)], ["video-src"])

    def test_inline_code_is_not_read_and_line_numbers_are_kept(self):
        text = '`<video src="missing.mp4">`\n\n<video controls></video>'
        root = build(dict(CLEAN, **{"README.md": CLEAN["README.md"] + "\n" + text + "\n"}))
        got = d.check(root, NO_ALLOWANCE)
        self.assertEqual([(p.rule, p.line) for p in got], [("video-src", 11)])


class Anchors(unittest.TestCase):
    def test_a_missing_anchor_in_another_page_fails(self):
        self.assertEqual(rules(**{"docs/guide/getting-started.md": CLEAN["docs/guide/getting-started.md"] + "\n[x](../TECHNICAL.md#nope)\n"}), ["link-anchor"])

    def test_a_missing_anchor_in_the_same_page_fails(self):
        self.assertEqual(rules(**{"docs/guide/getting-started.md": CLEAN["docs/guide/getting-started.md"] + "\n[x](#nope)\n"}), ["link-anchor"])

    def test_github_style_slugs_fail_because_the_forge_differs(self):
        page = "# Controls\n\n### L+R+A Reset\n\n## Display & Controls\n"
        base = {"docs/menu/c.md": page, "docs/README.md": CLEAN["docs/README.md"] + "- [c](menu/c.md)\n"}
        github = dict(base, **{"README.md": CLEAN["README.md"] + "\n[a](docs/menu/c.md#lra-reset) [b](docs/menu/c.md#display--controls)\n"})
        self.assertEqual([r for r, _, _ in problems(github)], ["link-anchor", "link-anchor"])
        forge = dict(base, **{"README.md": CLEAN["README.md"] + "\n[a](docs/menu/c.md#l-r-a-reset) [b](docs/menu/c.md#display-controls)\n"})
        self.assertEqual(problems(forge), [])

    def test_an_anchor_inside_a_code_block_is_not_a_heading(self):
        page = "# Page\n\n```md\n## Hidden\n```\n"
        files = {"docs/p.md": page, "docs/README.md": CLEAN["docs/README.md"] + "- [p](p.md)\n", "README.md": CLEAN["README.md"] + "\n[x](docs/p.md#hidden)\n"}
        self.assertEqual([r for r, _, _ in problems(files)], ["link-anchor"])

    def test_a_named_anchor_and_a_setext_heading_count(self):
        page = '# Page\n\nUnder line\n----------\n\n<a id="here"></a>\n'
        files = {"docs/p.md": page, "docs/README.md": CLEAN["docs/README.md"] + "- [p](p.md)\n", "README.md": CLEAN["README.md"] + "\n[x](docs/p.md#under-line) [y](docs/p.md#here)\n"}
        self.assertEqual(problems(files), [])

    def test_an_anchor_on_a_file_that_is_not_markdown_is_not_checked(self):
        files = {"README.md": CLEAN["README.md"] + "\n[x](Cargo.toml#L3)\n"}
        self.assertEqual(problems({}, **files), [])


class Orphans(unittest.TestCase):
    def test_a_page_nothing_links_to_fails(self):
        self.assertEqual(rules(**{"docs/guide/lonely.md": "# Lonely\n"}), ["orphan"])

    def test_a_page_reached_through_another_page_passes(self):
        files = {"docs/guide/a.md": "# A\n\n[b](b.md)\n", "docs/guide/b.md": "# B\n", "docs/README.md": CLEAN["docs/README.md"] + "- [a](guide/a.md)\n"}
        self.assertEqual(problems(files), [])

    def test_a_link_to_a_folder_does_not_reach_its_pages(self):
        files = {"docs/guide/b.md": "# B\n", "docs/README.md": CLEAN["docs/README.md"] + "- [all](guide/)\n"}
        self.assertEqual(rules(**files), ["orphan"])

    def test_a_listed_orphan_passes_and_a_stale_entry_fails(self):
        listed = cfg(orphan_ok=[("docs/old/*", "records")])
        self.assertEqual(problems({"docs/old/x.md": "# X\n"}, config=listed), [])
        self.assertEqual([r for r, _, _ in problems({}, config=listed)], ["allowlist-stale"])


class Versions(unittest.TestCase):
    def test_the_current_page_needs_one_machine_line(self):
        self.assertEqual(rules(**{d.CURRENT_PAGE: "# Current release\n\nrc.14 is current.\n"}), ["version-line"])
        self.assertEqual(rules(**{d.CURRENT_PAGE: CURRENT + "\nCurrent release: rc.15\n"}), ["version-line"])
        self.assertIn("version-line", rules(**{d.CURRENT_PAGE: None}))

    def test_a_line_inside_a_code_block_is_not_the_machine_line(self):
        self.assertEqual(rules(**{d.CURRENT_PAGE: "# C\n\n```\nCurrent release: rc.14\n```\n"}), ["version-line"])

    def test_the_readme_top_must_name_the_release_and_link_the_page(self):
        top = "# Project\n\nIntro only.\n\n## Install\n\nSee [docs](docs/README.md) rc.14 ![a](docs/media/pic.gif)\n"
        self.assertEqual(rules(**{"README.md": top}), ["version-name"])
        named = "# Project\n\nIntro: rc.14, no link.\n\n## Install\n\nSee [docs](docs/README.md).\n"
        got = problems({}, **{"README.md": named})
        self.assertEqual([r for r, _, _ in got], ["version-name"])
        self.assertIn("does not link", got[0][2])

    def test_a_later_release_in_the_readme_top_is_not_the_current_one(self):
        top = CLEAN["README.md"].replace("rc.14", "rc.13")
        self.assertEqual(rules(**{"README.md": top}), ["version-name", "version-stale"])
        quiet = top.replace("**Current release: [rc.13]", "**[rc.13]")
        self.assertEqual(rules(**{"README.md": quiet}), ["version-name"])

    def test_the_status_section_must_name_the_release(self):
        self.assertEqual(rules(**{"docs/TECHNICAL.md": "# Technical\n\n## Status\n\nNothing here.\n\n## Other\n\nrc.14 in another section.\n"}), ["version-name"])
        # no Status section at all: the link to #status breaks too
        self.assertEqual(rules(**{"docs/TECHNICAL.md": "# Technical\n\nrc.14\n"}), ["link-anchor", "version-name"])

    def test_a_page_that_calls_an_older_release_the_newest_fails(self):
        for sentence in ("The newest build is rc.13.", "The latest release is **rc.13**.", "This release, rc.13, adds a row.",
                         "The current build is [rc.13](https://example.invalid/v0.2.0-rc.13).", "It is currently rc13."):
            with self.subTest(sentence):
                text = CLEAN["docs/guide/getting-started.md"] + f"\n{sentence}\n"
                self.assertEqual(rules(**{"docs/guide/getting-started.md": text}), ["version-stale"])

    def test_the_stale_claim_in_a_table_row_or_a_list_item_fails(self):
        for text in ("\n| Build | State |\n| --- | --- |\n| Newest | rc.12 |\n", "\n- The latest is rc.12.\n- Fine.\n"):
            with self.subTest(text):
                self.assertEqual(rules(**{"docs/TECHNICAL.md": CLEAN["docs/TECHNICAL.md"] + text}), ["version-stale"])

    def test_the_current_release_in_a_claim_passes(self):
        text = CLEAN["docs/TECHNICAL.md"] + "\nThe newest build is rc.14, and the latest notes are [rc.14](CURRENT-RELEASE.md).\n"
        self.assertEqual(problems({}, **{"docs/TECHNICAL.md": text}), [])

    def test_the_crate_version_must_name_the_current_release(self):
        """The footer starts with the crate version and the tag is v0.2.0-rc.N: Cargo.toml and Cargo.lock follow CURRENT-RELEASE."""
        behind = '[package]\nname = "training_rooms"\nversion = "0.2.0-rc.13"\n'
        got = problems({}, **{"Cargo.toml": behind, "Cargo.lock": CLEAN["Cargo.lock"].replace("rc.14", "rc.13")})
        self.assertEqual([(r, p) for r, p, _ in got], [("version-cargo", "Cargo.toml")])
        self.assertIn("0.2.0-rc.13", got[0][2])
        self.assertIn("rc.14", got[0][2])
        ahead = CLEAN["Cargo.toml"].replace("rc.14", "rc.15")
        self.assertEqual(rules(**{"Cargo.toml": ahead, "Cargo.lock": CLEAN["Cargo.lock"].replace("rc.14", "rc.15")}), ["version-cargo"])

    def test_the_lock_file_must_agree_with_cargo_toml(self):
        got = problems({}, **{"Cargo.lock": CLEAN["Cargo.lock"].replace("training_rooms\"\nversion = \"0.2.0-rc.14", "training_rooms\"\nversion = \"0.2.0-rc.6")})
        self.assertEqual([(r, p) for r, p, _ in got], [("version-cargo", "Cargo.lock")])
        no_lock = {"Cargo.lock": None}
        self.assertEqual(problems({}, **no_lock), [])

    def test_the_crate_version_may_be_named_in_a_claim_sentence_only_as_the_current_release(self):
        text = CLEAN["docs/TECHNICAL.md"] + "\nThe version is currently 0.2.0-rc.14 in every build.\n"
        self.assertEqual(problems({}, **{"docs/TECHNICAL.md": text}), [])
        old = CLEAN["docs/TECHNICAL.md"] + "\nThe version is currently 0.2.0-rc.6 in every build.\n"
        self.assertEqual(rules(**{"docs/TECHNICAL.md": old}), ["version-stale"])

    def test_a_caption_that_names_the_build_of_a_clip_is_not_a_claim(self):
        text = CLEAN["docs/guide/getting-started.md"] + "\n*Eden, rc.13 (untested on hardware).*\n"
        self.assertEqual(problems({}, **{"docs/guide/getting-started.md": text}), [])

    def test_wording_without_a_release_is_not_a_claim(self):
        text = CLEAN["docs/TECHNICAL.md"] + "\nThe newest fighter in the list is Steve. The current page is the first.\n"
        self.assertEqual(problems({}, **{"docs/TECHNICAL.md": text}), [])

    def test_a_listed_historical_page_may_name_its_own_release(self):
        self.assertEqual(problems({}), [])  # RELEASE-NOTES-rc9.md says "The newest build is rc.9." and is listed
        unlisted = cfg(historical=[])
        got = problems({}, config=unlisted)
        self.assertIn("version-stale", [r for r, _, _ in got])

    def test_a_folder_of_player_pages_is_not_excluded_by_the_default_list(self):
        for pattern, _ in d.HISTORICAL:
            self.assertFalse(pattern.startswith(("docs/guide", "docs/menu/", "README")), pattern)
        self.assertFalse(any(p in ("docs/*", "docs/**") for p, _ in d.HISTORICAL))

    def test_an_allowed_claim_passes_and_a_stale_allowance_fails(self):
        text = CLEAN["docs/TECHNICAL.md"] + "\nThe newest notes written in this folder are rc.9's.\n"
        self.assertEqual(rules(**{"docs/TECHNICAL.md": text}), ["version-stale"])
        allowed = cfg(claim_ok=[("docs/TECHNICAL.md", "newest notes written in this folder", "about notes, not the build")])
        self.assertEqual(problems({}, config=allowed, **{"docs/TECHNICAL.md": text}), [])
        self.assertEqual([r for r, _, _ in problems({}, config=allowed)], ["allowlist-stale"])


SRC = '''
// "Gone in a comment"
/* "Gone in a block /* "nested" */ comment" */
pub const A: &str = "Wait for the swap to finish";
pub const B: &str = r#"A raw "quoted" text"#;
pub fn f(save: bool, name: &str) {
    let _quote = '"';
    let _lifetime: &'static str = "Real text after a char literal";
    let _ = format!("Loading {}/{}", 1, 2);
    let _ = if save { "A setup is placed: save again after it" } else { "A setup is placed: load again after it" };
    let _ = ["Same as ", name, ", nothing new saved"];
    let _ = ("Practice Stage", "Stage requested");
    let _ = "Escaped \\"quote\\" and a\\
              continued line";
}
'''
NOTICE_PAGE = "docs/guide/troubleshooting.md"
STRICT = cfg(notice_pages=[NOTICE_PAGE])


def notice_problems(page, text, sources=None, **config):
    """notice-text and allowlist problems of `page` with `text`; `sources` ({path: text}) replaces src/lib.rs's SRC."""
    files = dict(CLEAN, **(sources or {"src/lib.rs": SRC}), **{"docs/README.md": CLEAN["docs/README.md"] + f"- [p]({page[len('docs/'):]})\n", page: text})
    return [(r, p, t) for r, p, t in problems(files, config=config.pop("config", STRICT)) if r in ("notice-text", "allowlist-stale")]


WALL_PAGE = "docs/menu/page.md"
LONG = "word " * 30


def wall_problems(text, page=WALL_PAGE, **config):
    """wall and allowlist problems of `page` with `text` (linked from docs/README.md, so it is no orphan)."""
    files = dict(CLEAN, **{"docs/README.md": CLEAN["docs/README.md"] + f"- [p]({page[len('docs/'):]})\n", page: text})
    return [(r, p, t) for r, p, t in problems(files, config=cfg(**config)) if r in ("wall", "allowlist-stale")]


class Walls(unittest.TestCase):
    def test_a_long_caption_fails_and_a_short_one_passes(self):
        got = wall_problems(f"# T\n\n*{LONG}Eden, 2026-11-27.* [Open the clip](../media/pic.gif)\n")
        self.assertEqual([r for r, _, _ in got], ["wall"])
        self.assertIn("a caption of", got[0][2])
        self.assertEqual(wall_problems("# T\n\n*Mario jabs twice. Eden, 2026-11-27.* [Open the clip](../media/pic.gif)\n"), [])

    def test_the_link_to_the_clip_and_the_alt_text_of_the_image_are_not_counted(self):
        caption = "x" * 118
        self.assertEqual(wall_problems(f"# T\n\n![{LONG}](../media/pic.gif)<br>\n*{caption}.* [Open the clip](../media/pic.gif)\n"), [])

    def test_a_paragraph_of_three_sentences_fails_and_two_pass(self):
        got = wall_problems("# T\n\nOne thing. Another thing. A third thing.\n")
        self.assertEqual([r for r, _, _ in got], ["wall"])
        self.assertIn("3 sentences", got[0][2])
        self.assertEqual(wall_problems("# T\n\nOne thing. Another thing.\n"), [])

    def test_a_list_item_is_read_alone_and_a_long_one_fails(self):
        self.assertEqual(wall_problems("# T\n\n- One. Two.\n- Three. Four.\n"), [])
        self.assertEqual([r for r, _, _ in wall_problems("# T\n\n- One. Two.\n- Three. Four. Five.\n")], ["wall"])

    def test_one_very_long_sentence_fails(self):
        got = wall_problems(f"# T\n\n{'word ' * 70}end.\n")
        self.assertEqual([r for r, _, _ in got], ["wall"])
        self.assertIn("characters", got[0][2])

    def test_a_settings_bullet_fails(self):
        for word in ("Values", "Default", "How"):
            got = wall_problems(f"# T\n\n- **{word}:** On, Off\n")
            self.assertEqual([r for r, _, _ in got], ["wall"], word)
        self.assertEqual(wall_problems("# T\n\n- **Where:** the Saves page\n"), [])

    def test_tables_headings_code_and_html_are_not_read(self):
        text = f"# {LONG}\n\n| A | B |\n| --- | --- |\n| One. Two. Three. Four. | {LONG} |\n\n```\nOne. Two. Three. Four.\n```\n\n<video src=\"x\"></video>\n"
        self.assertEqual([r for r, _, _ in wall_problems(text) if r == "wall"], [])

    def test_only_menu_and_guide_pages_are_read(self):
        self.assertEqual(wall_problems("# T\n\nOne. Two. Three. Four.\n", page="docs/other.md"), [])
        self.assertEqual([r for r, _, _ in wall_problems("# T\n\nOne. Two. Three. Four.\n", page="docs/guide/other.md")], ["wall"])

    def test_a_pending_page_may_keep_its_count_but_not_grow(self):
        two = "# T\n\nOne. Two. Three.\n\nFour. Five. Six.\n"
        self.assertEqual(wall_problems(two, wall_pending={WALL_PAGE: 2}), [])
        got = wall_problems(two, wall_pending={WALL_PAGE: 1})
        self.assertEqual(sorted({r for r, _, _ in got}), ["wall"])
        self.assertEqual(len(got), 3)
        self.assertTrue(any("pending at 1" in t for _, _, t in got))

    def test_a_pending_count_that_is_too_high_or_names_no_page_is_stale(self):
        got = wall_problems("# T\n\nOne. Two. Three.\n", wall_pending={WALL_PAGE: 2})
        self.assertEqual([(r, "lower it to 1" in t) for r, _, t in got], [("allowlist-stale", True)])
        got = wall_problems("# T\n\nOne.\n", wall_pending={WALL_PAGE: 1})
        self.assertEqual([(r, "remove the entry" in t) for r, _, t in got], [("allowlist-stale", True)])
        got = wall_problems("# T\n\nOne.\n", wall_pending={"docs/menu/gone.md": 1})
        self.assertEqual([(r, "matches no page" in t) for r, _, t in got], [("allowlist-stale", True)])


def budget_problems(text, page=WALL_PAGE, **config):
    """budget and allowlist problems of `page` with `text`; walls are left to the wall rule's own tests."""
    files = dict(CLEAN, **{"docs/README.md": CLEAN["docs/README.md"] + f"- [p]({page[len('docs/'):]})\n", page: text})
    found = problems(files, config=cfg(budget_words=d.BUDGET_WORDS, **config))
    return [(r, p, t) for r, p, t in found if r in ("budget", "allowlist-stale") and p in (page, "tools/docs_check.py")]


TABLE = "| Setting | Values | What it does |\n| --- | --- | --- |\n"


class Budgets(unittest.TestCase):
    def test_a_short_page_of_one_sentence_and_a_table_passes(self):
        self.assertEqual(budget_problems(f"# T\n\nOne sentence.\n\n{TABLE}| A | On, Off | Does a thing. |\n"), [])

    def test_a_menu_page_over_1500_words_fails_and_a_guide_page_over_800(self):
        long_table = TABLE + "".join(f"| Row {n} | On | x |\n" for n in range(230))
        got = budget_problems(f"# T\n\n{long_table}")
        self.assertEqual([r for r, _, _ in got], ["budget"])
        self.assertIn("words (at most 1500)", got[0][2])
        middling = TABLE + "".join(f"| Row {n} | On | x |\n" for n in range(120))
        self.assertEqual(budget_problems(f"# T\n\n{middling}"), [])
        got = budget_problems(f"# T\n\n{middling}", page="docs/guide/page.md")
        self.assertIn("words (at most 800)", got[0][2])

    def test_a_table_cell_over_fifteen_words_fails_and_its_link_target_is_not_counted(self):
        got = budget_problems(f"# T\n\n{TABLE}| A | On | {'word ' * 16}|\n")
        self.assertEqual([r for r, _, _ in got], ["budget"])
        self.assertIn("a table cell of 16 words", got[0][2])
        cell = "word " * 13 + "[the guide](../guide/getting-started.md)"
        self.assertEqual(budget_problems(f"# T\n\n{TABLE}| <a id=\"a\"></a>A | On | {cell} |\n"), [])

    def test_a_caption_over_sixty_characters_fails(self):
        got = budget_problems(f"# T\n\n![alt](../media/pic.gif)<br>\n*{'x' * 61}*\n")
        self.assertEqual([r for r, _, _ in got], ["budget"])
        self.assertIn("a caption of 61 characters", got[0][2])
        self.assertEqual(budget_problems(f"# T\n\n![alt](../media/pic.gif)<br>\n*{'x' * 60}* [Open the clip](../media/pic.gif)\n"), [])

    def test_a_section_with_two_sentences_fails_and_each_section_is_counted_alone(self):
        self.assertEqual(budget_problems("# T\n\nOne.\n\n## A\n\nTwo.\n\n## B\n\nThree.\n"), [])
        got = budget_problems("# T\n\nOne.\n\n## A\n\nTwo. And more.\n")
        self.assertEqual([r for r, _, _ in got], ["budget"])
        self.assertIn("2 sentences of prose", got[0][2])
        got = budget_problems("# T\n\n## A\n\n- One.\n- Two.\n")
        self.assertEqual([r for r, _, _ in got], ["budget"])

    def test_one_good_to_know_table_of_five_rows_at_most(self):
        five = "## Good to know\n\n| If | Then |\n| --- | --- |\n" + "| a | b |\n" * 5
        self.assertEqual(budget_problems(f"# T\n\n{five}"), [])
        got = budget_problems(f"# T\n\n{five}| a | b |\n")
        self.assertEqual([("6 rows" in t) for _, _, t in got], [True])
        got = budget_problems(f"# T\n\n{five}\n{five}")
        self.assertEqual([("second" in t) for _, _, t in got], [True])

    def test_a_visual_is_not_counted_as_words(self):
        table = TABLE + "".join(f"| Row {n} | On | x |\n" for n in range(120))
        visual = f"![{'word ' * 400}](../media/pic.gif)<br>\n*Caption words here.*\n\n<video src=\"x y z\"></video>\n\n"
        self.assertEqual(budget_problems(f"# T\n\n{visual}{table}", page="docs/menu/page.md"), [])
        self.assertEqual(d.counted_words("# T\n\n![a b c](x.gif)<br>\n*d e.* [Open the clip](x.gif)\n\nOne two.\n"), 4)

    def test_every_section_of_a_menu_page_opens_with_a_visual(self):
        on = dict(budget_visuals=d.BUDGET_VISUALS)
        good = "# T\n\nOne.\n\n## A\n\n![a](../media/pic.gif)<br>\n*Caption.*\n\nTwo.\n\n## B\n\n<video src=\"x\"></video>\n\n*Caption.* [Open the clip](../media/pic.gif)\n"
        self.assertEqual(budget_problems(good, **on), [])
        got = budget_problems("# T\n\n## A\n\nTwo.\n\n![a](../media/pic.gif)\n", **on)
        self.assertEqual([("does not open with a clip or picture" in t) for _, _, t in got], [True])
        got = budget_problems("# T\n\n## A\n\n![a](../media/pic.gif)\n\n## B\n", **on)
        self.assertEqual(len(got), 1)
        self.assertEqual(budget_problems("# T\n\n## A\n\n![a](../media/pic.gif)\n\n### Sub\n\nTwo.\n", **on), [])

    def test_a_guide_page_has_at_least_one_visual(self):
        on = dict(budget_visuals=d.BUDGET_VISUALS)
        got = budget_problems("# T\n\nOne.\n\n## A\n\nTwo.\n", page="docs/guide/page.md", **on)
        self.assertEqual([("no clip or picture" in t) for _, _, t in got], [True])
        self.assertEqual(budget_problems("# T\n\n![a](../media/pic.gif)\n\n## A\n\nTwo.\n", page="docs/guide/page.md", **on), [])

    def test_only_menu_and_guide_pages_have_a_budget(self):
        self.assertEqual(budget_problems("# T\n\nOne. Two. Three.\n", page="docs/other.md"), [])

    def test_a_pending_page_is_not_held_to_the_budget_but_may_not_grow(self):
        text = "# T\n\nOne. Two. Three.\n"
        words = len(text.split())
        self.assertEqual(budget_problems(text, budget_pending={WALL_PAGE: words}), [])
        got = budget_problems(text, budget_pending={WALL_PAGE: words - 1})
        self.assertEqual([(r, "only falls" in t) for r, _, t in got], [("budget", True)])

    def test_a_pending_entry_that_is_too_high_met_or_names_no_page_is_stale(self):
        text = "# T\n\nOne. Two. Three.\n"
        got = budget_problems(text, budget_pending={WALL_PAGE: 99})
        self.assertEqual([(r, f"lower it to {len(text.split())}" in t) for r, _, t in got], [("allowlist-stale", True)])
        got = budget_problems("# T\n\nOne.\n", budget_pending={WALL_PAGE: 99})
        self.assertEqual([(r, "remove the entry" in t) for r, _, t in got], [("allowlist-stale", True)])
        got = budget_problems("# T\n\nOne.\n", budget_pending={"docs/menu/gone.md": 5})
        self.assertEqual([(r, "matches no page" in t) for r, _, t in got], [("allowlist-stale", True)])


class NoticeTexts(unittest.TestCase):
    def test_a_quoted_text_that_is_in_the_code_passes(self):
        self.assertEqual(notice_problems(NOTICE_PAGE, '# T\n\n| "Wait for the swap to finish" | why |\n'), [])

    def test_a_reworded_text_fails_on_the_notice_page(self):
        got = notice_problems(NOTICE_PAGE, '# T\n\n| "Wait for the swap to end" | why |\n')
        self.assertEqual([r for r, _, _ in got], ["notice-text"])
        self.assertIn("Wait for the swap to end", got[0][2])

    def test_the_holes_of_a_format_string_fit_a_shown_number(self):
        self.assertEqual(notice_problems(NOTICE_PAGE, '# T\n\n"Loading 6/10", then "Loading 7/10".\n'), [])
        self.assertEqual([r for r, _, _ in notice_problems(NOTICE_PAGE, '# T\n\n"Loading 6 of 10".\n')], ["notice-text"])

    def test_two_texts_in_one_cell_and_either_word_in_brackets(self):
        text = '# T\n\n| "A setup is placed: save again after it" (or "load again after it") |\n| "A setup is placed: save (load) again after it" |\n'
        self.assertEqual(notice_problems(NOTICE_PAGE, text), [])
        self.assertEqual([r for r, _, _ in notice_problems(NOTICE_PAGE, '# T\n\n"A setup is placed: save (stop) again after it".\n')], ["notice-text"])

    def test_text_around_a_name_or_a_dot_is_matched_piece_by_piece(self):
        self.assertEqual(notice_problems(NOTICE_PAGE, '# T\n\n"Same as ‹saved name›, nothing new saved" and "Practice Stage · Stage requested".\n'), [])
        self.assertEqual([r for r, _, _ in notice_problems(NOTICE_PAGE, '# T\n\n"Same as ‹name›, something else entirely".\n')], ["notice-text"])

    def test_a_text_only_in_a_comment_does_not_count_and_a_char_literal_does_not_shift_the_pairing(self):
        self.assertEqual([r for r, _, _ in notice_problems(NOTICE_PAGE, '# T\n\n"Gone in a comment".\n')], ["notice-text"])
        self.assertEqual(notice_problems(NOTICE_PAGE, '# T\n\n"Real text after a char literal".\n'), [])

    def test_other_player_pages_are_checked_only_in_sentences_about_notices(self):
        about = '# P\n\nThe notice reads "Not in the code at all".\n'
        self.assertEqual([r for r, _, _ in notice_problems("docs/guide/p.md", about)], ["notice-text"])
        for plain in ('# P\n\nThe row is called "Not in the code at all".\n', '# P\n\nThe help line says "Wait for the swap to finish".\n'):
            self.assertEqual(notice_problems("docs/guide/p.md", plain), [], plain)

    def test_only_the_message_column_of_restrictions_is_checked(self):
        row = '# R\n\n| ID | Message | Approved |\n| --- | --- | --- |\n| `a` | "Wait for the swap to finish" | operator ("yes to both") |\n| `b` | "Not a text" | ok |\n'
        cfg_r = cfg(notice_columns={"docs/RESTRICTIONS.md": 2})
        got = notice_problems("docs/RESTRICTIONS.md", row, config=cfg_r)
        self.assertEqual([(r, t.split('"')[1]) for r, _, t in got], [("notice-text", "Not a text")])

    def test_a_listed_text_passes_and_a_stale_entry_fails(self):
        page = '# T\n\nThe notice reads "Built from pieces at run time".\n'
        allowed = cfg(notice_pages=[NOTICE_PAGE], notice_ok=[(NOTICE_PAGE, "Built from pieces at run time", "joined at run time")])
        self.assertEqual(notice_problems(NOTICE_PAGE, page, config=allowed), [])
        self.assertEqual([r for r, _, _ in notice_problems(NOTICE_PAGE, "# T\n\nNothing quoted.\n", config=allowed)], ["allowlist-stale"])

    def test_a_historical_page_may_quote_a_notice_that_is_gone(self):
        text = '# Notes\n\nThe notice read "Long gone".\n'
        self.assertEqual(notice_problems("docs/RELEASE-NOTES-rc5.md", text), [])

    def test_a_tree_without_rust_source_reports_every_quoted_text(self):
        files = dict(CLEAN, **{NOTICE_PAGE: '# T\n\n"Wait for the swap to finish".\n', "docs/README.md": CLEAN["docs/README.md"] + "- [t](guide/troubleshooting.md)\n"})
        self.assertEqual([r for r, _, _ in problems(files, config=STRICT) if r == "notice-text"], ["notice-text"])

    def test_a_text_only_test_diag_or_simulator_code_holds_is_not_a_notice(self):
        """The player never sees what only tests (a `#[cfg(test)]` module, a `*_tests.rs` file, a tests/ folder), diag-only
        code or host-only simulator code hold, so a page that quotes it is wrong."""
        sources = {
            "src/lib.rs": 'pub const A: &str = "Shown to the player";\n'
                          '#[cfg(not(test))] pub const B: &str = "Shown outside tests too";\n'
                          '#[cfg(test)] mod tests { const T: &str = "Only in a test module"; }\n'
                          '#[cfg(test)] mod lib_tests;\n'
                          'mod deep;\n'
                          '#[cfg(feature = "diag")] pub const D: &str = "Only in the diag build";\n'
                          '#[cfg(feature = "simulator")] pub const S: &str = "Only in the simulator";\n'
                          '#[cfg(all(test, feature = "x"))] pub const U: &str = "Only in a gated test";\n',
            "src/lib_tests.rs": 'const F: &str = "Only in a tests file";\n',
            "src/deep.rs": '#[cfg(test)]\nmod tests;\npub const E: &str = "Shown from a nested module";\n',
            "src/deep/tests.rs": 'const G: &str = "Only in a nested tests file";\n',
            "src/tests/more.rs": 'const H: &str = "Only under a tests folder";\n',
            "crates/c/src/lib.rs": 'pub const C: &str = "Shown from a crate";\n',
            "crates/c/tests/it.rs": 'const I: &str = "Only in a crate test";\n',
        }
        shown = '# T\n\n"Shown to the player", "Shown outside tests too", "Shown from a nested module" and "Shown from a crate".\n'
        self.assertEqual(notice_problems(NOTICE_PAGE, shown, sources=sources), [])
        for text in ("Only in a test module", "Only in a tests file", "Only in a nested tests file", "Only under a tests folder",
                     "Only in a crate test", "Only in the diag build", "Only in the simulator", "Only in a gated test"):
            with self.subTest(text):
                got = notice_problems(NOTICE_PAGE, f'# T\n\n"{text}".\n', sources=sources)
                self.assertEqual([r for r, _, _ in got], ["notice-text"], text)

    def test_the_rust_tokenizer_keeps_real_strings_only(self):
        self.assertEqual(d.rust_strings('let a = "x"; // "no"\nlet b = \'"\'; let c: &\'a str = r##"y "# z"##; /* "no" */ let e = "t\\u{e9}\\n";'), ["x", 'y "# z', "té\n"])


def git(root, *args):
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    subprocess.run(["git", "-C", root, *args], check=True, capture_output=True, env=env)


class GateStage(unittest.TestCase):
    """remote-gate's stage leaves docs/media and every .gif out of the work tree (skip-worktree): links to them must
    be judged by what git holds."""

    def stage(self, extra=None, ignore=""):
        root = build(dict(CLEAN, **(extra or {})))
        git(root, "init", "-q")
        if ignore:
            pathlib.Path(root, ".gitignore").write_text(ignore)
        git(root, "add", "-A")
        git(root, "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
            "commit", "-qm", "fixture")
        return root

    def test_a_clip_that_git_holds_but_the_work_tree_lacks_passes(self):
        root = self.stage()
        git(root, "update-index", "--skip-worktree", "docs/media/pic.gif")
        os.remove(os.path.join(root, "docs/media/pic.gif"))
        self.assertEqual(d.check(root, NO_ALLOWANCE), [])

    def test_a_skip_worktree_video_uses_the_indexed_file_bytes(self):
        url = VideoSources.url + VideoSources.blob
        root = self.stage({"README.md": CLEAN["README.md"] + f'\n<video src="{url}"></video>\n',
                           "docs/media/clip.mp4": "fixture mp4"})
        git(root, "update-index", "--skip-worktree", "docs/media/clip.mp4")
        os.remove(os.path.join(root, "docs/media/clip.mp4"))
        self.assertEqual(d.check(root, NO_ALLOWANCE), [])

    def test_a_deleted_video_is_not_mistaken_for_skip_worktree(self):
        url = VideoSources.url + VideoSources.blob
        root = self.stage({"README.md": CLEAN["README.md"] + f'\n<video src="{url}"></video>\n',
                           "docs/media/clip.mp4": "fixture mp4"})
        os.remove(os.path.join(root, "docs/media/clip.mp4"))
        self.assertEqual([p.rule for p in d.check(root, NO_ALLOWANCE)], ["video-src"])

    def test_a_video_blob_must_belong_to_a_committed_mp4(self):
        url = VideoSources.url + VideoSources.blob
        for case in ("ignored", "untracked", "modified", "staged"):
            with self.subTest(case=case):
                path = "docs/media/draft.mp4"
                files = {"README.md": CLEAN["README.md"] + f'\n<video src="{url}"></video>\n'}
                if case in ("modified", "staged"):
                    files[path] = "committed version"
                root = self.stage(files, ignore=path if case == "ignored" else "")
                pathlib.Path(root, path).write_bytes(b"fixture mp4")
                if case == "staged":
                    git(root, "add", path)
                self.assertEqual([p.rule for p in d.check(root, NO_ALLOWANCE)], ["video-src"])

    def test_a_clip_that_git_does_not_hold_fails_even_when_it_is_on_disk(self):
        root = self.stage({"README.md": CLEAN["README.md"] + "\n![x](docs/media/local.gif)\n", "docs/media/local.gif": "GIF"}, ignore="local.gif\n")
        self.assertEqual([(p.rule, p.text) for p in d.check(root, NO_ALLOWANCE)], [("link-missing", "`docs/media/local.gif`: no such file or folder")])

    def test_an_untracked_file_that_is_not_ignored_counts_while_authoring(self):
        root = self.stage()
        pathlib.Path(root, "docs/media/new.gif").write_text("GIF")
        pathlib.Path(root, "README.md").write_text(CLEAN["README.md"] + "\n![x](docs/media/new.gif)\n")
        self.assertEqual(d.check(root, NO_ALLOWANCE), [])

    def test_the_case_of_a_path_must_match_git_s_list(self):
        root = self.stage({"README.md": CLEAN["README.md"] + "\n![x](docs/media/PIC.gif)\n"})
        self.assertEqual([p.rule for p in d.check(root, NO_ALLOWANCE)], ["link-missing"])

    def test_a_folder_link_resolves_through_the_list(self):
        root = self.stage({"README.md": CLEAN["README.md"] + "\n[x](docs/media/)\n"})
        self.assertEqual(d.check(root, NO_ALLOWANCE), [])

    def test_a_git_that_cannot_list_is_an_error_not_a_pass(self):
        root = build(CLEAN)
        pathlib.Path(root, ".git").write_text("not a repository")
        with self.assertRaises(OSError):
            d.check(root, NO_ALLOWANCE)


class Reports(unittest.TestCase):
    def test_a_problem_prints_its_rule_file_and_line(self):
        root = build(dict(CLEAN, **{"README.md": CLEAN["README.md"] + "\n[x](gone.md)\n"}))
        found = d.check(root, NO_ALLOWANCE)
        self.assertTrue(str(found[0]).startswith("link-missing README.md:9: "), str(found[0]))

    def test_main_exits_1_on_a_problem_0_when_clean_and_2_without_docs(self):
        broken = build(dict(CLEAN, **{"README.md": CLEAN["README.md"] + "\n[x](gone.md)\n"}))
        with contextlib.redirect_stdout(io.StringIO()):
            # main() runs the default allowlists, which name pages this small fixture lacks: it reports them as stale
            self.assertEqual(d.main(["--root", broken]), 1)
            self.assertEqual(d.main(["--root", tempfile.mkdtemp()]), 2)

    def test_external_links_lists_targets_and_bare_urls_once(self):
        text = "[a](https://one.invalid/x) and https://one.invalid/x and https://two.invalid/y.\n\n`https://code.invalid/z`\n"
        root = build(dict(CLEAN, **{"README.md": CLEAN["README.md"] + "\n" + text}))
        urls = sorted(u for u, _, _ in d.external_links(root))
        self.assertEqual(urls, ["https://one.invalid/x", "https://two.invalid/y"])


if __name__ == "__main__":
    unittest.main()
