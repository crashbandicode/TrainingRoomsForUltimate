"""Contract probes against the retained native lr07 log, including false matches."""
import json
from pathlib import Path
import re
import unittest

HERE = Path(__file__).resolve().parent
GATES = json.loads((HERE / 'rows.json').read_text())['gates']
LOG = (HERE.parent / 'lr07-shared/practice-diagnostics.log').read_text()


def matches(name, text):
    return re.search(GATES[name]['pattern'], text) is not None


class RetirementTests(unittest.TestCase):
    def test_observed_native_path(self):
        for name in ('lr08-cpu-synchronous', 'lr08-player-ledger-empty',
                     'lr08-cpu-native-retired', 'lr08-source-retired'):
            self.assertTrue(matches(name, LOG), name)

    def test_cpu_requires_every_retirement_link(self):
        for marker in ('[GroupSwap] pad route released owner=1',
                       '[MiiSavedAlpha] destroyed owner=1',
                       '[CharacterRoster] invalidated observed entry before destruction owner=1',
                       '[TrioBoundary] destroy entry=0x120ae18000 owner=1'):
            shortened = '\n'.join(line for line in LOG.splitlines() if marker not in line)
            self.assertFalse(matches('lr08-cpu-native-retired', shortened), marker)

    def test_cpu_rejects_bad_retirement_and_entry_mismatch(self):
        line = next(line for line in LOG.splitlines() if '[Destroy] plan owner=1 reason=finalize steps=1' in line)
        for old, new in (('skipped=0', 'skipped=1'), ('routes_stale=0', 'routes_stale=1')):
            self.assertFalse(matches('lr08-cpu-native-retired', LOG.replace(line, line.replace(old, new))))
        bad = LOG.replace('destroyed owner=1 entry=0x120ae18000', 'destroyed owner=1 entry=0xdeadbeef')
        self.assertFalse(matches('lr08-cpu-native-retired', bad))

    def test_source_requires_listener_and_same_owner_release(self):
        for old, new in (('listener_removed=Some(true)', 'listener_removed=Some(false)'),
                         ('claim owner=0x11ea39b200 released=true', 'claim owner=0xdeadbeef released=true')):
            self.assertFalse(matches('lr08-source-retired', LOG.replace(old, new)))

    def test_no_prior_row_or_cpu_bank_substitution(self):
        self.assertFalse(matches('lr08-cpu-native-retired', ''))
        self.assertFalse(matches('lr08-cpu-synchronous', LOG.replace('bank_build=false', 'bank_build=true')))
        self.assertFalse(matches('lr08-player-ledger-empty', LOG.replace('residual=[]', 'residual=[1]')))


if __name__ == '__main__':
    unittest.main()
