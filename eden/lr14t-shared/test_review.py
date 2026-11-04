import copy,importlib.util,json,tempfile,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('review',Path(__file__).with_name('review.py'))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class ReviewTests(unittest.TestCase):
    def fixture(self,root,wrong_thread=False):
        rows=[{'point_id':str(i),'event':'return' if i==5 else 'entry','sequence':str(n),'thread_id':'81' if wrong_thread and i==4 else '80','frame':'7','registers':{'x0':'0x1','x8':'0x0'}} for i,n in [(5,15),(3,16),(4,17)]]
        files={'out/RC8-unmapped-trace/unmapped-trace.json':{'trace':rows,'predicate':{'id':'5'}},'out/caller-points.json':{'elf_sha256':'fixture','points':{'load':{'id':'3'},'after':{'id':'4'}}},'state/lifecycle.json':{'cleanup_failures':[],'released':True},'state/yoga-restore.json':{},'state/yoga-verify.json':{}}
        for path,value in files.items():
            dest=root/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(json.dumps(value))
        (root/'state/run-yoga').mkdir();(root/'state/run-yoga/console.log').write_text('HW.Memory Unmapped Read64 @ 0x21\n')
        (root/'unit-journal.log').write_text('code=dumped status=11/SEGV\n')
    def test_does_not_hide_memory_or_crash(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);self.fixture(root);result=m.summarize(root)
            self.assertEqual(result['observed']['effective_address'],'0x21')
            self.assertEqual(len(result['memory_lines']),1);self.assertEqual(len(result['fatal_journal']),1)
    def test_cross_thread_is_not_a_bound_load(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);self.fixture(root,True)
            with self.assertRaises(ValueError):m.summarize(root)
if __name__=='__main__':unittest.main()
