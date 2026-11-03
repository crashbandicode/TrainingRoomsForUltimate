"""Negative evidence must survive aggregation: losses, stale rows, bad reads."""
import importlib.util
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location('analysis',Path(__file__).with_name('analyze_trace.py'))
analysis=importlib.util.module_from_spec(spec);spec.loader.exec_module(analysis)


class AnalysisTests(unittest.TestCase):
    def fixture(self):
        records=[{'event':event,'sequence':str(i),'point_id':'9','thread_id':'80','process_id':'81',
                  'wall_time_ns':'1500000000','captures':[
                      {'expression':k,'value':v,'error':''} for k,v in
                      {'x0+0xe0':'1','x1+0x18':'74','x2':'1'}.items()]}
                 for i,event in enumerate(['entry','return'],1)]
        counters={'boot_error':'','output_error':'','file_lost':'0','points':[
            {'id':'9','target':'main+0x653240','recorded':'2','hits':'1','matched':'1',
             'dropped':'0','abandoned_returns':'0','return_overflow':'0'}]}
        rows=[{'id':'cycle1-warm-groups','started':1,'ended':2}]
        return records,counters,rows,{'main+0x653240':'9'}

    def test_return_read_errors_do_not_rewrite_successful_entry(self):
        args=self.fixture();args[0][1]['captures'][0]['error']='memory_unavailable'
        result=analysis.analyze(*args)
        self.assertTrue(result['stream_reconciliation_pass'])
        self.assertEqual(result['capture_errors_by_event'],{'return':1})

    def test_entry_failure_stale_row_and_missing_return_each_fail(self):
        for mutation in ('bad_read','stale','missing_return'):
            args=self.fixture()
            if mutation=='bad_read':args[0][0]['captures'][0]['error']='memory_unavailable'
            elif mutation=='stale':args[2][0]['started']=1.8
            else:args[0].pop()
            self.assertFalse(analysis.analyze(*args)['stream_reconciliation_pass'],mutation)

    def test_counter_loss_and_sequence_gap_fail(self):
        for mutation in ('loss','sequence'):
            args=self.fixture()
            if mutation=='loss':args[1]['file_lost']='1'
            else:args[0][1]['sequence']='3'
            self.assertFalse(analysis.analyze(*args)['stream_reconciliation_pass'],mutation)


if __name__ == '__main__':unittest.main()
