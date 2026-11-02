"""Host tests of eden/harness/: the ledger parser on real ledger lines, the interleave rule, the parameter set.

Run: /usr/bin/python3 -I tests/eden/test_harness.py
No Eden, no network: the ledger lines below are copied from the real claim ledger (2026-11-02), owners as written
there, and the queue is a synthetic copy of the queue file's shape.
"""
import sys
import unittest
from dataclasses import replace
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "eden" / "harness"))
import ledger  # noqa: E402
import params  # noqa: E402

NOW = ledger.calendar.timegm((2026, 11, 2, 12, 0, 0))
LEDGER = """\
2026-11-02 11:48:34Z CLAIMED: Codex 01a19bfb-4a8f-7dc1-969f-79c438c25f4c/imgui-nvn-sol Yoga release, one compile from source 370e72e, no Eden. Foreign Yoga claims released.

2026-11-02 11:48:45Z RELEASED: Codex 01a19bfb-4a8f-7dc1-969f-79c438c25f4c/imgui-nvn-sol Yoga release passed; compile ended, no Eden started.

2026-11-02 11:49Z RELEASED: Claude 0caa72b8/f6 (f6 stream) TrainingRooms F6 Switch diag build of the fix (f4a612f) on the Surface done, exit 0 (compile only, no Eden started).

2026-11-02 11:49Z CLAIMED: Claude 0caa72b8/f6-eden TrainingRooms F6 Eden rows (first-boot group-build hitch), solo run f6-01 on the Surface, display :97, own profile /home/intpa/trainingrooms-eden-f6/profiles/f6 (copy of ssbu-desync-1029/profiles/g). No LAN.

2026-11-02 11:53Z CLAIMED: Claude 0caa72b8/stack-eden TrainingRooms stack-stream solo Eden run st-02 on Yoga, display :99, own profile /home/intpa/trainingrooms-eden-st/profiles/y (hard-linked copy of ssbu-desync-1029/profiles/b), muted.

2026-11-02 11:54Z RELEASED: Claude 0caa72b8/stack-eden TrainingRooms stack-stream solo Eden run st-02 stopped, Yoga display :99 stopped; profile retained.

2026-11-02 11:55Z CLAIMED: Claude 0caa72b8/stack-eden TrainingRooms stack-stream solo Eden run st-03 on Yoga, display :99, muted.

2026-10-29 13:08Z CLAIMED again: Claude tangent ssbu-refresh-clock-t2 (session b52c5218) for one validation rerun: Surface display :61 + run a02, Yoga display :62
2026-10-29 17:14Z RELEASED (not started): Claude tangent ssbu-refresh-clock-t2 run a07. The Surface stopped answering; nothing of this owner is running on Yoga.
2026-10-31 19:09Z CLAIMED (build only, no Eden, no display): Claude e202df30 runs a bounded incremental Eden build on the Surface (unit eden-main-build).
2026-10-31 19:17:30Z RELEASED (build only): Claude e202df30 Surface build unit eden-main-build finished; no Eden was started.
2026-11-02 11:57:12Z CLAIMED: Claude 0caa72b8/ui-screens-sync (ui-screens/sync) TrainingRooms gates compile on the Ally WSL, no eden.exe.
2026-11-02 11:59:40Z RELEASED: Claude 0caa72b8/ui-screens-sync (ui-screens/sync) TrainingRooms gate compile on the Ally WSL done.
"""
QUEUE = """\
## Yoga (`memento-claw-yoga-ubuntu`)
- (lead 12:2xZ) order: stack st-01 (running) -> hitbox hb-02 -> others as appended.
- stack (11:40Z): st-01.. thread-stack batch
- imgui-nvn-sol (Codex 01a19bfb, queued 2026-11-02): ABI draft host test, one compile per claim.
## Surface (`ubuntu-surface`)
2. (imgui-nvn-sol took the Yoga at 11:46Z; its Surface line is satisfied. Lead, 12:0xZ)
3. f6 (Opus 5.5, queued 11:10Z as Surface #2): Eden f6-01
## Ally - the lent rig
1. ui-screens - ui-NN batches
"""


def p(**kw):
    base = dict(owner="Claude 0caa72b8/stack-eden", prefix="st", root="/tmp/tr-test-root", host="yoga",
                display=":99", ledger_ssh="", fair_seconds=180)
    base.update(kw)
    return params.load(env={}, **base)


class LedgerParse(unittest.TestCase):
    def test_stamps_with_and_without_seconds_and_every_agent(self):
        events = ledger.parse(LEDGER)
        self.assertEqual(len(events), 13)
        self.assertEqual(events[0].owner, "Codex 01a19bfb-4a8f-7dc1-969f-79c438c25f4c/imgui-nvn-sol")
        self.assertEqual(events[3].owner, "Claude 0caa72b8/f6-eden")
        self.assertEqual(events[2].owner, "Claude 0caa72b8/f6")  # "(f6 stream)" is not part of the owner
        again = [e for e in events if "tangent" in e.text][0]
        self.assertEqual((again.kind, again.owner), ("CLAIMED", "Claude ssbu-refresh-clock-t2"))
        released = [e for e in events if e.kind == "RELEASED" and "ssbu-refresh" in e.text][0]
        self.assertEqual(released.owner, again.owner)
        seconds = [e for e in events if "e202df30" in e.owner]
        self.assertEqual(seconds[1].when - seconds[0].when, 8 * 60 + 30)  # 19:09Z to 19:17:30Z

    def test_a_seconds_stamp_release_closes_the_claim(self):
        fresh, _ = ledger.open_claims(LEDGER, "ally", now=NOW)
        self.assertEqual(fresh, [])  # CLAIMED 11:57:12Z, RELEASED 11:59:40Z
        text = LEDGER.replace("2026-11-02 11:59:40Z RELEASED", "2026-11-02 11:59:40Z NOTE")
        fresh, _ = ledger.open_claims(text, "ally", now=NOW)
        self.assertEqual([c.owner for c in fresh], ["Claude 0caa72b8/ui-screens-sync"])

    def test_claims_belong_to_the_host_they_name(self):
        yoga, _ = ledger.open_claims(LEDGER, "yoga", now=NOW)
        surface, _ = ledger.open_claims(LEDGER, "surface", now=NOW)
        self.assertEqual([c.owner for c in yoga], ["Claude 0caa72b8/stack-eden"])
        self.assertTrue(yoga[0].eden)
        self.assertEqual([c.owner for c in surface], ["Claude 0caa72b8/f6-eden"])  # "(copy of ...profiles/g)" is not a host

    def test_build_only_release_and_not_started_close_the_claim(self):
        for host in ("surface", "yoga"):
            fresh, stale = ledger.open_claims(LEDGER, host, now=NOW, stale_hours=10 ** 6)
            self.assertNotIn("Claude e202df30", [c.owner for c in fresh + stale])
            self.assertNotIn("Claude ssbu-refresh-clock-t2", [c.owner for c in fresh + stale])

    def test_own_claims_and_stale_claims_do_not_block(self):
        fresh, _ = ledger.open_claims(LEDGER, "yoga", exclude="Claude 0caa72b8/stack-eden", now=NOW)
        self.assertEqual(fresh, [])
        fresh, stale = ledger.open_claims(LEDGER, "yoga", now=NOW + 13 * 3600, stale_hours=12)
        self.assertEqual((fresh, [c.owner for c in stale]), ([], ["Claude 0caa72b8/stack-eden"]))

    def test_a_release_naming_one_host_keeps_the_other_claim(self):
        text = ("2026-11-02 10:00Z CLAIMED: Claude 0caa72b8/x one Eden on the Surface :1 and on the Yoga :2\n"
                "2026-11-02 10:30Z RELEASED: Claude 0caa72b8/x stopped Eden on Yoga :2\n")
        self.assertEqual([c.owner for c in ledger.open_claims(text, "surface", now=NOW)[0]], ["Claude 0caa72b8/x"])
        self.assertEqual(ledger.open_claims(text, "yoga", now=NOW)[0], [])

    def test_the_real_ledger_sample_parses_every_dated_line(self):
        lines = [l for l in LEDGER.splitlines() if l]
        self.assertEqual(len(ledger.parse(LEDGER)), len(lines))


class Claiming(unittest.TestCase):
    def test_format_names_only_the_claimed_host_and_has_no_seconds(self):
        line = ledger.format_line(p(), "CLAIMED", "Eden run st-04 on Yoga, display :99", now=NOW)
        self.assertEqual(line, "2026-11-02 12:00Z CLAIMED: Claude 0caa72b8/stack-eden Eden run st-04 on Yoga, display :99")
        with self.assertRaises(SystemExit):
            ledger.format_line(p(), "CLAIMED", "run on Yoga, copy of Surface profile", now=NOW)
        with self.assertRaises(SystemExit):
            ledger.format_line(p(), "CLAIMED", "Eden run", now=NOW)

    def test_a_foreign_open_claim_refuses(self):
        why = ledger.busy(p(host="surface", owner="Claude 0caa72b8/stack-eden"), LEDGER, NOW, queue="")
        self.assertTrue(any("f6-eden" in w for w in why), why)

    def test_fairness_after_own_release(self):
        mine = p(owner="Claude 0caa72b8/stack-eden")
        text = LEDGER.replace("11:55Z CLAIMED", "11:55Z RELEASED")  # last own line is a RELEASED at 11:55
        soon = ledger.calendar.timegm((2026, 11, 2, 11, 56, 0))  # 60 s after the RELEASED
        self.assertTrue(any("fairness" in w for w in ledger.busy(mine, text, soon, queue="")))
        self.assertEqual(ledger.busy(mine, text, NOW, queue=""), [])  # 5 minutes later

    def test_claim_writes_when_free_and_refuses_when_busy(self):
        import tempfile
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "ledger.md"
            path.write_text("", encoding="utf-8")
            mine = p(ledger=str(path))
            line = ledger.claim(mine, "Eden run st-04 on Yoga", now=NOW, text="", queue="")
            self.assertTrue(path.read_text().rstrip().endswith(line))
            other = p(ledger=str(path), owner="Claude 0caa72b8/hitbox-eden")
            with self.assertRaises(ledger.Busy):
                ledger.claim(other, "Eden run hb-02 on Yoga", now=NOW, queue="")
            ledger.release(mine, "Eden run st-04 stopped on Yoga", now=NOW + 60)
            self.assertEqual(ledger.busy(other, now=NOW + 120, queue=""), [])


class InterleaveRule(unittest.TestCase):
    """Queue file rule (lead, 2026-11-02): imgui-nvn-sol takes a queued host after each TrainingRooms run."""

    def test_queue_head_skips_notes_and_reads_the_host_section(self):
        self.assertEqual(ledger.queue_head(QUEUE, "yoga"), "stack (11:40Z): st-01.. thread-stack batch")
        self.assertEqual(ledger.queue_head(QUEUE, "surface"), "f6 (Opus 5.5, queued 11:10Z as Surface #2): Eden f6-01")
        self.assertEqual(ledger.queue_head(QUEUE, "ally"), "ui-screens - ui-NN batches")

    def test_waits_while_queued_and_not_yet_served_since_our_release(self):
        reason = ledger.parallel_turn(LEDGER, QUEUE, "yoga")
        self.assertIn("imgui-nvn-sol", reason)  # TrainingRooms released 11:54Z; imgui claimed 11:48Z (before it)
        why = ledger.busy(p(), LEDGER.replace("11:55Z CLAIMED", "11:55Z RELEASED"), NOW, queue=QUEUE)
        self.assertTrue(any("imgui-nvn-sol" in w for w in why), why)

    def test_clear_once_imgui_claimed_after_our_last_release(self):
        text = LEDGER.replace("11:55Z CLAIMED", "11:55Z RELEASED") + (
            "\n2026-11-02 11:58Z CLAIMED: Codex 01a19bfb-4a8f-7dc1-969f-79c438c25f4c/imgui-nvn-sol Yoga ABI gates, one compile, no Eden.\n"
            "2026-11-02 11:59Z RELEASED: Codex 01a19bfb-4a8f-7dc1-969f-79c438c25f4c/imgui-nvn-sol Yoga ABI gates passed.\n")
        self.assertIsNone(ledger.parallel_turn(text, QUEUE, "yoga"))
        self.assertEqual(ledger.busy(p(), text, NOW, queue=QUEUE), [])

    def test_not_queued_not_waiting(self):
        self.assertIsNone(ledger.parallel_turn(LEDGER, QUEUE, "ally"))  # no imgui line for the Ally
        self.assertIsNone(ledger.parallel_turn(LEDGER, QUEUE, "surface"))  # its Surface line is satisfied
        self.assertIsNone(ledger.parallel_turn(LEDGER, "", "yoga"))  # no queue file

    def test_no_wait_before_our_first_release_on_the_host(self):
        text = "2026-11-02 11:00Z CLAIMED: Codex 01a19bfb/imgui-nvn-sol Yoga build, no Eden.\n"
        self.assertIsNone(ledger.parallel_turn(text, QUEUE, "yoga"))

    def test_the_rule_is_not_applied_to_imgui_itself(self):
        mine = p(owner="Codex 01a19bfb-4a8f-7dc1-969f-79c438c25f4c/imgui-nvn-sol")
        self.assertEqual([w for w in ledger.busy(mine, LEDGER, NOW, queue=QUEUE) if "queued for" in w], [])

    def test_tm_f8_terra_and_its_variants_get_the_same_turn(self):
        queue = QUEUE.replace("## Ally - the lent rig\n", "## Ally - the lent rig\n"
                              "- tm-f8-terra-ally (Codex 0f3c): LDN-free Training row on the lent rig\n")
        released = ("2026-11-02 12:10Z CLAIMED: Claude 0caa72b8/ui-screens TrainingRooms ui rows on the Ally\n"
                    "2026-11-02 12:20Z RELEASED: Claude 0caa72b8/ui-screens TrainingRooms ui rows stopped on the Ally\n")
        self.assertEqual(ledger.parallel_turns(released, queue, "yoga"), [])  # no TrainingRooms release on the Yoga here
        why = ledger.parallel_turns(released, queue, "ally")
        self.assertEqual([w.split()[0] for w in why], ["tm-f8-terra"])
        me = p(host="ally", owner="Claude 0caa72b8/ui-screens")
        self.assertTrue(any("tm-f8-terra" in w for w in ledger.busy(me, released, NOW, queue=queue)))
        for variant in ("tm-f8-terra", "tm-f8-terra-ally", "tm-f8-terra-eden"):
            served = released + f"2026-11-02 12:30Z CLAIMED: Codex 0f3c-aa/{variant} Ally LDN-free row, no LAN\n"
            self.assertEqual(ledger.parallel_turns(served, queue, "ally"), [], variant)
        # a release by one parallel owner does not count as a TrainingRooms release
        only = released + "2026-11-02 12:40Z RELEASED: Codex 0f3c-aa/tm-f8-terra-ally Ally row done\n"
        self.assertEqual(len(ledger.parallel_turns(only, queue, "ally")), 1)

    def test_each_parallel_owner_waits_on_its_own_and_both_can_be_pending(self):
        queue = ("## Yoga\n- imgui-nvn-sol (Codex a): ABI gates\n- tm-f8-terra-eden (Codex b): Eden row\n- stack: batch\n")
        text = "2026-11-02 11:00Z CLAIMED: Claude 0caa72b8/stack TrainingRooms run on Yoga\n2026-11-02 11:10Z RELEASED: Claude 0caa72b8/stack run stopped on Yoga\n"
        self.assertEqual(len(ledger.parallel_turns(text, queue, "yoga")), 2)
        after_one = text + "2026-11-02 11:20Z CLAIMED: Codex a-1/imgui-nvn-sol Yoga ABI gates, no Eden\n"
        self.assertEqual([w.split()[0] for w in ledger.parallel_turns(after_one, queue, "yoga")], ["tm-f8-terra"])
        both = after_one + "2026-11-02 11:30Z CLAIMED: Codex b-1/tm-f8-terra-eden Yoga Eden row\n"
        self.assertEqual(ledger.parallel_turns(both, queue, "yoga"), [])
        for who in ("Codex a-1/imgui-nvn-sol", "Codex b-1/tm-f8-terra-eden"):
            self.assertEqual([w for w in ledger.busy(p(host="yoga", owner=who), text, NOW, queue=queue) if "queued for" in w], [])

    def test_a_compile_release_counts_as_our_release(self):
        text = LEDGER.replace("11:55Z CLAIMED", "11:55Z RELEASED")
        text += "\n2026-11-02 11:56Z CLAIMED: Claude 0caa72b8/slim TrainingRooms host gates on Yoga, no Eden.\n2026-11-02 11:57Z RELEASED: Claude 0caa72b8/slim gates on Yoga done.\n"
        self.assertIn("imgui-nvn-sol", ledger.parallel_turn(text, QUEUE, "yoga"))


class Parameters(unittest.TestCase):
    def test_precedence_given_env_config_default(self):
        import json
        import tempfile
        with tempfile.TemporaryDirectory() as d:
            cfg = Path(d) / "c.json"
            cfg.write_text(json.dumps({"owner": "Claude x/cfg", "display": ":90", "nro": ["/a.nro", "/b.nro:b2.nro"]}))
            env = {"TR_CONFIG": str(cfg), "TR_DISPLAY": ":91", "TR_HOST": "yoga", "TR_PREFIX": "st", "TR_ROOT": "/tmp/r"}
            got = params.load(env=env, owner="Claude y/given")
            self.assertEqual((got.owner, got.display), ("Claude y/given", ":91"))  # given > env > config
            self.assertEqual(got.nro, ("/a.nro", "/b.nro:b2.nro"))
            self.assertEqual(params.load(env={**env, "TR_NRO": "/x.nro,/y.nro"}).nro, ("/x.nro", "/y.nro"))
            self.assertEqual(params.load(env={"TR_HOST": "yoga"}).output, "TrainingRooms")  # default

    def test_hosts_and_derived_values(self):
        self.assertEqual(params.detect_host("ubuntu-surface"), "surface")
        self.assertEqual(params.detect_host("yoga-ubuntu"), "yoga")
        with self.assertRaises(SystemExit):
            params.detect_host("dreamland-ally")
        y, s = p(host="yoga"), p(host="surface")
        self.assertEqual((y.profile, s.profile), ("y", "g"))
        self.assertTrue(y.info["gpu_lock"] and not s.info["gpu_lock"])
        self.assertEqual(s.info["forced"], {"lan_online": "false", "airplane_mode": "true"})
        self.assertEqual(y.aside, "plugins-set-aside-trst")
        self.assertEqual(str(y.data_dir), "/tmp/tr-test-root/profiles/y/user/sdmc/ultimate/TrainingRooms")
        self.assertEqual(p(host="ally").ally_root, "C:/Eden/c1162-w5-ally")  # the lent rig
        self.assertEqual(params.load(env={"TR_ALLY_ROOT": "D:\\Eden\\other\\", "TR_HOST": "ally"}).ally_root, "D:/Eden/other")

    def test_labels_are_the_streams_prefix(self):
        mine = p()
        self.assertEqual(params.label_ok(mine, "st-03"), "st-03")
        for bad in ("s6-01", "st-1", "tr101", "st-03x"):
            with self.assertRaises(SystemExit):
                params.label_ok(mine, bad)
        self.assertEqual(params.label_ok(p(prefix="f6"), "f6-12"), "f6-12")

    def test_never_display_61_profile_a_or_a_relative_root(self):
        for bad in ({"display": ":61"}, {"display": "99"}, {"profile": "a"}, {"root": "rel/dir"}, {"prefix": "S T"}):
            with self.assertRaises(SystemExit, msg=bad):
                p(**bad)

    def test_dump_is_what_the_eden_host_reads_back(self):
        import json
        dumped = json.loads(params.dump(p(nro=("/local/x.nro",))))
        self.assertNotIn("nro", dumped)  # driver-side paths stay home
        self.assertEqual(params.load(env={}, **dumped).owner, "Claude 0caa72b8/stack-eden")
        self.assertEqual(params.split_spec("/a/b.nro:c.nro"), (Path("/a/b.nro"), "c.nro"))


def synthetic_profile(top, old=b"old"):
    """A source profile in the shape of the real one: config, read-only game content, one plugin, old plugin data."""
    src = top / "src-b"
    (src / "user/config").mkdir(parents=True)
    (src / "user/config/qt-config.ini").write_text(
        f"nand_directory={src}/user/nand\nsdmc_directory={src}/user/sdmc\npauseWhenInBackground=true\n"
        "volume=100\nvolume\\default=true\nother=1\n", encoding="utf-8")
    for rel in ("user/nand/user/Contents", "user/nand/system/Contents", "user/firmware"):
        (src / rel).mkdir(parents=True)
        (src / rel / "game.bin").write_bytes(b"x" * 100)
    plugins = src / "user/sdmc/atmosphere/contents/01006A800016E000/romfs/skyline/plugins"
    plugins.mkdir(parents=True)
    (plugins / "libother.nro").write_bytes(b"original-plugin")
    old_data = src / "user/sdmc/ultimate/TrainingRooms"
    old_data.mkdir(parents=True)
    (old_data / "events.log").write_bytes(old)
    return src


class GvProfile(unittest.TestCase):
    def setUp(self):
        import gv
        import tempfile
        self.gv = gv
        self.tmp = tempfile.TemporaryDirectory()
        self.top = Path(self.tmp.name)
        self.src = synthetic_profile(self.top)
        self.p = p(root=str(self.top / "work"), source=str(self.src))
        gv.configure(self.p)
        self.nro = self.top / "libtr.nro"
        self.nro.write_bytes(b"plugin-under-test")

    def tearDown(self):
        self.tmp.cleanup()

    def prep(self, label="st-01", *extra):
        args = self.gv.build_parser().parse_args(["prep", label, *extra])
        args.fn(args)

    def test_prep_links_game_content_forces_mute_sets_aside_and_receipts(self):
        import json
        settings = self.top / "settings.json"
        settings.write_text('{"a": 1}')
        self.prep("st-01", "--nro", str(self.nro), "--settings", str(settings), "--put", f"{settings}:extra.json")
        prof = self.p.profile_dir
        content = prof / "user/nand/user/Contents/game.bin"
        self.assertEqual(content.stat().st_nlink, 2)  # hard-linked, not copied
        self.assertEqual(content.stat().st_ino, (self.src / "user/nand/user/Contents/game.bin").stat().st_ino)
        config = (prof / "user/config/qt-config.ini").read_text().splitlines()
        for line in ("volume=0", "volume\\default=false", "pauseWhenInBackground=false", "other=1",
                     f"sdmc_directory={prof}/user/sdmc"):
            self.assertIn(line, config)
        plugins = prof / self.gv.PLUGINS
        self.assertEqual([x.name for x in plugins.glob("*.nro")], ["libtr.nro"])
        self.assertEqual(len(list((plugins.parent / self.p.aside).glob("libother.*.nro"))), 1)
        data = self.p.data_dir
        self.assertEqual((data / "settings.json").read_text(), '{"a": 1}')
        self.assertTrue((data / "extra.json").exists() and not (data / "settings.json").is_symlink())
        self.assertEqual((data / "events.log").exists(), False)  # the old data was archived, not left for the boot
        self.assertEqual((prof / "user/sdmc/ultimate/TrainingRooms.before-st-01/events.log").read_bytes(), b"old")
        receipt = json.loads((Path(self.p.root) / "PREP-st-01.json").read_text())
        self.assertEqual(receipt["plugins"]["libtr.nro"], self.gv.digest(self.nro))
        self.assertEqual(receipt["put"]["settings.json"]["text"], '{"a": 1}')
        self.assertEqual((self.src / "user/config/qt-config.ini").read_text().count("volume=100"), 1)  # source untouched

    def test_second_prep_archives_again_and_keep_data_keeps(self):
        self.prep("st-01", "--nro", str(self.nro))
        (self.p.data_dir / "save_states.json").write_text("{}")
        self.prep("st-02", "--nro", str(self.nro), "--keep-data")
        self.assertTrue((self.p.data_dir / "save_states.json").exists())
        self.prep("st-03", "--nro", str(self.nro))
        self.assertFalse((self.p.data_dir / "save_states.json").exists())
        self.assertTrue((self.p.sdmc / "ultimate/TrainingRooms.before-st-03/save_states.json").exists())

    def test_the_fork_nro_and_a_missing_nro_are_refused(self):
        fork = self.top / "libtraining_modpack.nro"
        fork.write_bytes(b"fork")
        with self.assertRaises(AssertionError):
            self.prep("st-01", "--nro", str(fork))
        with self.assertRaises(AssertionError):
            self.prep("st-04", "--nro", str(self.top / "nope.nro"))

    def test_restore_puts_the_original_set_back_and_removes_our_data(self):
        self.prep("st-01", "--nro", str(self.nro), "--put", f"{self.nro}:blob.bin")
        self.prep("st-02", "--nro", f"{self.nro}:libzz.nro")  # a second set-aside round
        self.gv.restore(None)
        plugins = self.p.profile_dir / self.gv.PLUGINS
        self.assertEqual({x.name: x.read_bytes() for x in plugins.glob("*.nro")}, {"libother.nro": b"original-plugin"})
        self.assertFalse((plugins.parent / self.p.aside).exists())
        self.assertEqual(list((self.p.sdmc / "ultimate").glob("TrainingRooms*")), [])

    def test_restore_refuses_when_an_original_is_gone(self):
        self.prep("st-01", "--nro", str(self.nro))
        for f in (self.p.profile_dir / self.gv.PLUGINS.parent / self.p.aside).glob("*.nro"):
            f.unlink()
        with self.assertRaises(AssertionError):
            self.gv.restore(None)

    def test_the_source_profile_a_is_never_used(self):
        a = self.top / "a"
        a.mkdir()
        self.gv.configure(p(root=str(self.top / "w2"), source=str(a)))
        with self.assertRaises(AssertionError):
            self.prep("st-01")


class GvChecks(unittest.TestCase):
    def setUp(self):
        import gv
        self.gv = gv
        gv.configure(p())

    def test_muted_reads_every_reply_shape(self):
        for ok in (0, "0", {"key": "core.volume", "value": 0}, {"value": "0"}, {"result": {"effective": 0.0}}):
            self.assertTrue(self.gv.muted(ok), ok)
        for bad in (50, {"value": 50}, {"value": 0, "effective": 100}, {}, {"key": "core.volume"}, None, True):
            self.assertFalse(self.gv.muted(bad), bad)

    def test_scan_fails_unmapped_below_0x1000_or_in_the_window_only(self):
        log = ["[   1.000000] Frontend <Info> main: Booting",
               "[  10.000000] HW.Memory <Error> memory.cpp:1: Unmapped Read32 @ 0x0000000000000008",
               "[  20.000000] HW.Memory <Error> memory.cpp:1: Unmapped Write64 @ 0x0000007100001000",
               "[  30.000000] HW.Memory <Error> memory.cpp:1: Unmapped Read8 @ 0x0000007100000038",
               "[  31.000000] Core <Critical> panic in thread",
               "[  32.000000] Service.FS <Error> nothing to see"]
        alone = self.gv.verdict(log)
        self.assertEqual([n for n, _ in alone["fail"]], [2])  # 0x8 < 0x1000
        self.assertEqual([n for n, _ in alone["unmapped_other"]], [3, 4])
        self.assertEqual([n for n, _ in alone["review"]], [5])
        windowed = self.gv.verdict(log, (25.0, 35.0))
        self.assertEqual([n for n, _ in windowed["fail"]], [2, 4])  # inside the plugin's window now
        self.assertEqual([n for n, _ in self.gv.verdict(log[:1] + log[5:])["fail"]], [])
        # the old line scan the evidence docs cite still sees the same lines
        self.assertEqual([n for n, _ in self.gv.scan_lines(log)], [2, 3, 4, 5])

    def test_arguments_parse_like_the_playbook_and_labels_are_checked(self):
        ap = self.gv.build_parser()
        a = ap.parse_args(["press", "st-01", "lstick,up", "150"])
        self.assertEqual((a.label, a.buttons, a.hold), ("st-01", "lstick,up", 150))
        a = ap.parse_args(["prep", "st-02", "--nro", "/a.nro:b.nro", "--pilot", "--tile", "--tree", "/f:ultimate/X:sub", "/pos.nro"])
        self.assertEqual((a.nro, a.nros, a.pilot, a.tile, a.tree), (["/a.nro:b.nro"], ["/pos.nro"], True, True, ["/f:ultimate/X:sub"]))
        a = ap.parse_args(["stick", "st-01", "0", "-1", "250"])
        self.assertEqual((a.x, a.y, a.ms), (0.0, -1.0, 250))
        self.assertEqual(ap.parse_args(["lines", "tile", "--after", "7"]).after, 7)
        self.assertEqual(ap.parse_args(["scan", "st-01", "--window", "1:2"]).window, "1:2")
        with self.assertRaises(SystemExit):
            ap.parse_args(["press", "e19", "a"])  # the playbook's own label is not this stream's


class Route(unittest.TestCase):
    """Route step generation and gating on a scripted fake instance (no Eden, no Pillow)."""

    def setUp(self):
        import route
        self.route = route
        self.real_dist = route.dist

    def tearDown(self):
        self.route.dist = self.real_dist

    def args(self, *extra):
        a = self.route.build_parser().parse_args(["st-01", *extra])
        a.install_gate = a.install_gate or "result=installed"
        return a

    def session(self, bad=(), logs=None, on_press=None):
        route = self.route

        class Clock:
            t = 0.0

        class Fake(route.Transport):
            def __init__(self):
                self.calls, self.logs, self.n = [], logs if logs is not None else {"events": ["x result=installed y"], "tile": ["old settled screen=sss at=6 (before)"]}, 0

            def press(self, buttons, hold=150):
                self.calls.append(("press", buttons))
                if on_press:
                    on_press(self, buttons)

            def stick(self, x, y, ms):
                self.calls.append(("stick", x, y, ms))

            def shot(self, name):
                self.calls.append(("shot", name))
                self.n += 1
                return Path(f"/shots/{name}.png")

            def lines(self, name, after=0):
                return self.logs.get(name, [])[after:]

            def macro(self, name, cap=120):
                self.calls.append(("macro", name))
                return {"sha256": "ab" * 32, "state": "finished", "error": None}

        def sleep(s):
            Clock.t += s

        route.dist = lambda png, ref, box: 99.0 if any(b in str(ref) for b in bad) else 1.0
        t = Fake()
        return route.Session(t, "/refs", sleep=sleep, clock=lambda: Clock.t), t

    def test_default_plan_is_the_hybrid_route_with_every_wait_gated(self):
        plan = self.route.plan(self.args())
        kinds = [s[0] for s in plan]
        self.assertEqual(plan[0], ("wait_line", "events", "result=installed", 240, False))
        self.assertEqual(kinds.count("tile"), 7)  # right x6 on the stage select, up on the character select
        self.assertEqual([s for s in plan if s[0] == "tile"][0], ("tile", "right", 0.5))
        self.assertEqual([s for s in plan if s[0] == "tile"][-1], ("tile", "up", 0.6))
        self.assertIn(("wait_line", "tile", "settled screen=sss at=6", 10, False), plan)
        self.assertEqual(plan[-1], ("wait_match", "training scene"))
        self.assertEqual([s[1] for s in plan if s[0] == "wait_match"],
                         ["stage select", "character select", "ready to fight", "training scene"])
        self.assertEqual([s for s in plan if s[0] == "macro"], [("macro", "tr-main-to-stage-select.edenmacro.json")])
        # no step is a bare sleep: the only waits are log lines, screenshot matches and tile pauses
        self.assertTrue(all(k in ("wait_line", "wait_match", "main menu", "macro", "tile", "press") for k in kinds))

    def test_plan_options(self):
        plan = self.route.plan(self.args("--stage-steps", "down:3,right:8", "--tile-gate", "41", "--stage-ref", "sss-t41.png",
                                         "--p1-steps", "up,left", "--from-menu", "--install-gate", "plugin=training_rooms/",
                                         "--training-gate", "diag:[CharacterRoster] observed owner=0"))
        self.assertEqual(plan[0][2], "plugin=training_rooms/")
        self.assertNotIn(("main menu",), plan)
        self.assertEqual([s[1] for s in plan if s[0] == "tile"], ["down"] * 3 + ["right"] * 8 + ["up", "left"])
        self.assertIn(("wait_match", "stage name", "sss-t41.png"), plan)
        self.assertEqual(plan[-1], ("wait_line", "diag", "[CharacterRoster] observed owner=0", 90, True))
        self.assertEqual(self.route.parse_steps("up"), ["up"])
        self.assertEqual(self.route.parse_steps(""), [])

    def test_full_run_presses_in_order_and_keeps_the_macro_receipt(self):
        def tile_log(inst, buttons):
            if buttons == "lstick,right":
                inst.logs["tile"].append("settled screen=sss at=%d" % (len([c for c in inst.calls if c == ("press", buttons)])))

        s, t = self.session(on_press=tile_log)
        self.route.to_training(s, self.args())
        presses = [c[1] for c in t.calls if c[0] == "press"]
        self.assertEqual(presses, ["a"] + ["lstick,right"] * 6 + ["a", "lstick,up", "a", "a"])  # menu A, stage, A, P1, A, A
        self.assertEqual([c for c in t.calls if c[0] == "macro"], [("macro", "tr-main-to-stage-select.edenmacro.json")])

    def test_the_tile_gate_ignores_lines_from_before_the_steps(self):
        s, t = self.session()  # the tile log already has "at=6" from an earlier boot; the steps never produce it
        with self.assertRaises(SystemExit) as stop:
            self.route.to_training(s, self.args())
        self.assertEqual(stop.exception.code, 2)
        self.assertEqual([c[1] for c in t.calls if c[0] == "press"].count("a"), 1)  # no A after the missed gate

    def test_a_screen_that_never_matches_stops_without_pressing_on(self):
        for screen, ref in (("character select", "css.png"), ("stage select", "sss-t0.png"), ("ready to fight", "ready.png")):
            s, t = self.session(bad=(ref,), on_press=lambda i, b: i.logs["tile"].append("settled screen=sss at=6"))
            with self.assertRaises(SystemExit) as stop:
                self.route.to_training(s, self.args())
            self.assertEqual(stop.exception.code, 2, screen)
        s, t = self.session(bad=("css.png",), on_press=lambda i, b: i.logs["tile"].append("settled screen=sss at=6"))
        with self.assertRaises(SystemExit):
            self.route.to_training(s, self.args())
        self.assertNotIn(("press", "lstick,up"), t.calls)  # never onto a screen it has not seen

    def test_the_main_menu_gate_gives_up_after_ten_presses(self):
        s, t = self.session(bad=("menu.png",))
        with self.assertRaises(SystemExit):
            self.route.to_training(s, self.args())
        self.assertEqual(len([c for c in t.calls if c == ("press", "a")]), 10)
        self.assertEqual([c for c in t.calls if c[0] == "macro"], [])

    def test_a_missing_install_line_stops_the_route_first(self):
        s, t = self.session(logs={"events": [], "tile": []})
        with self.assertRaises(SystemExit):
            self.route.to_training(s, self.args())
        self.assertEqual(t.calls, [])

    def test_the_training_gate_can_be_a_plugin_line(self):
        logs = {"events": ["plugin=training_rooms/0.1 cut"], "tile": [], "diag": ["[CharacterRoster] observed owner=0 x"]}
        s, t = self.session(logs=logs, on_press=lambda i, b: i.logs["tile"].append("settled screen=sss at=6"))
        a = self.args("--install-gate", "plugin=training_rooms/", "--training-gate", "diag:[CharacterRoster] observed owner=0")
        self.route.to_training(s, a)
        self.assertEqual(t.calls[-1], ("shot", "training-scene"))


class AllyAdapter(unittest.TestCase):
    def test_wait_log_hits_misses_and_continues_after_the_hit(self):
        import importlib.util
        import tempfile
        spec = importlib.util.spec_from_file_location("wa", ROOT / "eden/harness/ally/wa.py")
        wa = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(wa)
        with tempfile.TemporaryDirectory() as d:
            log = Path(d) / "console.log"
            log.write_bytes(b"one\ntwo target\nthree\nfour target\npartial tar")
            clock = type("C", (), {"t": 0.0})
            sleep = lambda s: setattr(clock, "t", clock.t + s)  # noqa: E731
            now = lambda: clock.t  # noqa: E731
            hit, pos = wa.wait_log(log, 0, "target", 5, sleep=sleep, clock=now)
            self.assertEqual((hit, pos), ("two target", len(b"one\ntwo target\n")))
            hit, pos = wa.wait_log(log, pos, "target", 5, sleep=sleep, clock=now)
            self.assertEqual(hit, "four target")
            self.assertEqual(wa.wait_log(log, pos, "target", 1, True, sleep, now)[0], None)  # the partial line is not a line
            with self.assertRaises(TimeoutError):
                wa.wait_log(log, pos, "target", 1, False, sleep, now)
            self.assertEqual(wa.wait_log(log, pos, "target", 5, True, sleep, now)[1], pos)  # unchanged: nothing complete

    def test_the_rig_defaults_to_the_lent_rig_and_needs_a_claim(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location("wa2", ROOT / "eden/harness/ally/wa.py")
        wa = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(wa)
        self.assertEqual(str(wa.O).replace("\\", "/"), "C:/Eden/c1162-w5-ally")
        with self.assertRaises(AssertionError):
            wa.owned()  # no claim configured: refuses before touching the rig


if __name__ == "__main__":
    unittest.main()
