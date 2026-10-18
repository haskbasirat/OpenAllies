"""Bounded actual57E57A..58C and57E62E..640 comparison/store slices.
Explicit finite candidate/prior binary32 values; FPU input load is an explicit
out-of-image fixture step. No value producer, CRT, whole Performer or source
binding claim. Every path must execute one +60 write, even unchanged maximum.
"""
from pathlib import Path
import sys,json,hashlib,itertools,struct
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn import UC_HOOK_MEM_WRITE
from unicorn import x86_const as X
ORIGINAL=nm.original
def run(image,pc,start,end,prior,candidate):
 nm.original=image;m=nm.Machine(pc);p=m.alloc(0x80);number=m.alloc(4);load=m.alloc(8)
 m.w(p+0x60,nm.f32(prior));m.w(number,nm.f32(candidate));m.u.mem_write(load,b'\xd9\x05'+struct.pack('<I',number))
 m.u.reg_write(X.UC_X86_REG_FPCW,nm.PRECISIONS[pc]);m.u.reg_write(X.UC_X86_REG_FPSW,0)
 m.u.emu_start(load,load+6);m.u.reg_write(X.UC_X86_REG_EBP,p);m.u.reg_write(X.UC_X86_REG_ESP,nm.STACK-0x100)
 writes=[]
 def write(u,kind,address,size,value,_):
  if address==p+0x60:writes.append([size,value])
 m.u.hook_add(UC_HOOK_MEM_WRITE,write);m.u.emu_start(start,end,count=100)
 assert m.u.reg_read(X.UC_X86_REG_EIP)==end
 expected=nm.f32(prior if candidate<prior else candidate)
 assert m.r(p+0x60)==expected and writes==[[4,expected]],(pc,prior,candidate,writes)
 return {'value':m.r(p+0x60),'writes':writes},m.visited
def main():
 rows=[];pins={}
 for pc,path,prior,candidate in itertools.product(nm.PRECISIONS,['primary','defender'],[0.0,1.0,3.0],[0.0,0.5,3.0,9.0]):
  start,end=(0x57e57a,0x57e58f)if path=='primary'else(0x57e62e,0x57e643)
  a,pa=run(ORIGINAL,pc,start,end,prior,candidate);b,pb=run(nm.modified,pc,start,end,prior,candidate)
  assert a==b and pa==pb;pins.update(pa)
  rows.append({'precision':pc,'path':path,'prior_bits':nm.f32(prior),'candidate_bits':nm.f32(candidate),'output':a})
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 out={'schema':1,'scope':__doc__,'paired_cases':len(rows),'native_executions':2*len(rows),'successful_child_seams':[],
  'cases':rows,'executed_instruction_pins':{f'{va:08x}':raw.hex()for va,raw in sorted(pins.items())}}
 p=HERE/'native-maximum-store.json'
 if '--write'in sys.argv:
  with p.open('x')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert json.loads(p.read_bytes())==out
 print(json.dumps({'paired_cases':len(rows),'native_executions':2*len(rows),'executed_pins':len(pins),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
