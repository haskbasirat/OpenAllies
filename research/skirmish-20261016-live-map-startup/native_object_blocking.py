"""Actual4EB6E0 control/GTID dispatch and real4D67F0 children, both P/V/allPC.
554350/555620 are EXPLICIT descriptor/discovery seams, not spatial proof.
The outer descriptor overrides are observed at the real555620 call. Children
execute native eligibility/typed-list expiry code and receive an empty query
result only as the explicitly supplied fixture input. Allocation/free reuse
the recorded native-machine seams. No other successful native body is hidden.
"""
from pathlib import Path
import argparse,hashlib,json,sys
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-conversion'))
import native_machine as nm
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ECX

def run(image,pc,c):
 m=nm.Machine(pc)
 old=(nm.P.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 actor=m.alloc(0x200,0xA5);world=m.alloc(0x200,0xA5);manager=m.alloc(0x60004,0)
 m.w(actor+8,c['flags']);m.w(0xB36534,world);m.w(world+0x30,0x12345678);m.w(world+0xE0,c['time'])
 m.w(0xB364C0,manager);owners={};refs=[]
 for i,d in enumerate(c['discovered'],1):
  a=m.alloc(0x200,0xA5);ref=0x10000+i;owners[a]=(i,d);refs.append(ref)
  vt=0xA273F0 if d['actor'] else 0xA2A2A0
  assert image.range(vt+0xB4,4)==nm.P.range(vt+0xB4,4)==nm.V.range(vt+0xB4,4)
  m.w(a,vt)
  m.w(a+8,d['flags']);m.w(a+0x100,d['low']);m.w(a+0x104,d['high'])
  m.w(a+0x110,0);m.w(a+0x114,0)
  node=m.alloc(12,0xA5);m.w(node,0x20002);m.w(node+4,0);m.w(node+8,0)
  m.w(a+0x110,node);m.w(a+0x114,node)
  m.u.mem_write(manager+4+i*2,(1).to_bytes(2,'little'));m.w(manager+0x20004+i*4,a)
 events=[];pins={};seams={};prepared={}
 def observe(_,address,size,__):
  if image.base<=address<image.base+image.image_size:
   raw=image.range(address,size);assert raw==nm.P.range(address,size)==nm.V.range(address,size)
   (seams if address in m.seams else pins)[f'{address:08x}']=raw.hex()
  if address==0x4D67F0:events.append(['refresh',owners[m.u.reg_read(UC_X86_REG_ECX)][0]])
 m.u.hook_add(UC_HOOK_CODE,observe)
 def prepare(machine):
  who=machine.arg(0);assert machine.arg(1)==0
  base=machine.u.reg_read(UC_X86_REG_ECX);prepared[base]=who
  for offset in range(0,0x50,4):machine.w(base+offset,0x98760000+offset)
  events.append(['prepare',0 if who==actor else owners[who][0]])
  machine.ret(8)
 def query(machine):
  descriptor=machine.arg(0);out=machine.arg(1);assert machine.arg(2)==1
  who=prepared[descriptor]
  if who==actor:
   assert machine.r(descriptor+0x1C)==0 and machine.r(descriptor+0x20)==actor
   events.append(['outer_overrides',None,'current'])
   values=refs
  else:
   events.append(['child_query',owners[who][0]]);values=[]
  nodes=[]
  for ref in values:
   n=machine.alloc(8,0xA5);machine.w(n,ref);nodes.append(n)
  for i,n in enumerate(nodes):machine.w(n+4,nodes[i+1] if i+1<len(nodes) else 0)
  machine.w(out,nodes[0] if nodes else 0);machine.w(out+4,nodes[-1] if nodes else 0)
  machine.ret(12)
 m.seam(0x554350,prepare);m.seam(0x555620,query)
 m.call(0x4EB6E0,actor,[],0)
 assert not m.null_accesses and m.outcome is None
 remaining=[]
 for a,(i,d) in owners.items():remaining.append([i,m.r(a+0x110)!=0,m.r(a+0x114)!=0])
 return dict(events=events,remaining=remaining),pins,seams

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--write',action='store_true');args=ap.parse_args()
 kinds=[dict(actor=False,flags=1<<10,low=0,high=0),dict(actor=True,flags=(1<<29)|(1<<10),low=0,high=0),dict(actor=True,flags=0,low=0,high=0),dict(actor=True,flags=1<<10,low=1<<25,high=0),dict(actor=True,flags=1<<10,low=0,high=0)]
 inputs=[dict(flags=flags,time=time,discovered=ds) for flags in [0,1<<10,(1<<29)|(1<<10)] for time in [0,0x3F800000] for ds in [[],kinds,kinds[::-1]]]
 cases=[];pins={};seams={}
 for pc in ['Single24','Double53','Extended64']:
  for c in inputs:
   a,pa,sa=run(nm.P,pc,c);b,pb,sb=run(nm.V,pc,c);assert a==b and pa==pb and sa==sb
   cases.append(dict(precision=pc,input=c,output=a));pins.update(pa);seams.update(sa)
 out=dict(schema=1,scope=__doc__,paired_cases=len(cases),native_executions=len(cases)*2,cases=cases,executed_instruction_pins=pins,seam_entry_pins=seams,source_images=[dict(name=name,sha256=hashlib.sha256(image.b).hexdigest()) for name,image in [('original',nm.P),('v14',nm.V)]])
 target=HERE/'native-object-blocking.json'
 if args.write:
  assert not target.exists();target.write_text(json.dumps(out,indent=2)+'\n')
 else:assert json.loads(target.read_bytes())==out
 print(json.dumps(dict(paired_cases=len(cases),native_executions=len(cases)*2,pins=len(pins),seam_pins=len(seams),sha256=hashlib.sha256(target.read_bytes()).hexdigest())))
if __name__=='__main__':main()
