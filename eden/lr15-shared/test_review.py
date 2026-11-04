import copy,importlib.util,json,tempfile,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('review',Path(__file__).with_name('review.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class ReviewTests(unittest.TestCase):
    def pair(self):
        raw=bytearray(64);raw[8:12]=(1).to_bytes(4,'little');raw[12:16]=(6).to_bytes(4,'little');raw[32:40]=(0x10000).to_bytes(8,'little')
        caps=[{'expression':'x0','hex':raw.hex()}]+[{'expression':e,'value':str(v)} for e,v in [('x0+0x8',1),('x0+0xc',6),('x0+0x10',0),('x0+0x20',0x10000)]]
        e={'event':'entry','point_id':'3','thread_id':'80','process_id':'81','sequence':'1','wall_time_ns':'10','registers':{'x0':'0x20000','x1':'0x0','x30':'0x9000'},'locations':{'0x8000':'main+0xb96770','0x9000':'plugin+1'},'captures':caps}
        r={**e,'event':'return','sequence':'2','wall_time_ns':'11','registers':{'pc':'0x9000'}}
        return {'entry':e,'return':r}
    def test_pair_requires_body_role_thread_and_return_site(self):
        pair=self.pair();self.assertEqual(m.pair_check(pair,1,0)['accessor'],'0x10000')
        for key,value in [('thread_id','99'),('point_id','4'),('process_id','82')]:
            changed=copy.deepcopy(pair);changed['return'][key]=value
            with self.assertRaises(AssertionError):m.pair_check(changed,1,0)
        with self.assertRaises(AssertionError):m.pair_check(pair,1,1)
        with self.assertRaises(AssertionError):m.pair_check(pair,2,0)
    def test_regular_build_is_not_an_interruption(self):
        lines=['100 #1 bodies queued owner=0','110 #2 [Destroy] plan owner=0 reason=finalize','120 #4 construction drained owner=0 waited=1 us=10 alias_ok=true','130 #5 [GroupSwap] teardown owner=0 map_after=0 skipped=0 alias_ok=true','130 #6 [Ledger] reconcile owner=0 phase=finalize map=0 task_pending=0 predicted=[] actual=[] residual=[] missing=[]']
        self.assertEqual(m.teardown(lines)['interruptions'],1)
        with self.assertRaises(AssertionError):m.teardown(lines+['115 #3 [GroupSwap] built owner=0'])
        with self.assertRaises(AssertionError):m.teardown([x.replace('residual=[]','residual=[1]') for x in lines])
    def test_manifest_binding_rejects_mutated_bytes(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);d=root/'out/manifests';d.mkdir(parents=True)
            p=d/'temp';p.write_text(json.dumps({'rows':[{'id':'A'}]}));digest=m.sha(p);p=p.rename(d/(digest+'.json'))
            receipt={'id':'A','manifest_sha256':digest};self.assertEqual(m.receipt_binding(root,receipt),digest)
            p.write_text(p.read_text()+' ')
            with self.assertRaises(AssertionError):m.receipt_binding(root,receipt)
if __name__=='__main__':unittest.main()
