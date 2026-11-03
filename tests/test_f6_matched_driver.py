"""Offline driver scenarios: no Eden, controller or profile access."""
import argparse
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import sys
import tempfile
import types
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('f6_rows', ROOT / 'eden/f6_rows.py')
rows = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rows)

QUALIFICATION = '''590866092 #1 [PracticeDiagnostics] adapter=ring-diag session_start;
591121258 #124 [CharacterRoster] observed owner=0 entry=0x11ebe3e000 kind=0x58 epoch=1
591121429 #128 [CharacterRoster] observed owner=1 entry=0x11f60d8000 kind=0x42 epoch=1
591123222 #232 [MiiAutoInit] catalog read queued generations=[2, 2] opening=2
591124188 #240 [TrainingRooms] chords_passed bits=0x20
591124189 #241 [CharacterRoster] request handed to the ordinary Mii owner role=Player selection=Cycle
591125169 #249 [MiiAutoInit] catalog ready; capture_armed=[true, true] generations=[2, 2] opening=2
591125189 #250 [MiiOrdinaryPlayer] catalog owner qualified; immutable origin retained
591125208 #251 [MiiOrdinaryCpu] catalog owner qualified; immutable origin retained'''.splitlines()

class MatchedDriver(unittest.TestCase):
    def run_lane(self, native, bootstrap, fault=None):
        with tempfile.TemporaryDirectory(prefix='f6-matched-') as directory:
            root = Path(directory)
            (root / 'route.py').touch()
            diag = root / 'practice-diagnostics.log'
            clock = [100.0]
            presses = []
            def emit(line):
                with diag.open('a') as f:
                    f.write(line + '\n')
            emit('[CyclePreload] loaded slots=0 fighters=true bank_members=0 group_swap=false' if native else
                 '[CyclePreload] loaded slots=5 fighters=true bank_members=3 group_swap=true')
            for line in QUALIFICATION[:6] if fault in ('late_qualification', 'missing_qualification') else QUALIFICATION:
                emit(line)
            if not native:
                emit('[PreloadStatus] delivered text=Some("Swap both once to load")')
                emit('[GroupSwap] prefetched')
            if fault == 'wrong_config':
                diag.write_text(diag.read_text().replace('loaded slots=', 'loaded other='))
            if fault == 'wrong_start':
                diag.write_text(diag.read_text().replace('kind=0x58', 'kind=0x55'))
            targets = {0: [0x2d] if bootstrap else [0x2c,0x1c,6,0xb,0x1f,0x2e,0x58,0x2d,0x2c,0x1c,6],
                       1: [8] if bootstrap else [0xf,1,7,2]}
            counts = [0,0]
            def press(button, duration):
                role = 0 if button == 'zl,right' else 1
                i = counts[role]
                counts[role] += 1
                presses.append((role, clock[0]))
                target = targets[role][i]
                if fault == 'wrong_target':
                    target = 0x55
                adapter = 'MiiOrdinaryPlayer' if role == 0 else 'MiiOrdinaryCpu'
                ticket = f'Ticket {{ epoch: 2, entry: {256*(role+1)}, sequence: {i+1} }}'
                mode = 'submit' if native or bootstrap else 'bank switch'
                emit(f'100 #1 [{adapter}] {mode} ticket={ticket} outgoing=0x58 target={target:#x}')
                # A role-only legacy completion must never rescue a bad strict result.
                emit(f'101 #2 [CharacterRoster] complete owner={role}')
                returned_ticket = ticket.replace('epoch: 2', 'epoch: 99') if fault == 'wrong_ticket' else ticket
                returned_role = 'MiiOrdinaryCpu' if fault == 'wrong_role' and role == 0 else adapter
                event = 'ready' if mode == 'submit' else 'bank ready'
                if fault == 'refused':
                    event = 'refused'
                released = 'false' if fault == 'unreleased' else 'true'
                emit(f'102 #3 [{returned_role}] {event} ticket={returned_ticket} target={target:#x} released={released}')
                if not native and fault != 'missing_bank':
                    emit(f'[BodyBank] ready owner={role}')
            old_here, old_time = rows.HERE, rows.time
            old_modules = {k:sys.modules.get(k) for k in ('route','params')}
            sys.modules['route'] = types.SimpleNamespace(session=lambda *args: types.SimpleNamespace(
                press=press, t=types.SimpleNamespace(shot=lambda name: name)))
            sys.modules['params'] = types.SimpleNamespace(load=lambda: types.SimpleNamespace(data_dir=root))
            rows.HERE = root
            def sleep(seconds):
                clock[0] += seconds
                if fault == 'late_qualification' and clock[0] == 101.0:
                    for line in QUALIFICATION[6:]:
                        emit(line)
            rows.time = types.SimpleNamespace(time=lambda:clock[0], monotonic=lambda:clock[0],
                                               sleep=sleep)
            args = argparse.Namespace(label='dry',harness=str(root),bootstrap=bootstrap,matched=True,
                                      native_reload=native,p1=1 if bootstrap else 11,cpu=1 if bootstrap else 4,
                                      gap=6,settle=1)
            error = None
            try:
                with contextlib.redirect_stdout(io.StringIO()):
                    try:
                        rows.laps(args)
                    except SystemExit as exc:
                        error = str(exc)
                file = root / 'runs/dry' / ('bootstrap.jsonl' if bootstrap else 'laps.jsonl')
                records = [json.loads(line) for line in file.read_text().splitlines()] if file.exists() else []
            finally:
                rows.HERE, rows.time = old_here, old_time
                for key,value in old_modules.items():
                    if value is None:
                        sys.modules.pop(key,None)
                    else:
                        sys.modules[key]=value
            return presses, records, error

    def test_both_lanes_same_bootstrap_order_and_cadence(self):
        native = self.run_lane(True,True)
        candidate = self.run_lane(False,True)
        self.assertIsNone(native[2])
        self.assertIsNone(candidate[2])
        self.assertEqual(native[0], [(0,100.0),(1,106.0)])
        self.assertEqual(native[0],candidate[0])
        self.assertTrue(all(row['queued'] and row['done'] for row in native[1]+candidate[1]))

    def test_both_lanes_same_eleven_plus_four_and_repeated_kirby(self):
        native = self.run_lane(True,False)
        candidate = self.run_lane(False,False)
        self.assertIsNone(native[2])
        self.assertIsNone(candidate[2])
        self.assertEqual(native[0],candidate[0])
        self.assertEqual([r for r,t in native[0]], [0]*11+[1]*4)
        self.assertEqual([r['expected_target'] for r in native[1]][2:11:8], [6,6])
        self.assertEqual([t for r,t in native[0]], [100.0+6*i for i in range(15)])

    def test_invalid_strict_receipts_fail_without_next_input(self):
        for native in (True,False):
            for fault in ('wrong_ticket','wrong_role','wrong_target','unreleased','refused'):
                with self.subTest(native=native,fault=fault):
                    presses,records,error = self.run_lane(native,False,fault)
                    self.assertIsNotNone(error)
                    self.assertEqual(len(presses),1)
                    self.assertEqual(len(records),1)
                    self.assertIsNone(records[0]['done'])

    def test_invalid_start_or_effective_config_sends_no_input(self):
        for native in (True,False):
            for fault in ('wrong_config','wrong_start','missing_qualification'):
                with self.subTest(native=native,fault=fault):
                    presses,records,error = self.run_lane(native,True,fault)
                    self.assertIsNotNone(error)
                    self.assertEqual(presses,[])
                    self.assertEqual(records,[])

    def test_candidate_bootstrap_still_requires_body_bank(self):
        presses,records,error = self.run_lane(False,True,'missing_bank')
        self.assertIsNotNone(error)
        self.assertEqual(len(presses),1)
        self.assertIn('body bank',records[0]['refused'])

    def test_original_39_input_was_before_qualification(self):
        self.assertIsNone(rows.qualified_boot(QUALIFICATION[:6]))
        self.assertFalse(any(rows.cycle_request(line, 0) for line in QUALIFICATION[:6]))
        self.assertIsNone(rows.qualified_boot(QUALIFICATION[:8]))
        result = rows.qualified_boot(QUALIFICATION)
        self.assertEqual(result['boot_to_qualification_ms'], 259116)
        self.assertEqual(result['first_physical_observation_to_qualification_ms'], 3950)

    def test_old_boot_or_new_catalog_cannot_supply_qualification(self):
        self.assertIsNone(rows.qualified_boot(QUALIFICATION + QUALIFICATION[:6]))
        self.assertIsNone(rows.qualified_boot(QUALIFICATION[1:]))
        self.assertIsNone(rows.qualified_boot(QUALIFICATION + [
            '591125300 #252 [MiiAutoInit] catalog read queued generations=[3, 3] opening=3']))

    def test_both_lanes_wait_identically_without_extra_inputs(self):
        for native in (True, False):
            presses, records, error = self.run_lane(native, True, 'late_qualification')
            self.assertIsNone(error)
            self.assertEqual(presses, [(0,101.0),(1,107.0)])
            self.assertEqual(len(records), 2)

if __name__ == '__main__':
    unittest.main()
