"""Original/v144E48C0/4E4930/4E3940/4E1F60 array writes and failure prefixes.

Finite supported inputs reuse preserved phase2 native fixtures, each action
starting at that fixture's native prior state. No arithmetic is hooked.
Explicit injected dependency stops occur BEFORE native memory instructions;
they are not represented as native exceptions or native allocation behavior.
"""
from pathlib import Path
import importlib.util,json,sys,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
spec=importlib.util.spec_from_file_location('economy_base',ROOT/'research/evidence/economy-phase2-oracle.py')
base=importlib.util.module_from_spec(spec);spec.loader.exec_module(base)
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EAX,UC_X86_REG_EDX
ORIGINAL,ACCEPTED=base.ORIGINAL,base.ACCEPTED
STORES={0x4e1ff3,0x4e395d,0x4e39a4,0x4e39c2,0x4e4911}
FIELDS=['raw_provided','effective_provided','consumed']
class Dependency(Exception):pass
class Machine(base.Machine):
    def __init__(self,case,cw):
        self.case=case;self.writes=[];self.store_attempt=0
        super().__init__(case['catalog'],case['initial'],case['limits'],cw)
        self.put(0x2020000+0x38,case.get('resource_count',self.count))
        self.put(0x2022000+0x28,case.get('consumption_count',self.count))
        self.put(self.KINGDOM+0x1a4,case.get('effective_count',self.count))
        self.uc.hook_add(UC_HOOK_MEM_WRITE,self.write_observer)
    def write_observer(self,u,access,address,size,value,user):
        for field in FIELDS:
            offset=address-self.ARRAYS[field]
            if 0<=offset<self.count*4:
                assert size==4 and offset%4==0
                self.writes.append([field,offset//4,value&0xffffffff]);return
    def observe(self,u,pc,size,user):
        failure=self.case.get('failure')
        if pc in STORES:
            if failure and failure.get('write')==self.store_attempt:raise Dependency('write')
            self.store_attempt+=1
        if failure:
            if failure.get('maximum') and pc==0x4e3971:raise Dependency('maximum')
            if failure.get('coefficient') and pc==0x4e1fe4:raise Dependency('coefficient')
            if 'input_length' in failure and pc in [0x4e48e7,0x4e48ef,0x4e4957,0x4e495f]:
                if u.reg_read(UC_X86_REG_ESI)>=failure['input_length']:raise Dependency('input_length')
            if failure.get('consumed_missing') and pc==0x4e4909:raise Dependency('consumed_missing')
        super().observe(u,pc,size,user)
    def run(self):
        outcome='ok'
        try:self.action(self.case['action'])
        except Dependency as error:outcome=str(error)
        return {'outcome':outcome,'state':self.state(),'writes':self.writes}

def cases():
    preserved=json.loads((ROOT/'crates/aa-economy/tests/fixtures/economy-native-vectors.json').read_text())
    # The fixture already includes all three precision variants; generate
    # source inputs from its first variant then independently run all modes.
    for scenario in preserved['cases']:
        if scenario['precision']!='Single24':continue
        # Each precision has its own native prior state after earlier actions.
        mode_scenarios={s['precision']:s for s in preserved['cases'] if s['name']==scenario['name']}
        for step_index,step in enumerate(scenario['steps']):
            if step['action']['kind'] not in ['Provider','Providers','Consumption']:continue
            yield {'name':scenario['name']+'_'+str(step_index),'catalog':scenario['catalog'],
                'limits':scenario['limits'],'action':step['action'],
                'by_precision':{mode:{'initial':s['initial'] if step_index==0 else s['steps'][step_index-1]['expected']['state'],
                                    'expected':s['steps'][step_index]['expected']['state']}
                                for mode,s in mode_scenarios.items()}}
    cat=base.catalog(['Stockpiled','Upkeep','Stockpiled'],[[0,0,0],[2,0,3],[0,0,0]],limited=(1,))
    initial={'raw_provided':list(map(base.bits,[5,2,7])),
        'effective_provided':list(map(base.bits,[5,2,7])),
        'consumed':list(map(base.bits,[17,8,23])),'stockpiled':list(map(base.bits,[100,0,200]))}
    item={'catalog':cat,'limits':{'provided_maximum':100,'stockpiled_maximum':base.bits(10000)},'initial':initial}
    for kind,action in [('providers',{'kind':'Providers','values':list(map(base.bits,[0,4,0])),'add':True}),
                        ('consumption',{'kind':'Consumption','values':list(map(base.bits,[0,3,0])),'add':False})]:
        for failure in [{'write':n} for n in [0,1,2,3,6,8]]+[{'coefficient':True},{'input_length':1}]:
            yield dict(copy.deepcopy(item),name=kind+'_'+str(failure),action=action,failure=failure)
    yield dict(copy.deepcopy(item),name='provider_missing_maximum',
        action={'kind':'Provider','index':1,'delta':base.bits(4)},failure={'maximum':True})
    for counts in [{'effective_count':0},{'effective_count':-1},
                   {'effective_count':1,'consumption_count':2,'resource_count':1},
                   {'effective_count':2,'consumption_count':1,'resource_count':2},
                   {'effective_count':3,'consumption_count':0,'resource_count':-1}]:
        for kind in ['Consumption','Providers']:
            yield dict(copy.deepcopy(item),name='counts_'+kind+'_'+str(counts),
                action={'kind':kind,'values':list(map(base.bits,[0,1,2])),'add':True},**counts)
    yield dict(copy.deepcopy(item),name='uninitialized_consumed_after_empty_shortage',effective_count=0,
        action={'kind':'Consumption','values':list(map(base.bits,[1,2,3])),'add':True},
        failure={'consumed_missing':True})

def main():
    rows=[]
    for image_name,image in [('original',ORIGINAL),('v14',ACCEPTED)]:
        base.ORIGINAL=image
        for pc,cw in base.MODES:
            for source in cases():
                case=copy.deepcopy(source)
                if 'by_precision'in case:
                    precise=case.pop('by_precision')[pc];case['initial']=precise['initial']
                else:precise=None
                output=Machine(case,cw).run()
                if precise:assert output['outcome']=='ok' and output['state']==precise['expected']
                rows.append({'image':image_name,'precision':pc,'input':case,'output':output})
    base.ORIGINAL=ORIGINAL
    ranges={name:{'start':hex(a),'end':hex(b),'sha256':base.sha(ORIGINAL.at(a,b-a))}
            for name,(a,b) in base.RANGES.items() if name in ['shortage','provider','consumption_vector','provider_vector']}
    out={'schema':1,'scope':__doc__,'original_sha256':base.sha(ORIGINAL.data),'v14_sha256':base.sha(ACCEPTED.data),
        'producer_sha256':base.sha(Path(__file__).read_bytes()),'base_oracle_sha256':base.sha(Path(base.__file__).read_bytes()),
        'preserved_fixture_sha256':base.sha((ROOT/'crates/aa-economy/tests/fixtures/economy-native-vectors.json').read_bytes()),
        'ranges':ranges,'cases':rows}
    path=HERE/'native-borrowed-flows.json'
    if '--write'in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
    else:assert json.loads(path.read_text())==out,'native borrowed flows differ'
    print(json.dumps({'cases':len(rows),'sha256':base.sha(path.read_bytes())}))
if __name__=='__main__':main()
