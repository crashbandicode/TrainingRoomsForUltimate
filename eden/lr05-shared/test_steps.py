"""Offline contract probes for observed CSS endpoint convergence (no Eden)."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('lr05_steps', Path(__file__).with_name('steps.py'))
steps = importlib.util.module_from_spec(spec)
spec.loader.exec_module(steps)


class Context:
    def __init__(self, edge, fail_press=False):
        self.edge, self.fail_press, self.actions = edge, fail_press, []

    def gate(self, name):
        self.actions.append(('gate', name))

    def play(self, name):
        self.actions.append(('play', name))

    def wait_log(self, source, pattern, cap):
        self.actions.append(('wait', pattern))
        return f'step screen=css dir=right from={self.edge[0]} to={self.edge[1]} token=p2'

    def log(self, value):
        self.actions.append(('log', value))

    def shot(self, name):
        self.actions.append(('shot', name))

    def call(self, method, params):
        self.actions.append(('call', method, params))

    def press(self, button, ms):
        self.actions.append(('press', button))
        if self.fail_press:
            raise RuntimeError('injected input failure')


class ConvergenceTests(unittest.TestCase):
    def test_roy_requires_down_then_left_before_placement(self):
        ctx = Context((26, 27))
        result = steps.return_cpu_to_lucas(ctx)
        self.assertEqual([a[1] for a in ctx.actions if a[0] == 'press'], ['down', 'left'])
        self.assertEqual(result['final_tile'], 39)
        self.assertLess(ctx.actions.index(('gate', 'lucas')), ctx.actions.index(('play', 'lucas-pick')))
        self.assertEqual(ctx.actions[-2], ('gate', 'ready'))

    def test_sonic_does_not_take_roy_down_step(self):
        ctx = Context((39, 40))
        steps.return_cpu_to_lucas(ctx)
        self.assertEqual([a[1] for a in ctx.actions if a[0] == 'press'], ['left'])
        self.assertIn(('gate', 'return-sonic-held'), ctx.actions)

    def test_crossed_endpoint_pair_is_rejected_without_correction(self):
        ctx = Context((26, 40))
        with self.assertRaisesRegex(RuntimeError, 'unqualified'):
            steps.return_cpu_to_lucas(ctx)
        self.assertFalse(any(a[0] == 'press' for a in ctx.actions))
        self.assertNotIn(('play', 'lucas-pick'), ctx.actions)

    def test_input_error_releases_l3_and_never_places(self):
        ctx = Context((39, 40), fail_press=True)
        with self.assertRaisesRegex(RuntimeError, 'injected'):
            steps.return_cpu_to_lucas(ctx)
        self.assertEqual(ctx.actions[-1], ('call', 'input.button', {'button': 'lstick', 'state': 'release'}))
        self.assertNotIn(('play', 'lucas-pick'), ctx.actions)

    def test_missing_pickup_predicate_does_not_guess(self):
        ctx = Context((39, 40))
        def missing(*args, **kwargs):
            raise RuntimeError('no fresh token')
        ctx.wait_log = missing
        with self.assertRaisesRegex(RuntimeError, 'no fresh token'):
            steps.return_cpu_to_lucas(ctx)
        self.assertFalse(any(a[0] == 'press' for a in ctx.actions))


if __name__ == '__main__':
    unittest.main()
