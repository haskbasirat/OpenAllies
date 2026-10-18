"""Actual-source constructor4D4CF0 list fields on original/v14, all PCs.

Same sealed borrowed-core oracle; allocation/free remain explicit transport
seams. The missing moved-Bank dependency probe stops at the native source-flags
read after actual old-list clear, not at a fabricated successful callback.
"""
from pathlib import Path
import sys,json,hashlib,copy,importlib.util
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
SOURCE=ROOT/'research/skirmish-20261016-actor-property-core/native_actor_property_core.py'
spec=importlib.util.spec_from_file_location('actor_list_core',SOURCE);core=importlib.util.module_from_spec(spec);spec.loader.exec_module(core)

def run(row,image,pc):
 c=copy.deepcopy(row['input']);names=sorted({v for a in c['actors']for key in ['list','source']for v in a[key]})
 ids={name:i+1 for i,name in enumerate(names)};reverse={v:k for k,v in ids.items()}
 for a in c['actors']:
  for key in ['list','source']:a[key]=[ids[v]for v in a[key]]
 if row['phase']=='missing_source_bank':c['stop']=dict(event='definition_flags',occurrence=1)
 out=core.Probe(c,image,pc).run();actor=out['actors'][0];node=actor['head'];values=[];seen=set()
 while node is not None:
  assert node not in seen;seen.add(node);n=out['nodes'][node];assert not n['freed'];values.append(reverse[n['value']]);node=n['next']
 old=len(c['actors'][0]['list']);sources=len(c['actors'][0]['source'])
 owned_live=sum(not node['freed']for i,node in enumerate(out['nodes'])if i<old or i>=old+sources)
 state=dict(values=values,allocated_nodes=owned_live,head=actor['head']is not None,tail=actor['tail']is not None)
 assert out['outcome']==('definition_flags'if row['phase']=='missing_source_bank'else'ok')
 return state,out

def main():
 inputs=json.loads((HERE/'actual-constructor-inputs.json').read_text());rows=[];runs=0
 for row in inputs['rows']:
  states={};traces={}
  for pc in core.h.PRECISION:
   a,ta=run(row,'original',pc);b,tb=run(row,'v14',pc);assert(a,ta)==(b,tb);assert a==row['output'];runs+=2
   states[pc]=a;traces[pc]=ta
  rows.append(dict(phase=row['phase'],input=row['input'],output=row['output'],native=traces))
 seams={0x6f7810,0x6f7430}
 for va,raw in core.h.VISITED.items():assert core.ORIGINAL.range(va,len(bytes.fromhex(raw)))==core.h.V.range(va,len(bytes.fromhex(raw)))==bytes.fromhex(raw)
 doc=dict(schema=1,scope=__doc__,core_source_sha256=hashlib.sha256(SOURCE.read_bytes()).hexdigest(),rows=rows,native_executions=runs,
  instructions=[dict(va=hex(k),bytes=v)for k,v in sorted(core.h.VISITED.items())if k not in seams],
  intercepted_entry_byte_pins=[dict(va=hex(k),bytes=v)for k,v in sorted(core.h.VISITED.items())if k in seams])
 p=HERE/'native-stored-actor-properties.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=runs,executed_instruction_pins=len(doc['instructions']),intercepted_entry_pins=len(doc['intercepted_entry_byte_pins']),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
