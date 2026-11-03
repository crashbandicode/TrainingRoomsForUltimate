import importlib.util,tempfile,unittest,struct
from pathlib import Path
def module(name):
    s=importlib.util.spec_from_file_location(name,Path(__file__).with_name(name+'.py'))
    m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m
class EvidenceTests(unittest.TestCase):
    def test_allocator_distinguishes_unknown_from_zero_and_companions(self):
        with tempfile.TemporaryDirectory(dir='/tmp/opencode') as d:
            p=Path(d)/'log';p.write_text('[Companion] heap_allocations=999\n[TrainingModpack] heap_allocations=unmetered\n[TrainingRooms] hook_stats allocations=0 delta=0\n')
            rows=module('alloc_compare').extract(p)['rows']
            self.assertEqual([(x['line'],x['counters']) for x in rows],[(2,{'heap_allocations':'unmetered'}),(3,{'allocations':'0'})])
    def test_native_tuple_offsets_do_not_use_preidentity_or_adjacent_bytes(self):
        raw=bytearray([0xee]*0x260)
        struct.pack_into('<5H4B',raw,0x1a8,7,61,9,3,42,2,1,0,2)
        self.assertEqual(module('review_trace').decode(raw),{'index':7,'tokens':[61,9,3,42],'specials':[2,1,0,2]})
if __name__=='__main__':unittest.main()
