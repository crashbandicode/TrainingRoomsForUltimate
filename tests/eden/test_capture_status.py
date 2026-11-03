"""Regression for ui-16's false Off-to-On arming failure."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "capture_status", Path(__file__).resolve().parents[2] / "eden/capture_status.py")
capture_status = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture_status)


class CaptureStatusTests(unittest.TestCase):
    def status(self, fields):
        return "387982810 #1114 [NamedMacroDiag] status " + fields + " recorder=Idle"

    def test_ready_idle_on_is_enabled_without_arming_a_record(self):
        line = self.status("capture_ready=true capture_desired=true capture_phase=Idle capture_armed=false")
        self.assertTrue(capture_status.idle_preference(line, True))
        self.assertFalse(capture_status.idle_preference(line, False))

    def test_off_seed_is_not_proven_by_unarmed_alone(self):
        line = self.status("capture_ready=true capture_desired=false capture_phase=Idle capture_armed=false")
        self.assertTrue(capture_status.idle_preference(line, False))
        self.assertFalse(capture_status.idle_preference(line, True))
        old = self.status("capture_ready=true capture_armed=false")
        self.assertFalse(capture_status.idle_preference(old, False))
        self.assertFalse(capture_status.idle_preference(old, True))

    def test_pending_choice_during_unarmed_record_is_not_idle_acceptance(self):
        line = self.status("capture_ready=true capture_desired=true capture_phase=Unarmed capture_armed=false")
        self.assertFalse(capture_status.idle_preference(line, True))

    def test_active_record_does_not_substitute_for_idle_preference(self):
        for desired in ("true", "false"):
            line = self.status(f"capture_ready=true capture_desired={desired} capture_phase=Armed capture_armed=true")
            self.assertFalse(capture_status.idle_preference(line, True))

    def test_unavailable_and_incoherent_receipts_fail(self):
        for fields in (
            "capture_ready=false capture_desired=true capture_phase=Idle capture_armed=false",
            "capture_ready=true capture_desired=true capture_phase=Idle capture_armed=true",
            "capture_ready=true capture_desired=false capture_desired=true capture_phase=Idle capture_armed=false",
        ):
            self.assertFalse(capture_status.idle_preference(self.status(fields), True))

    def test_missing_and_non_status_receipts_fail(self):
        for line in (None, "", "[ReplayCapture] menu preference enabled=true result=Applied",
                     "capture_ready=true capture_desired=true capture_phase=Idle capture_armed=false"):
            self.assertFalse(capture_status.idle_preference(line, True))


if __name__ == "__main__":
    unittest.main()
