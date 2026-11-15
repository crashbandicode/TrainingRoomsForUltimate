"""Host tests of tools/docs_check.py: the clean fixture passes, and each rule fails on its own seeded break.

Run by `tests/docs_gates.rs` (`python3 -I tools/test_docs_check.py`). Every test builds a small docs tree in a temporary
folder, so nothing depends on the real docs.
"""
import contextlib
import io
import os
import pathlib
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
    "Cargo.toml": '[package]\nversion = "0.2.0-rc.6"\n',
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
        "# Getting started\n\nThe footer reads `0.2.0-rc.6 · rc14 1a2b3c4d`: that is rc.14.\n\n"
        "## Which build\n\nSee [Technical](../TECHNICAL.md#status) and [this page](#which-build).\n"
    ),
    "docs/TECHNICAL.md": "# Technical\n\n## Status\n\nThe current release is rc.14, as the footer shows.\n\n## Other\n\nText.\n",
    "docs/RELEASE-NOTES-rc9.md": "# rc.9\n\nThe newest build is rc.9.\n",
    "docs/media/pic.gif": "GIF",
}

def cfg(**changes):
    """A config with no allowance and no notice pages: the real lists name pages the small fixtures do not have."""
    base = dict(historical=[("docs/RELEASE-NOTES-rc*.md", "a past release")], orphan_ok=[], claim_ok=[], must_name=d.MUST_NAME,
                notice_ok=[], notice_pages=[], notice_columns={})
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

    def test_the_crate_version_is_not_a_release_claim(self):
        text = CLEAN["docs/TECHNICAL.md"] + "\nThe version is currently 0.2.0-rc.6 in every build.\n"
        self.assertEqual(problems({}, **{"docs/TECHNICAL.md": text}), [])

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


def notice_problems(page, text, **config):
    files = dict(CLEAN, **{"src/lib.rs": SRC, "docs/README.md": CLEAN["docs/README.md"] + f"- [p]({page[len('docs/'):]})\n", page: text})
    return [(r, p, t) for r, p, t in problems(files, config=config.pop("config", STRICT)) if r in ("notice-text", "allowlist-stale")]


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

    def test_the_rust_tokenizer_keeps_real_strings_only(self):
        self.assertEqual(d.rust_strings('let a = "x"; // "no"\nlet b = \'"\'; let c: &\'a str = r##"y "# z"##; /* "no" */ let e = "t\\u{e9}\\n";'), ["x", 'y "# z', "té\n"])


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
