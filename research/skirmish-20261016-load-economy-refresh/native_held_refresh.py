"""Actual570020/5704C0 composition over the immediate held Actor/Kingdom.

The full refresh and concrete Kingdom/vector arithmetic execute. Pure value
getter/predicate seams in that outer run are independently recomputed using
the actual captured source/actor graph in original/v14 getter executions.
Allocation/free and Properties-context identity remain named seams. Actual
Kingdom arrays come from the immediate load; no test reset/default is supplied.
"""
from pathlib import Path
import sys,json,hashlib,copy,importlib.util
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-saved-economy-integration'))
import native_saved_refresh as old
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-saved-economy-values'))
from native_saved_values import Native as SourceMachine
H=lambda b:hashlib.sha256(b).hexdigest()
OUTER_LEAVES={0x6f7810,0x6f7430,0x56fb10,0x56fc20,0x56fa30,0x56f9a0,0x4cf0f0}
SOURCE_LEAVES={0x6f7810,0x6f7430,0x4cf110}
class RefreshMachine(old.RefreshMachine):
 def run_entry(self,entry):
  self.call(entry,[],this=old.native.COMPONENT)
  old.check_null_accesses(self)
  state={}
  for name,off in old.FIELDS.items():
   count=self.read(self.KINGDOM+off+4);ptr=self.read(self.KINGDOM+off)
   import struct
   state[name]=list(struct.unpack('<'+'I'*count,self.uc.mem_read(ptr,4*count)))if count else[]
  return dict(component=self.snapshot(),kingdom=state)
def main():
 source=HERE/'actual-held-input.json';capture=json.loads(source.read_text());rows=[];values=[];executed={};leaf_entries={}
 def record(pins,leaves):
  for a,b in pins.items():
   raw=old.ORIGINAL.at(a,b)if isinstance(b,int)else b
   assert raw==old.ORIGINAL.at(a,len(raw))==old.ACCEPTED.at(a,len(raw)),hex(a)
   (leaf_entries if a in leaves else executed)[a]=raw.hex()
 for case in capture['cases']:
  c=case['input'];graph=case['graph'];assert not case['flow_reset_in_test']
  for image in ('original','v14'):
   for operation,key in [('Cost','cost'),('Upkeep','upkeep'),('Production','provided'),('eligible',None),('upkeep_blocked',None)]:
    if operation=='Upkeep'and c['block_reason']!='none':continue
    if operation=='Production'and not c['provides'][0]:continue
    g=copy.deepcopy(graph);g.update(actor=1,operation=operation,cache_members=True,precision=c['precision'])
    machine=SourceMachine(g,image);result=machine.run();record(machine.visited,SOURCE_LEAVES)
    if key:assert result['values']==c[key],(case['actor'],operation,result['values'],c[key])
    elif operation=='eligible':assert result['predicate']==c['provides'][0]
    else:assert result['predicate']==(c['block_reason']!='none')
    values.append(dict(actor=case['actor'],precision=c['precision'],image=image,operation=operation,output=result))
  for entry in (0x570020,0x5704c0):
   results=[]
   for image in ('original','v14'):
    machine=RefreshMachine(c,image);result=machine.run_entry(entry);record(machine.visited,OUTER_LEAVES)
    assert result==case['output'],(case['actor'],c['precision'],hex(entry),image,result,case['output'])
    results.append(result)
   assert results[0]==results[1]
   rows.append(dict(actor=case['actor'],owner=case['owner'],precision=c['precision'],entry=hex(entry),input=c,output=results[0]))
 dependencies=[ROOT/'research'/p for p in ['skirmish-20261016-saved-economy-integration/native_saved_refresh.py','skirmish-20261016-saved-economy-values/native_saved_values.py','phase7-economy-refresh/native_refresh.py','phase7-economy-values/native_values.py','evidence/economy-phase2-oracle.py','phase8-economy/harness.py']]
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),captured_input_sha256=H(source.read_bytes()),original_sha256=old.sha(old.ORIGINAL.data),v14_sha256=old.sha(old.ACCEPTED.data),cases=rows,source_cases=values,refresh_machines=len(rows)*2,source_machines=len(values),executed_address_byte_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(executed.items())],intercepted_entry_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(leaf_entries.items())],dependencies=[dict(path=p.relative_to(ROOT).as_posix(),sha256=H(p.read_bytes()))for p in dependencies])
 raw=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-held-refresh.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),refresh_machines=len(rows)*2,source_machines=len(values),executed_pins=len(executed),intercepted_entries=len(leaf_entries),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
