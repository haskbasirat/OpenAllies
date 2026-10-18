"""Bounded actual 4D67F0 list/eligibility path, both original/v14 and all x87 PCs.
Executes actual4D10A0, expiry/removal79E3C0, successful prepend and temporary
list cleanup. Required queries554350/555620 are explicit seams supplying only
query discovery references; this fixture is NOT a spatial/collision proof.
A separate stop-before-query run proves expiry remains committed on failure.
Allocation6F7810/free6F7430 use the existing recorded native-machine seams.
"""
from pathlib import Path
import argparse,hashlib,json,sys
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-conversion'))
import native_machine as nm
from unicorn import UC_HOOK_CODE
class Required(Exception): pass

def run(image,pc,row):
 m=nm.Machine(pc)
 old=(nm.P.image_size+4095)&~4095; new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 actor=m.alloc(0x200,0xA5); world=m.alloc(0x200,0xA5)
 m.w(actor+8,row['flags']);m.w(actor+0x100,row['state_low']);m.w(actor+0x104,row['state_high'])
 m.w(0xB36534,world);m.w(world+0xE0,row['time_bits']);m.w(world+0x30,0x12345678)
 nodes=[]
 for ref,expiry in row['entries']:
  node=m.alloc(12,0xA5);m.w(node,ref);m.w(node+4,expiry);nodes.append(node)
 for i,node in enumerate(nodes):m.w(node+8,nodes[i+1] if i+1<len(nodes) else 0)
 m.w(actor+0x110,nodes[0] if nodes else 0);m.w(actor+0x114,nodes[-1] if nodes else 0)
 pins={}; events=[]
 def observe(_,address,size,__):
  if image.base<=address<image.base+image.image_size:
   raw=image.range(address,size);assert raw==nm.P.range(address,size)==nm.V.range(address,size)
   pins[f'{address:08x}']=raw.hex()
  if address==0x4D10A0:events.append('actual_4D10A0')
 m.u.hook_add(UC_HOOK_CODE,observe)
 def prepare(machine):
  events.append('required_554350')
  assert machine.arg(0)==actor and machine.arg(1)==0
  if row['stop_before_query']:raise Required()
  machine.ret(8)
 def collect(machine):
  events.append('required_555620')
  assert machine.arg(2)==1
  out=machine.arg(1);found=[]
  for ref in row['discovered']:
   n=machine.alloc(8,0xA5);machine.w(n,ref);found.append(n)
  for i,n in enumerate(found):machine.w(n+4,found[i+1] if i+1<len(found) else 0)
  machine.w(out,found[0] if found else 0);machine.w(out+4,found[-1] if found else 0)
  machine.ret(12)
 m.seam(0x554350,prepare);m.seam(0x555620,collect)
 outcome='returned'
 try:m.call(0x4D67F0,actor,[],0)
 except Required:outcome='required_query'
 assert not m.null_accesses and m.outcome is None
 out=[];node=m.r(actor+0x110);last=0
 while node:
  assert len(out)<100;out.append([m.r(node),m.r(node+4)]);last=node;node=m.r(node+8)
 assert last==m.r(actor+0x114)
 return dict(outcome=outcome,entries=out,events=events,head_null=m.r(actor+0x110)==0,tail_null=m.r(actor+0x114)==0),pins

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--write',action='store_true');args=ap.parse_args()
 entries=[[0x10001,0],[0x10002,0x3F800000],[0x10001,0x7F800000],[0x10003,0x7FC00001],[0x10004,0xFF800000],[0x10005,0x80000000],[0x10006,0xBF800000]]
 inputs=[]
 for flags,low,high in [(1<<29,0,0),(0,0,0),(1<<10,1<<25,0),(1<<10,1<<26,0),(1<<10,0,4),(1<<10,0,16),(1<<10,1,1),(1<<10,0,0)]:
  for time in [0,0x80000000,0x3F800000,0xBF800000,0x7F800000,0xFF800000,0x7FC00001]:
   for stop in [False,True]:
    inputs.append(dict(flags=flags,state_low=low,state_high=high,time_bits=time,entries=entries,discovered=[0x20002,0x30003,0x20002],stop_before_query=stop))
 for time in [0,0x3F800000]:
  inputs.append(dict(flags=1<<10,state_low=0,state_high=0,time_bits=time,entries=[],discovered=[],stop_before_query=False))
 cases=[];pins={}
 for pc in ['Single24','Double53','Extended64']:
  for row in inputs:
   a,pa=run(nm.P,pc,row);b,pb=run(nm.V,pc,row);assert a==b and pa==pb
   pins.update(pa);cases.append(dict(precision=pc,input=row,output=a))
 target=HERE/'native-actor-overlap-list.json'
 output=dict(schema=1,scope=__doc__,paired_case_count=len(cases),native_execution_count=len(cases)*2,cases=cases,instruction_pins=pins,source_images=[dict(name=name,sha256=hashlib.sha256(image.b).hexdigest()) for name,image in [('original',nm.P),('v14',nm.V)]])
 if args.write:
  assert not target.exists(),'frozen fixture exists';target.write_text(json.dumps(output,indent=2)+'\n')
 else:assert json.loads(target.read_text())==output
 print(json.dumps(dict(paired_cases=len(cases),native_executions=len(cases)*2,pins=len(pins),sha256=hashlib.sha256(target.read_bytes()).hexdigest())))
if __name__=='__main__':main()
