import importlib.util,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('timeline',Path(__file__).with_name('teardown_timeline.py'))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class TimelineTests(unittest.TestCase):
    def test_earliest_finalize_not_later_group_teardown(self):
        lines=['100 #1 [GroupSwap] bodies queued owner=0 group=0',
               '140 #2 [Destroy] plan owner=1 reason=finalize steps=0',
               '150 #3 [GroupSwap] built owner=0 group=0',
               '160 #4 [Destroy] plan owner=0 reason=finalize steps=0',
               '200 #5 [GroupSwap] teardown owner=0 reason=finalize']
        result=m.timeline(lines)['queue_relative_ms']
        self.assertEqual((result['built'],result['earliest_scene_end_observed'],result['group_teardown']),(50,60,100))
    def test_absence_stays_unknown(self):
        result=m.timeline(['90 #1 pause_owner begin','100 #2 [GroupSwap] bodies queued owner=0'])['queue_relative_ms']
        self.assertEqual(result['pause'],-10)
        self.assertIsNone(result['earliest_scene_end_observed'])
if __name__=='__main__':unittest.main()
