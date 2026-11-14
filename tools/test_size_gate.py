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
        impl, sim, tests, vendored = sloc.measure({'src/a.rs': SOURCE}, 0)[:4]
        self.assertEqual((dict(impl), sim, vendored), ({'src/a.rs': 4}, 0, 0))
        self.assertGreater(tests, 0)

    def test_tests_and_vendored_code_are_not_counted(self):
        with tempfile.TemporaryDirectory() as root:
            for rel in ('src/a.rs', 'tests/t.rs', 'third_party/sdk/src/lib.rs', 'crates/c/src/lib.rs',
                        'crates/c/tests/x.rs', 'target/debug/build/gen.rs'):
                path = pathlib.Path(root, rel)
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(SOURCE)
            impl, _, _, vendored = sloc.measure(sloc.files_tree(root), 0)[:4]
            self.assertEqual(dict(impl), {'src/a.rs': 4, 'crates/c/src/lib.rs': 4})
            self.assertEqual(vendored, 4)

    def test_test_modules_and_simulator_code_are_set_apart(self):
        lib = ('fn a() {}\n#[cfg(test)]\nmod tests;\n#[cfg(test)]\n#[path = "extra_tests.rs"]\nmod extra;\n'
               '#[cfg(all(test, feature = "x"))]\nmod inline {\n    fn t() {}\n}\n'
               '#[cfg(any(test, feature = "simulator"))]\nmod sim;\nmod live;\n')
        files = {'src/lib.rs': lib, 'src/tests.rs': 'fn t() {}\n', 'src/extra_tests.rs': 'fn u() {}\n',
                 'src/sim.rs': 'fn s() {}\n', 'src/live.rs': 'fn v() {}\n'}
        impl, sim, tests, _ = sloc.measure(files, 0)[:4]
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


def counted(files):
    """Implementation lines per file (index 0 of measure, so the old counter runs these tests too)."""
    return dict(sloc.measure(files, 0)[0])


class DiagOnly(unittest.TestCase):
    """Operator 2026-11-14: diagnostic code that won't ship is not counted (code compiled only with `diag` or a
    feature that turns it on). Test and simulator code keep sloc4's rules."""

    def test_a_diag_only_fn_is_not_counted(self):
        text = 'fn a() {}\n#[cfg(feature = "diag")]\nfn d() {\n    x();\n}\nfn b() {}\n'
        self.assertEqual(counted({'src/a.rs': text}), {'src/a.rs': 2})
        self.assertEqual(sloc.measure({'src/a.rs': text}, 0)[4], 4)

    def test_a_diag_only_module_file_and_its_children_are_not_counted(self):
        files = {'src/lib.rs': 'mod live;\n#[cfg(feature = "diag")]\npub(crate) mod probe;\n',
                 'src/live.rs': 'fn v() {}\n',
                 'src/probe.rs': 'mod inner;\nfn p() {\n    q();\n}\n',
                 'src/probe/inner.rs': 'fn i() {}\n'}
        self.assertEqual(counted(files), {'src/lib.rs': 1, 'src/live.rs': 1})
        self.assertEqual(sloc.measure(files, 0)[4], 2 + 4 + 1)

    def test_a_module_also_declared_for_the_release_build_is_counted(self):
        files = {'src/lib.rs': '#[cfg(feature = "diag")]\n#[path = "shared.rs"]\nmod a;\n#[path = "shared.rs"]\nmod b;\n',
                 'src/shared.rs': 'fn s() {}\n'}
        self.assertEqual(counted(files), {'src/lib.rs': 2, 'src/shared.rs': 1})

    def test_any_test_or_diag_is_not_counted(self):
        text = ('fn a() {}\n#[cfg(any(test, feature = "diag"))]\nfn d() {}\n'
                '#[cfg(any(feature = "diag", test))]\nmod m {\n    fn e() {}\n}\n')
        self.assertEqual(counted({'src/a.rs': text}), {'src/a.rs': 1})

    def test_not_diag_is_counted(self):
        text = '#[cfg(not(feature = "diag"))]\nfn r() {}\n#[cfg(not(any(test, feature = "diag")))]\nfn s() {}\n'
        self.assertEqual(counted({'src/a.rs': text}), {'src/a.rs': 4})

    def test_cfg_macro_and_cfg_attr_are_counted(self):
        text = ('fn a() {\n    if cfg!(feature = "diag") {\n        x();\n    }\n}\n'
                '#[cfg_attr(feature = "diag", derive(Debug))]\nstruct S;\n')
        self.assertEqual(counted({'src/a.rs': text}), {'src/a.rs': 7})

    def test_nested_items(self):
        text = ('mod m {\n    fn a() {}\n    #[cfg(feature = "diag")]\n    fn d() {\n        y();\n    }\n'
                '    fn b() {}\n}\n')
        self.assertEqual(counted({'src/a.rs': text}), {'src/a.rs': 4})

    def test_a_diag_only_impl_block(self):
        text = ('struct S;\n#[cfg(feature = "diag")]\nimpl S {\n    fn d(&self) -> [u8; 2] {\n        [0; 2]\n    }\n}\n'
                'impl S {\n    fn r(&self) {}\n}\n')
        self.assertEqual(counted({'src/a.rs': text}), {'src/a.rs': 4})

    def test_statements_in_a_fn_body(self):
        text = ('fn a() {\n    let x = 1;\n    #[cfg(feature = "diag")]\n    log(x);\n'
                '    #[cfg(feature = "diag")]\n    {\n        trace();\n    }\n'
                '    #[cfg(feature = "diag")]\n    if x > 0 {\n        t();\n    } else {\n        u();\n    }\n'
                '    #[cfg(feature = "diag")]\n    let t = match x {\n        1 => 2,\n        _ => 3,\n    };\n'
                '    use_it(x);\n}\n')
        self.assertEqual(counted({'src/a.rs': text}), {'src/a.rs': 4})

    def test_fields_variants_arms_and_parameters(self):
        text = ('struct S {\n    a: u8,\n    #[cfg(feature = "diag")]\n    trace: Trace,\n}\n'
                'enum E {\n    A,\n    #[cfg(feature = "diag")]\n    Probe(u8),\n}\n'
                'fn f(e: E, #[cfg(feature = "diag")] t: u8) -> u8 {\n    match e {\n        E::A => 1,\n'
                '        #[cfg(feature = "diag")]\n        E::Probe(n) => {\n            n\n        }\n    }\n}\n'
                'fn g() -> S {\n    S {\n        a: 1,\n        #[cfg(feature = "diag")]\n        trace: Trace::new(),\n    }\n}\n')
        # 16: the parameter goes mid-line, so `fn f(e: E, ) -> u8 {` still counts.
        self.assertEqual(counted({'src/a.rs': text}), {'src/a.rs': 16})

    def test_macro_bodies_stay_counted(self):
        text = 'thread_local! {\n    #[cfg(feature = "diag")]\n    static X: u8 = 0;\n}\n'
        self.assertEqual(counted({'src/a.rs': text}), {'src/a.rs': 4})

    def test_an_inner_cfg_drops_the_whole_file(self):
        files = {'src/lib.rs': 'mod a;\n', 'src/a.rs': '#![cfg(feature = "diag")]\nfn a() {}\nfn b() {}\n'}
        self.assertEqual(counted(files), {'src/lib.rs': 1})

    def test_features_that_turn_diag_on_come_from_the_manifests(self):
        manifest = ('[package]\nname = "p"\n[features]\ndefault = ["sdk"]\nsdk = []\ndiag = []\n'
                    'trace = ["diag", "lock/owners"]\n[dependencies]\nlock = { path = "crates/lock" }\n')
        files = {'Cargo.toml': manifest,
                 'crates/lock/Cargo.toml': '[package]\nname = "lock"\n[features]\nowners = []\nfast = []\n',
                 'src/lib.rs': ('#[cfg(feature = "trace")]\nfn t() {}\n#[cfg(feature = "sdk")]\nfn s() {}\n'
                                '#[cfg(not(feature = "sdk"))]\nfn n() {}\n'),
                 'crates/lock/src/lib.rs': ('mod owners;\n#[cfg(feature = "owners")]\nfn o() {}\n'
                                            '#[cfg(feature = "fast")]\nfn f() {}\n'),
                 'crates/lock/src/owners.rs': '#![cfg(feature = "owners")]\nfn table() {}\n'}
        self.assertEqual(counted(files), {'src/lib.rs': 4, 'crates/lock/src/lib.rs': 3})

    def test_default_features_that_turn_diag_on_are_refused(self):
        with self.assertRaisesRegex(ValueError, 'default features turn on diag'):
            sloc.Release({'Cargo.toml': '[features]\ndefault = ["diag"]\ndiag = []\n'})

    def test_host_only_code_and_unknown_cfgs_stay_counted(self):
        text = ('#[cfg(not(target_os = "switch"))]\nfn host() {}\n#[cfg(target_os = "switch")]\nfn console() {}\n'
                '#[cfg(not(feature = "deluxe-sdk"))]\nfn fallback() {}\n#[cfg($cfg)]\nfn odd() {}\n')
        self.assertEqual(counted({'src/a.rs': text}), {'src/a.rs': 8})

    def test_test_items_keep_sloc4s_rules_exactly(self):
        # sloc4 ends a gated item at its first ';' or '{'; the rest of this test const is counted, as before (the
        # primary 2026-11-14: changing that is a separate decision). Only diag-only code changed.
        text = '#[cfg(test)]\nconst X: [u8; 2] = [1, 2];\n#[cfg(test)]\nuse a::{b, c};\nfn a() {}\n'
        self.assertEqual(counted({'src/a.rs': text}), {'src/a.rs': 3})

    def test_a_diag_item_inside_a_test_item_is_left_to_the_test_rule(self):
        text = '#[cfg(test)]\nmod tests {\n    #[cfg(feature = "diag")]\n    fn d() {}\n}\nfn a() {}\n'
        m = sloc.measure({'src/a.rs': text}, 0)
        self.assertEqual((dict(m[0]), m[4]), ({'src/a.rs': 1}, 0))

    def test_the_predicate_is_parsed_and_evaluated_in_three_values(self):
        def pred(text):
            return sloc.parse_cfg(sloc.token_tree(sloc.tokenize(text)).children)
        diag_off = sloc.knowledge({'diag'})
        self.assertIs(sloc.evaluate(pred('all(test, feature = "x")'), diag_off), False)
        self.assertIsNone(sloc.evaluate(pred('any(test, feature = "x")'), diag_off))
        self.assertIs(sloc.evaluate(pred('any(test, feature = "diag")'), diag_off), False)
        self.assertIs(sloc.evaluate(pred('not(any(test, feature = "diag"))'), diag_off), True)
        self.assertIsNone(sloc.evaluate(pred('not(target_os = "switch")'), diag_off))
        self.assertIs(sloc.evaluate(pred('all()'), diag_off), True)
        self.assertIs(sloc.evaluate(pred('any()'), diag_off), False)
        for bad in ('a, b', 'not(a, b)', 'feature = 3', '"x"'):
            with self.assertRaises(sloc.CfgSyntax):
                pred(bad)

    def test_a_dropped_item_must_own_whole_lines(self):
        text = 'fn a() {\n    #[cfg(feature = "diag")]\n    log();\n    keep();\n}\n'
        start = text.index('#[cfg')
        end = text.index('keep();') + len('keep();')
        with self.assertRaisesRegex(ValueError, 'runs into the next element'):
            sloc.layout_check('src/a.rs', '', [(start, end, 'diag', 'stmts')], text)
        with self.assertRaisesRegex(ValueError, 'does not own whole lines'):
            sloc.layout_check('src/a.rs', '', [(start, text.index('log') + 3, 'diag', 'stmts')], text)
        sloc.layout_check('src/a.rs', '', [(start, text.index('log();') + 6, 'diag', 'stmts')], text)

    def test_the_rest_must_still_parse(self):
        command = sloc._rustfmt()
        self.assertTrue(sloc.parses(command, 'fn a() {\n    \n}\n/// doc\n'))
        self.assertFalse(sloc.parses(command, 'fn a() {\n    log(\n}\n'))

    def test_normalized_count_of_diag_code(self):
        text = ('fn a(){let x=1;\n#[cfg(feature="diag")]\nlog(x);use_it(x);}\n'
                '#[cfg(feature = "diag")] fn d() { x(); }\n')
        m = sloc.measure_normalized({'src/a.rs': text}, 0)
        self.assertEqual((dict(m[0]), m[4]), ({'src/a.rs': 4}, 2 + 4))


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
