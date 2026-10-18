"""Actual primary+8/+14 tables and pure6A71D0/4B2830 leaves.
Separate original/v14 machines at all3 PCs; no seams. An unmapped ECX proves
the constant leaves do not read component payloads. Constructor/class binding
is the existing native-component-leaves owner; this probe does not rerun it.
"""
from pathlib import Path
import argparse,ast,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified
from unicorn.x86_const import UC_X86_REG_EAX
sha=lambda b:hashlib.sha256(b).hexdigest()
prior=ROOT/'research/phase8-actor-creation/native_component_leaves.py'
vt=next(ast.literal_eval(n.value)for n in ast.parse(prior.read_text()).body
 if isinstance(n,ast.Assign)and any(isinstance(t,ast.Name)and t.id=='VT'for t in n.targets))

def machine(image,pc):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 return m

def main():
 parser=argparse.ArgumentParser();parser.add_argument('--create',action='store_true');args=parser.parse_args()
 rows=[];pins={};tables=[];executions=0
 for kind,address in vt.items():
  fields={}
  for offset,name in [(8,'init_existing'),(0x14,'after_init')]:
   raw=original.range(address+offset,4);assert raw==modified.range(address+offset,4)
   fields[name]=int.from_bytes(raw,'little');pins[address+offset]=raw
  tables.append(dict(kind=str(kind),vtable=address,**fields))
  for pc in ['Single24','Double53','Extended64']:
   pair=[]
   for image in [original,modified]:
    m=machine(image,pc);result={}
    for name,leaf in fields.items():
     if leaf not in [0x6a71d0,0x4b2830]:continue
     m.u.reg_write(UC_X86_REG_EAX,0xa5a5a5ee)
     value=m.call(leaf,0xdeadc000,[],count=20);executions+=1
     assert value==(0xa5a5a501 if leaf==0x6a71d0 else 0xa5a5a5ee)
     result[name]=dict(leaf=leaf,eax=value)
    for at,raw in m.visited.items():
     assert raw==original.range(at,len(raw))==modified.range(at,len(raw));pins[at]=raw
    pair.append(result)
   assert pair[0]==pair[1]
   rows.append(dict(kind=str(kind),precision=pc,native=pair[0]))
 doc=dict(scope=__doc__,producer_sha256=sha(Path(__file__).read_bytes()),
  original_sha256=sha(original.b),v14_sha256=sha(modified.b),
  prior_table_producer_sha256=sha(prior.read_bytes()),tables=tables,rows=rows,
  constant_leaf_calls=executions,pins=[dict(address=hex(a),bytes=b.hex())for a,b in sorted(pins.items())],seams=[])
 path=HERE/'native-constants.json'
 if args.create:
  with path.open('x')as file:json.dump(doc,file,indent=1);file.write('\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=len(rows)*2,constant_leaf_calls=executions,pins=len(pins),sha256=sha(path.read_bytes()),immutable=not args.create)))
if __name__=='__main__':main()
