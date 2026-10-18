"""Fresh full-registry bounded payload replay, preserving the old mixed corpus.

Eight small-input seeds per descriptor; stop at the first actual native null
access. Definition manager choices remain explicit synthetic fixture inputs.
Order ordinals/count are always produced by actual CRT registration execution.
"""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-orders'))
import native_order_traces as n
from recover_orders import proofs

fixture=HERE/'synthetic-definition-managers.json'
choices=json.loads(fixture.read_text())['managers']
n.manager_globals=lambda:sorted(int(g,16)for g in choices)

class Machine(n.TraceMachine):
 def null_hook(self,u,access,address,size,value,user):
  before=len(self.null_accesses)
  super().null_hook(u,access,address,size,value,user)
  if len(self.null_accesses)>before:u.emu_stop()

m=Machine()
actual=[(row['id'],row['name'],int(row['handler'],16))for row in m.registrations]
assert actual==n.DESCRIPTORS and len(actual)==358
assert {f'{g:#x}':c for g,c in m.managers.items()}==choices
out=[]
for descriptor in actual:
 validity=[int(v,16)for v in n.STATIC[descriptor[0]].get('validity')or[]]
 runs=[]
 for seed in range(8):
  row=m.run(descriptor,seed,'small',force=validity)
  row['forced_validity']=[hex(v)for v in validity]
  runs.append(row)
 out.append({'id':descriptor[0],'name':descriptor[1],'handler':hex(descriptor[2]),'runs':runs})
 print(descriptor[0],descriptor[1],sorted({r['status']for r in runs}),flush=True)
report={'schema':1,'scope':__doc__,'original_sha256':n.sha(n.PE0.b),'v14_sha256':n.sha(n.PE1.b),
        'producer_sha256':n.sha(Path(__file__).read_bytes()),'trace_machine_sha256':n.sha((ROOT/'research/phase8-orders/native_order_traces.py').read_bytes()),
        'synthetic_manager_input_sha256':n.sha(fixture.read_bytes()),'registry_context_sha256':n.sha(json.dumps(m.registrations,sort_keys=True).encode()),
        'managers':{f'{g:#x}':c for g,c in m.managers.items()},
        'environment':{'definition_plus_0x20':f'index % {n.DEF_KIND_MODULUS}','definition_plus_0xc':'index',
                       'team_command_flags':n.TEAM_FLAGS,'session_player_ids':n.SESSION_PLAYER_IDS,
                       'thing_slots':'every slot resolves; thing+0x14 low16 = slot; all type checks true',
                       'definitions_valid':'virtual +0x38 returns1 for every index<count','game_data_plus_0x4a0_word':n.GD_4A0_WORD},
        'shared_code_proof':proofs(m),'descriptors':out}
path=ROOT/'research/phase8-orders/order-traces.json'
path.write_text(json.dumps(report)+'\n')
print(json.dumps({'descriptors':len(out),'runs':sum(len(d['runs'])for d in out),'sha256':n.sha(path.read_bytes())}))
