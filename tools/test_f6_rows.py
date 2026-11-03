"""Exercise F6 row failures without Eden or a claim."""
import argparse
import importlib.util
import json
from pathlib import Path
import tempfile
import types
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("f6_rows", Path(__file__).resolve().parents[1] / "eden/f6_rows.py")
rows = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rows)


class Rows(unittest.TestCase):
    def run_rows(self, native, complete, prefetch=True, bootstrap=False, observed=True, native_off=True, delivered=True, events=None):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "route.py").touch()
            diag = root / "practice-diagnostics.log"
            diag.write_text("[GroupSwap] prefetched\n" if prefetch else "")
            if native and native_off:
                with diag.open("a") as stream:
                    stream.write("[CyclePreload] loaded slots=0 fighters=true bank_members=0 group_swap=false\n")
            if bootstrap and observed:
                with diag.open("a") as stream:
                    stream.write("[CharacterRoster] observed owner=0\n[CharacterRoster] observed owner=1\n")
            if bootstrap and delivered:
                with diag.open("a") as stream:
                    stream.write('[PreloadStatus] delivered text=Some("Swap both once to load")\n')
            presses = []

            def press(buttons, hold):
                presses.append(buttons)
                if complete:
                    with diag.open("a") as stream:
                        role = 0 if buttons == "zl,right" else 1
                        stream.write(f"[CharacterRoster] queued owner={role}\n")
                        if events is not None:
                            stream.write(events)
                            return
                        event = "[CharacterRoster] complete" if native else "[BodyBank] ready"
                        stream.write(f"{event} owner={role} kind=0x6\n")
                        if not native and not bootstrap:
                            adapter = "MiiOrdinaryPlayer" if role == 0 else "MiiOrdinaryCpu"
                            ticket = "Ticket { epoch: 2, entry: 76789817344, sequence: " + str(len(presses)) + " }"
                            stream.write(f"[{adapter}] bank switch ticket={ticket} outgoing=0x1c target=0x6 index=0 mii=false\n")
                            stream.write(f"[{adapter}] bank ready ticket={ticket} target=0x6 physical=0x6 released=true\n")

            session = types.SimpleNamespace(press=press, t=types.SimpleNamespace(shot=lambda name: root / (name + ".png")))
            modules = {"params": types.SimpleNamespace(load=lambda: types.SimpleNamespace(data_dir=root)),
                       "route": types.SimpleNamespace(session=lambda *_: session)}
            args = argparse.Namespace(harness=str(root), label="f6-99", native_reload=native,
                                      settle=15, p1=1 if bootstrap else 2, cpu=1 if bootstrap else 0,
                                      gap=6, bootstrap=bootstrap)
            ticks = iter(range(0, 10000, 5))
            error = None
            with patch.dict("sys.modules", modules), patch.object(rows, "HERE", root), \
                 patch.object(rows.time, "time", side_effect=lambda: next(ticks)), \
                 patch.object(rows.time, "sleep"):
                try:
                    rows.laps(args)
                except SystemExit as exc:
                    error = str(exc)
            receipt = root / "runs/f6-99" / ("bootstrap.jsonl" if bootstrap else "laps.jsonl")
            records = [json.loads(line) for line in receipt.read_text().splitlines()] if receipt.exists() else []
            return presses, records, error

    def test_missing_prefetch_sends_no_input(self):
        presses, records, error = self.run_rows(False, False, prefetch=False)
        self.assertEqual((presses, records), ([], []))
        self.assertIn("no prefetched group", error)

    def test_bootstrap_requires_observed_roles_and_bank_completion_before_laps(self):
        presses, records, error = self.run_rows(False, True, prefetch=False, bootstrap=True)
        self.assertIsNone(error)
        self.assertEqual(presses, ["zl,right", "zl,left"])
        self.assertEqual([r["phase"] for r in records], ["bootstrap", "bootstrap"])
        self.assertEqual([r["role"] for r in records], [0, 1])

    def test_bootstrap_stops_on_missing_body_without_sending_cpu_cycle(self):
        presses, records, error = self.run_rows(False, False, prefetch=False, bootstrap=True)
        self.assertEqual(presses, ["zl,right"])
        self.assertIsNone(records[0]["done"])
        self.assertIn("GATE FAIL", error)

    def test_bootstrap_refuses_an_unobserved_role(self):
        presses, records, error = self.run_rows(False, True, prefetch=False, bootstrap=True, observed=False)
        self.assertEqual((presses, records), ([], []))
        self.assertIn("was not observed", error)

    def test_bootstrap_waits_for_training_notice_not_just_early_owner_observation(self):
        presses, records, error = self.run_rows(False, True, bootstrap=True, delivered=False)
        self.assertEqual((presses, records), ([], []))
        self.assertIn("notice was not delivered", error)

    def test_missing_completion_retains_failure_and_stops(self):
        presses, records, error = self.run_rows(False, False)
        self.assertEqual(len(presses), 1)
        self.assertEqual(len(records), 1)
        self.assertIsNone(records[0]["done"])
        self.assertIn("GATE FAIL", error)

    def test_native_reload_requires_its_own_completion_without_prefetch(self):
        presses, records, error = self.run_rows(True, True, prefetch=False)
        self.assertIsNone(error)
        self.assertEqual(len(presses), 2)
        self.assertEqual(len(records), 2)
        self.assertTrue(all("[CharacterRoster] complete owner=0" in row["done"] for row in records))

    def test_native_baseline_refuses_unproved_off_configuration(self):
        presses, records, error = self.run_rows(True, True, native_off=False)
        self.assertEqual((presses, records), ([], []))
        self.assertIn("Off was not observed", error)

    def test_native_owner_ready_requires_matching_role_and_released_lease(self):
        line = "536782616 #307 [MiiOrdinaryPlayer] ready ticket=Ticket { epoch: 2, entry: 76769632256, sequence: 1 } target=0x2d index=0 resource=[00] released=true saved_files_written=false"
        self.assertTrue(rows.native_complete(line, 0))
        self.assertFalse(rows.native_complete(line, 1))
        self.assertFalse(rows.native_complete(line.replace("released=true", "released=false"), 0))
        self.assertFalse(rows.native_complete(line.replace("] ready ticket=", "] bank ready ticket="), 0))


    def test_matching_bank_transaction_completes_each_press(self):
        presses, records, error = self.run_rows(False, True)
        self.assertIsNone(error)
        self.assertEqual(len(presses), 2)
        self.assertTrue(all("bank ready ticket=" in r["done"] for r in records))

    def test_fallback_ready_does_not_complete_refused_request(self):
        # Actual f6-14 transaction: requested Kirby, fallback readied R.O.B.,
        # then the native adapter refused the same ticket.
        events = (
            "566809913 #421 [MiiOrdinaryPlayer] bank switch ticket=Ticket { epoch: 2, entry: 76789817344, sequence: 4 } outgoing=0x1c target=0x6 index=0 mii=false\n"
            "566809961 #428 [BodyBank] ready owner=0 index=0 kind=0x2d previous=Changing bodies=[]\n"
            "566809962 #431 [MiiOrdinaryPlayer] bank switch refused ticket=Ticket { epoch: 2, entry: 76789817344, sequence: 4 } diverged=false owner_kept=true INSTALL_released=true reason=bank settled on another member\n"
        )
        presses, records, error = self.run_rows(False, True, events=events)
        self.assertEqual(presses, ["zl,right"])
        self.assertEqual(len(records), 1)
        self.assertIsNone(records[0]["done"])
        self.assertIn("bank switch refused", records[0]["refused"])
        self.assertIn("refused", error)

    def test_bank_receipt_requires_current_role_ticket_target_and_release(self):
        ticket = "Ticket { epoch: 2, entry: 76789817344, sequence: 4 }"
        request = f"[MiiOrdinaryPlayer] bank switch ticket={ticket} outgoing=0x1c target=0x6 index=0"
        ready = f"[MiiOrdinaryPlayer] bank ready ticket={ticket} target=0x6 physical=0x6 released=true"
        self.assertEqual(rows.cycle_result(ready, 0, request), "ready")
        for wrong in (ready.replace("sequence: 4", "sequence: 3"),
                      ready.replace("epoch: 2", "epoch: 1"),
                      ready.replace("target=0x6", "target=0x60"),
                      ready.replace("released=true", "released=false"),
                      ready.replace("MiiOrdinaryPlayer", "MiiOrdinaryCpu")):
            self.assertIsNone(rows.cycle_result(wrong, 0, request))
        self.assertIsNone(rows.cycle_result(ready, 1, request))
        self.assertIsNone(rows.cycle_result(ready, 0, None))



    def test_native_original_return_requires_its_own_matching_receipt(self):
        # Actual f6-18 sequence8 returns to original kind0x58 through submit,
        # not a bank reuse. Its native ready receipt still proves completion.
        ticket = "Ticket { epoch: 2, entry: 75985362944, sequence: 8 }"
        request = f"[MiiOrdinaryPlayer] submit ticket={ticket} outgoing=0x2e target=0x58 index=0 bank_build=true"
        ready = f"[MiiOrdinaryPlayer] ready ticket={ticket} target=0x58 index=0 released=true saved_files_written=false"
        self.assertEqual(rows.cycle_result(ready, 0, request), "ready")
        self.assertIsNone(rows.cycle_result(ready.replace("] ready", "] bank ready"), 0, request))
        self.assertIsNone(rows.cycle_result(ready.replace("target=0x58", "target=0x2e"), 0, request))


if __name__ == "__main__":
    unittest.main()
