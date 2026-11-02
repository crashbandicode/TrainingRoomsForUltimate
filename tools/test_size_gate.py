"""Host tests of tools/sloc.py and tools/size_gate.py on synthetic trees (no git: the directory walk)."""
import io
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import sloc  # noqa: E402
import size_gate  # noqa: E402

SOURCE = '''// a comment
/// a doc comment
fn a() {
    let s = "// not a comment";   /* inline */

    let t = 1;
}
/* block
   comment */
#[cfg(test)]
mod tests {
    #[test]
    fn t() {}
}
#[cfg(test)]
mod more;
'''
# Lines that count: fn a() {, let s = ..., let t = 1;, }  -> 4.

ENTRY = '[[budget]]\nscope = "{scope}"\nceiling = {ceiling}\ndate = "{date}"\nreason = "{reason}"\n{extra}\n'


def budget(*entries):
    return ''.join(ENTRY.format(**{'date': '2026-11-02', 'reason': 'r', 'extra': '', **e}) for e in entries)


def approved(scope, ceiling, **more):
    return {'scope': scope, 'ceiling': ceiling, 'extra': 'approved_by = "lead"', **more}


class Counting(unittest.TestCase):
    def test_rules_of_the_operator_counter(self):
        self.assertEqual(sloc.count(SOURCE), 4)

    def test_tests_and_vendored_code_are_not_counted(self):
        with tempfile.TemporaryDirectory() as root:
            for rel in ('src/a.rs', 'tests/t.rs', 'third_party/sdk/src/lib.rs', 'crates/c/src/lib.rs',
                        'crates/c/tests/x.rs', 'target/debug/build/gen.rs'):
                path = pathlib.Path(root, rel)
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(SOURCE)
            ours, vendored = sloc.totals(sloc.items_tree(root), 0)
            self.assertEqual(dict(ours), {'src/a.rs': 4, 'crates/c/src/lib.rs': 4})
            self.assertEqual(vendored, 4)


class Budget(unittest.TestCase):
    def check(self, text):
        import tomllib
        return size_gate.check_budget(tomllib.loads(text)['budget'])

    def test_first_ceiling_and_raises_need_approval_and_cuts_do_not(self):
        ceilings, errors = self.check(budget(approved('total', 100), {'scope': 'total', 'ceiling': 90}))
        self.assertEqual((ceilings, errors), ({'total': 90}, []))
        _, errors = self.check(budget({'scope': 'total', 'ceiling': 100}))
        self.assertIn('the first ceiling needs approved_by', errors[0])
        _, errors = self.check(budget(approved('total', 100), {'scope': 'total', 'ceiling': 101}))
        self.assertIn('a raise from 100 to 101 needs approved_by', errors[0])
        _, errors = self.check(budget(approved('total', 100), approved('total', 120)))
        self.assertEqual(errors, [])

    def test_reason_date_scope_and_total_are_required(self):
        _, errors = self.check(budget(approved('total', 100, reason=' ')))
        self.assertIn('reason is required', errors[0])
        _, errors = self.check(budget(approved('total', 100, date='2026-11-02'),
                                      {'scope': 'total', 'ceiling': 90, 'date': '2026-11-01'}))
        self.assertIn('before the previous entry', errors[0])
        _, errors = self.check(budget(approved('src/', 100)))
        self.assertIn('trailing slash', errors[0])
        _, errors = self.check(budget(approved('src/model', 100)))
        self.assertEqual(errors, ['no "total" ceiling'])


class Gate(unittest.TestCase):
    def gate(self, text):
        with tempfile.TemporaryDirectory() as root:
            for rel in ('src/model/a.rs', 'src/model/b.rs', 'src/runtime/c.rs'):
                path = pathlib.Path(root, rel)
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(SOURCE)
            pathlib.Path(root, 'size-budget.toml').write_text(text)
            out = io.StringIO()
            return size_gate.run(root, pathlib.Path(root, 'size-budget.toml'), out=out), out.getvalue()

    def test_within_over_and_invalid(self):
        status, text = self.gate(budget(approved('total', 12), approved('src/model', 8)))
        self.assertEqual(status, 0, text)
        self.assertIn('src/model', text)
        status, text = self.gate(budget(approved('total', 11)))
        self.assertEqual(status, 1, text)
        self.assertIn('over budget in total', text)
        status, text = self.gate(budget(approved('total', 12), approved('src/model', 7)))
        self.assertEqual(status, 1, text)
        self.assertIn('over budget in src/model', text)
        status, text = self.gate(budget(approved('total', 12), {'scope': 'total', 'ceiling': 13}))
        self.assertEqual(status, 2, text)


if __name__ == '__main__':
    unittest.main()
