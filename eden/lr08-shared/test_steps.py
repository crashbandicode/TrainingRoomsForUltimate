"""Offline failure-boundary probes for the native-quit project hook."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('steps', Path(__file__).with_name('steps.py'))
steps = importlib.util.module_from_spec(spec)
spec.loader.exec_module(steps)


class Context:
    def __init__(self, fail=None):
        self.fail, self.events = fail, []

    def gate(self, name):
        self.events.append(('gate', name))
        if name == self.fail:
            raise RuntimeError('gate refused: ' + name)

    def press(self, name, ms):
        self.events.append(('press', name, ms))

    def play(self, name):
        self.events.append(('play', name))

    def shot(self, name):
        self.events.append(('shot', name))


class QuitTests(unittest.TestCase):
    def test_bad_start_never_sends_input(self):
        ctx = Context('training-hud-unpaused')
        with self.assertRaises(RuntimeError):
            steps.quit_warm_training(ctx)
        self.assertFalse(any(e[0] in ('press', 'play') for e in ctx.events))

    def test_unverified_native_pause_never_plays_quit(self):
        ctx = Context('native-pause')
        with self.assertRaises(RuntimeError):
            steps.quit_warm_training(ctx)
        self.assertFalse(any(e[0] == 'play' for e in ctx.events))

    def test_unclean_finalize_never_accepts_css(self):
        ctx = Context('lr08-cpu-native-retired')
        with self.assertRaises(RuntimeError):
            steps.quit_warm_training(ctx)
        self.assertNotIn(('gate', 'ready'), ctx.events)
        self.assertFalse(any(e[0] == 'shot' for e in ctx.events))

    def test_pause_before_quit_and_finalization_before_css(self):
        ctx = Context()
        result = steps.quit_warm_training(ctx)
        self.assertLess(ctx.events.index(('gate', 'native-pause')), ctx.events.index(('play', 'focus-native-quit-frames')))
        self.assertLess(ctx.events.index(('gate', 'quit-focus')), ctx.events.index(('play', 'confirm-native-quit-frames')))
        self.assertLess(ctx.events.index(('gate', 'lr08-cpu-native-retired')), ctx.events.index(('gate', 'ready')))
        self.assertEqual(result['end'], 'css-return')
        self.assertFalse(any('training-closed' in e for e in ctx.events))


if __name__ == '__main__':
    unittest.main()
