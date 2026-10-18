"""Revalidate existing Construction565140 pure fixture on both actual images.

This imports the existing oracle without invoking its overwriting main.
All 640 existing inputs are run at PC24/53/64 on original and independently
loaded v14. Expected outputs remain the preserved existing fixture. Also
prove the Settings dependency failure prefix used by the new owned adapter.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-restoration'))
import native_construction_actions as source
import native_load_oracle as loader
import actor_states as states
from unicorn.x86_const import UC_X86_REG_FPCW
P,V,H=source.P,source.V,source.H

class MissingSettings(Exception): pass
class Failure(source.Machine):
    def hook(self,u,pc,size,user):
        if pc==0x4b2e60:raise MissingSettings()
        super().hook(u,pc,size,user)

def main():
    original_fixture=ROOT/'research/phase8-restoration/native-construction-actions.json'
    expected=json.loads(original_fixture.read_text())
    results=[];observed={};count=0
    for name,image in [('original',P),('v14',V)]:
        # Base Machine maps this chosen PE before state registration. Changing
        # the actor_states observation source has no effect on gameplay data.
        loader.P=image;states.P=image
        for pc,cw in [(24,0x7f),(53,0x27f),(64,0x37f)]:
            for row in expected['cases']:
                machine=source.Machine(row['input'])
                machine.u.reg_write(UC_X86_REG_FPCW,cw)
                assert machine.run()==row['output'],(name,pc,row['input']['name'])
                observed.update(machine.observed);count+=1
            case={'actors':{'self':{'state':(1<<27)|(1<<20),'actions':0xffffffffffffffff,
                'can_sell':True,'can_raze':False,'upgrade_count':0,'properties':0}},
                'allow_sell_under_siege':False,'upgrades_ready':False}
            machine=Failure(case);machine.u.reg_write(UC_X86_REG_FPCW,cw)
            failed=False
            try:machine.run()
            except MissingSettings:failed=True
            assert failed
            actor=machine.actors['self'];bits=machine.r(actor+0xf0)|(machine.r(actor+0xf4)<<32)
            assert bits==(0xffffffffffffffff&~((1<<21)|(1<<22)|(1<<19)))
            results.append({'image':name,'precision':pc,'settings_failure_actions':bits})
    loader.P=P;states.P=P
    for address,data in observed.items():assert data==P.range(address,len(data))==V.range(address,len(data))
    report={'schema':1,'scope':__doc__,'original_sha256':H(P.b),'v14_sha256':H(V.b),
        'producer_sha256':H(Path(__file__).read_bytes()),'existing_oracle_sha256':H(Path(source.__file__).read_bytes()),
        'preserved_fixture_sha256':H(original_fixture.read_bytes()),'native_comparisons':count,
        'distinct_inputs':len(expected['cases']),'observed_instructions':len(observed),
        'settings_failure_cases':results}
    path=HERE/'native-bridge-validation.json'
    if '--write' in sys.argv:path.write_text(json.dumps(report,indent=2)+'\n')
    else:assert json.loads(path.read_text())==report,'native bridge evidence differs'
    print(json.dumps({'native_comparisons':count,'settings_failure_cases':len(results),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
