"""Whole45BC70 generation/pointer lookup, both PEs/all3PC, no seams.

Input generation arrays and pointer slots are controlled oracle state, not
live Rust constructor receipts. Results normalize native nonnull pointer to
its exact logical reference, for Zone::passes' nullness predicate.
"""
from pathlib import Path
import argparse,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified
from unicorn import UC_HOOK_MEM_READ
sha=lambda b:hashlib.sha256(b).hexdigest()
def run(c,image,pc):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 tile=m.alloc(48);registry=m.alloc(0x60010);actor=m.alloc(0x200)
 ref=c['reference'];slot=ref&65535;generation=registry+4+slot*2;pointer=registry+0x20004+slot*4
 m.w(tile+0x1c,ref);m.w(0xb364c0,registry);m.u.mem_write(generation,struct.pack('<H',c['slot_generation']))
 m.w(pointer,actor if c['present']else 0);reads=[]
 def memory(u,access,address,size,value,data):
  if address==generation:reads.append('generation')
  if address==pointer:reads.append('pointer')
 m.u.hook_add(UC_HOOK_MEM_READ,memory)
 result=m.call(0x45bc70,tile)
 assert result in[0,actor]
 assert reads==(['generation','pointer']if ref>>16==c['slot_generation']else['generation'])
 return dict(live=result!=0,logical=ref if result else 0,reads=reads),m.visited
def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');a=p.parse_args()
 inputs=[dict(reference=0,slot_generation=0,present=False)]
 for ref in[1,0x80000001,0xffff0001,0x1234ffff]:
  for generation in[ref>>16,(ref>>16)^1]:
   for present in[False,True]:inputs.append(dict(reference=ref,slot_generation=generation,present=present))
 rows=[];pins={}
 for name,image in[('original',original),('v14',modified)]:
  for pc in['Single24','Double53','Extended64']:
   for c in inputs:
    out,visited=run(c,image,pc);pins.update(visited);rows.append(dict(image=name,precision=pc,input=c,native=out))
 out=dict(scope=__doc__,original_sha256=sha(original.b),v14_sha256=sha(modified.b),rows=rows,instructions=[])
 for address,raw in sorted(pins.items()):
  assert raw==original.range(address,len(raw))==modified.range(address,len(raw)),hex(address)
  out['instructions'].append(dict(address=hex(address),bytes=raw.hex()))
 path=HERE/'native-primary-reference.json'
 if a.create:
  with path.open('x',encoding='utf-8')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert out==json.loads(path.read_text(encoding='utf-8'))
 print(json.dumps(dict(rows=len(rows),pins=len(pins),fixture_sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
