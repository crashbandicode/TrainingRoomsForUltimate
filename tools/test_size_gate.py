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

ENTRY = '[[budget]]\nbasis = "rustfmt-sloc4"\nscope = "{scope}"\nceiling = {ceiling}\ndate = "{date}"\nreason = "{reason}"\n{extra}\n'


def budget(*entries):
    return ''.join(ENTRY.format(**{'date': '2026-11-02', 'reason': 'r', 'extra': '', **e}) for e in entries)


def approved(scope, ceiling, **more):
    return {'scope': scope, 'ceiling': ceiling, 'extra': 'approved_by = "lead"', **more}


class Counting(unittest.TestCase):
    def test_rules_of_the_counter(self):
        impl, sim, tests, vendored = sloc.measure({'src/a.rs': SOURCE}, 0)
        self.assertEqual((dict(impl), sim, vendored), ({'src/a.rs': 4}, 0, 0))
        self.assertGreater(tests, 0)

    def test_tests_and_vendored_code_are_not_counted(self):
        with tempfile.TemporaryDirectory() as root:
            for rel in ('src/a.rs', 'tests/t.rs', 'third_party/sdk/src/lib.rs', 'crates/c/src/lib.rs',
                        'crates/c/tests/x.rs', 'target/debug/build/gen.rs'):
                path = pathlib.Path(root, rel)
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(SOURCE)
            impl, _, _, vendored = sloc.measure(sloc.files_tree(root), 0)
            self.assertEqual(dict(impl), {'src/a.rs': 4, 'crates/c/src/lib.rs': 4})
            self.assertEqual(vendored, 4)

    def test_test_modules_and_simulator_code_are_set_apart(self):
        lib = ('fn a() {}\n#[cfg(test)]\nmod tests;\n#[cfg(test)]\n#[path = "extra_tests.rs"]\nmod extra;\n'
               '#[cfg(all(test, feature = "x"))]\nmod inline {\n    fn t() {}\n}\n'
               '#[cfg(any(test, feature = "simulator"))]\nmod sim;\nmod live;\n')
        files = {'src/lib.rs': lib, 'src/tests.rs': 'fn t() {}\n', 'src/extra_tests.rs': 'fn u() {}\n',
                 'src/sim.rs': 'fn s() {}\n', 'src/live.rs': 'fn v() {}\n'}
        impl, sim, tests, _ = sloc.measure(files, 0)
        self.assertEqual(dict(impl), {'src/lib.rs': 2, 'src/live.rs': 1})
        self.assertEqual(sim, 3)  # the gated declaration (2 lines) and sim.rs
        self.assertGreaterEqual(tests, 2 + 4)


    def test_layout_normalization_and_script_exclusion(self):
        compact = {"src/a.rs": "fn f(){let x=1;let y=2;consume(x,y);}"}
        expanded = {"src/a.rs": "fn f() {\n    let x = 1;\n    let y = 2;\n    consume(x, y);\n}\n"}
        expected = sloc.measure_normalized(expanded, 0)
        self.assertEqual(sloc.measure_normalized(compact, 0), expected)
        self.assertEqual(expected[0]["src/a.rs"], 5)
        compact["tools/script.rs"] = "fn script() {}"
        self.assertEqual(sloc.measure_normalized(compact, 0), expected)

    def test_invalid_rust_cannot_receive_a_count(self):
        with self.assertRaisesRegex(ValueError, "rustfmt failed"):
            sloc.measure_normalized({"src/bad.rs": "fn {"}, 0)


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


    def test_unit_migration_preserves_raw_history_and_needs_approval(self):
        entries = [
            dict(scope="total", ceiling=100, date="2026-11-02", reason="old", approved_by="lead"),
            dict(scope="total", ceiling=130, date="2026-11-02", reason="normalized",
                 basis=sloc.NORMALIZED_BASIS, approved_by="operator"),
            dict(scope="total", ceiling=120, date="2026-11-02", reason="cut",
                 basis=sloc.NORMALIZED_BASIS),
        ]
        self.assertEqual(size_gate.check_budget(entries), ({"total": 120}, []))
        self.assertEqual(size_gate.check_budget(entries, "raw-sloc4"), ({"total": 100}, []))
        del entries[1]["approved_by"]
        self.assertIn("the first ceiling needs approved_by", size_gate.check_budget(entries)[1][0])


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
