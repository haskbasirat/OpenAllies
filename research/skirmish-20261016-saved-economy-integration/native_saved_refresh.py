"""Bounded original570020 composition over captured saved/source facts.

The initialized fixture explicitly runs the proven Init flow-reset fragment
in Rust test storage; it is not a claim that natural record Init is bound.
Natural linked HQ cases retain empty Kingdom arrays and execute the actual
4CF0F0 absent-parent-Economy return and56F9A0 linked early return. Cost/value
seams are independently re-executed using the saved source graph in original
code, rather than accepting Rust vector output as the expected answer.
"""
from pathlib import Path
import copy, importlib.util, json, struct, sys, hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import guard_null_page,check_null_accesses
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-saved-economy-values'))
from native_saved_values import Native as SourceMachine
spec=importlib.util.spec_from_file_location('refresh_native',ROOT/'research/phase7-economy-refresh/native_refresh.py')
native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
from unicorn.x86_const import UC_X86_REG_ECX

FIELDS={'raw_provided':0x1ac,'effective_provided':0x1a0,'consumed':0x1b8,'stockpiled':0x1c4}
ORIGINAL=native.base.ORIGINAL;ACCEPTED=native.base.ACCEPTED
def sha(data):return hashlib.sha256(data).hexdigest()
def load_json(path):return json.loads(path.read_text())

class RefreshMachine(native.Machine):
    def __init__(self,c,image,natural=False):
        self.natural=natural
        old=native.base.ORIGINAL
        native.base.ORIGINAL=ORIGINAL if image=='original' else ACCEPTED
        try:super().__init__(c)
        finally:native.base.ORIGINAL=old
        guard_null_page(self)
        # Actual stable owner dispatch, not an allegiance callback seam.
        self.put(native.VT+0x108,0x4d1e80);self.put(native.ACTOR+0xe8,self.KINGDOM)
        for k,state in zip(self.kingdoms,c['kingdoms']):
            for name,offset in FIELDS.items():self.put(k+offset+4,len(state[name]))
        # Captured HQ parents genuinely have no Economy payload.
        self.put(native.ORGANIZATION+0x84,0)
        self.visited={}
    def observe(self,u,address,n,user):
        if address==0x4cf0f0:
            self.visited[address]=n;self.event('RefreshParent',2)
            assert self.read(u.reg_read(UC_X86_REG_ECX)+0x84)==0
            return # execute actual absent-Economy branch
        if self.natural and address==0x56f9a0:
            self.visited[address]=n
            return # actual linked early return, no predicate seam
        super().observe(u,address,n,user)
    def run_actual(self):
        result=super().run();check_null_accesses(self)
        state={}
        for name,offset in FIELDS.items():
            count=self.read(self.KINGDOM+offset+4)
            state[name]=list(struct.unpack('<'+'I'*count,self.uc.mem_read(self.read(self.KINGDOM+offset),4*count))) if count else []
        if self.natural:
            assert 0x4cf0f0 in self.visited and 0x56f9a0 in self.visited
            assert 0x4e48c0 not in self.visited and 0x4e4930 not in self.visited
        for a,n in self.visited.items():assert ORIGINAL.at(a,n)==ACCEPTED.at(a,n),hex(a)
        return {'component':result['component'],'kingdom':state}

def source_result(graph,raw,op,image,pc):
    dense=graph['source_actor_ids'].index(raw)+1
    c=copy.deepcopy(graph);c.update(actor=dense,operation=op,cache_members=True,precision=pc)
    return SourceMachine(c,image).run()

def main():
    captured=load_json(HERE/'actual-initialized-input.json')
    graphpath=ROOT/'research/skirmish-20261016-saved-economy-values/actual-saved-input.json'
    graph=load_json(graphpath)
    rows=[];executions=[];visited=set()
    for case in captured:
        c=case['input'];results=[]
        for image in ('original','v14'):
            for op,key in [('Cost','cost'),('Upkeep','upkeep'),('Production','provided')]:
                source=source_result(graph,case['actor'],op,image,c['precision'])
                assert source['values']==c[key],(case['actor'],op,source['values'],c[key])
            machine=RefreshMachine(c,image);actual=machine.run_actual();visited.update(machine.visited)
            assert actual==case['rust_output'],(case['actor'],image,actual,case['rust_output'])
            results.append(actual);executions.append({'image':image,'actor':case['actor'],'precision':c['precision'],'output':actual})
        assert results[0]==results[1]
        rows.append({'input':c,'output':results[0],'actor':case['actor'],'definition':case['definition']})
    common={'scope':__doc__,'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),'source_graph_sha256':sha(graphpath.read_bytes()),'generator_sha256':sha(Path(__file__).read_bytes())}
    (HERE/'native-initialized-refresh.json').write_text(json.dumps({**common,'cases':rows,'executions':executions},indent=2)+'\n')
    observations=load_json(HERE/'actual-natural-refresh.json');hq=[]
    for row in observations:
        if row['early_return'] or row['error'] is not None:continue
        assert row['actor'] in [4100,8131] and row['parent'] is not None and row['parent_economy_present'] is False
        for pc,_ in native.base.MODES:
            for image in ('original','v14'):
                assert source_result(graph,row['actor'],'eligible',image,pc)['predicate'] is False
                assert source_result(graph,row['actor'],'upkeep_blocked',image,pc)['predicate'] is True
                cost=source_result(graph,row['actor'],'Cost',image,pc)['values']
                c=copy.deepcopy(captured[0]['input']);c.update(name=f"actual_hq_{row['actor']}_{pc}",precision=pc,
                    flags=[int(row['before']['flag_10']),int(row['before']['flag_11'])],resources=[v['values'] for v in row['before']['resources']],
                    kingdoms=[row['before_kingdom'],row['before_kingdom']],block_reason='linked',parent=True,provides=[False],cost=cost)
                machine=RefreshMachine(c,image,natural=True);actual=machine.run_actual();visited.update(machine.visited)
                expected={'component':{'flags':[int(row['after']['flag_10']),int(row['after']['flag_11'])],'resources':[v['values'] for v in row['after']['resources']]},'kingdom':row['after_kingdom']}
                assert actual==expected,(row['actor'],image,pc,actual,expected)
                hq.append({'actor':row['actor'],'image':image,'precision':pc,'input':c,'output':actual})
    assert len(hq)==12
    (HERE/'native-natural-hq-refresh.json').write_text(json.dumps({**common,'cases':hq,'natural_observation_sha256':sha((HERE/'actual-natural-refresh.json').read_bytes())},indent=2)+'\n')
    print(json.dumps({'initialized_executions':len(executions),'natural_hq_executions':len(hq),'source_getter_executions':len(executions)*3+len(hq)*3,'instruction_sites':len(visited)}))
if __name__=='__main__':main()
