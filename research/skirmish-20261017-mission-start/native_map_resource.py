"""Fresh managed MapResource ownership, unchanged original/v14 instructions.

Executes whole factory5A28B0, constructor5A27A0 and base5AA890 with explicit
CString, lock, heap, stream/header and manager-registration transport seams.
The actual resource allocation/constructor pointer flow and+A8 zero store run.
Separate exact native temp-mode/store and normal-return restore intervals run
with supplied valid frame inputs; this does NOT execute all of5A2B90 or certify
the omitted file loading/record lifecycle. World ctor resource+28 assignment
also runs as an isolated instruction with explicit register inputs. No original,
profile or map file is modified.
"""
from pathlib import Path
import hashlib,itertools,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn.x86_const import *
ORIGINAL=h.P

class Machine(h.Machine):
 def __init__(self,image,pc):
  h.P=ORIGINAL if image=='original' else h.V;super().__init__(pc)
  self.manager=self.alloc(0x100);self.w32(0xb36540,self.manager)
  self.lock_vt=self.alloc(16);self.w32(self.manager+0x20,self.lock_vt)
  self.stream=self.alloc(0x80);self.streamvt=self.alloc(0x40);self.w32(self.stream,self.streamvt)
  self.empty=self.alloc(0x40);self.w32(self.empty+12,5)
  self.input=self.alloc(0x80);self.w32(self.input,self.streamvt)
  self.header=self.alloc(0x68);self.resources=[];self.registered=[];self.seam_entries=set()
  def seam(a,f):self.seams[a]=f;self.seam_entries.add(a)
  def lock(m):m.event('lock');m.ret()
  def unlock(m):m.event('unlock');m.ret()
  # Fresh stub addresses are external service vtables, not patched game code.
  self.lock_entry=self.stub_next;self.stub_next+=16;self.unlock_entry=self.stub_next;self.stub_next+=16
  self.copy_entry=self.stub_next;self.stub_next+=16
  self.w32(self.lock_vt+4,self.lock_entry);self.w32(self.lock_vt+8,self.unlock_entry)
  self.w32(self.streamvt+12,self.copy_entry)
  seam(self.lock_entry,lock);seam(self.unlock_entry,unlock)
  seam(self.copy_entry,lambda m:m.ret(pop=4 if m.ecx()==self.input else 0,value=self.stream))
  seam(0x401b00,self.cstring);seam(0x5ac690,lambda m:m.ret(pop=4,value=0))
  seam(0x5b1850,self.mutex);seam(0x8584a0,lambda m:m.ret(value=m.ecx()))
  seam(0x4012c0,lambda m:m.ret(value=m.arg()))
  increment=self.stub_next;self.stub_next+=16;self.w32(0x9f01e8,increment)
  seam(increment,self.increment)
  seam(0x856370,lambda m:m.ret(value=self.stream));seam(0x5a23f0,lambda m:m.ret(pop=4,value=self.header))
  seam(0x6f7810,self.allocate);seam(0x5acec0,self.register)
 def cstring(self,m):m.w32(m.ecx(),self.empty+16);m.ret(pop=4,value=m.ecx())
 def mutex(self,m):m.w32(m.ecx(),self.lock_vt,0);m.ret(pop=8,value=m.ecx())
 def increment(self,m):p=m.arg();v=m.r32(p)+1;m.w32(p,v);m.ret(pop=4,value=v)
 def allocate(self,m):
  n=m.arg();p=m.alloc(n);m.event('allocate',n)
  if n==0xb4:self.resources.append(p)
  m.ret(value=p)
 def register(self,m):
  p=m.arg();self.registered.append(p);m.event('registered',m.r32(p+0xa8));m.ret(pop=4)
 def factory(self):
  p=self.call(0x5a28b0,0,[self.input]);assert p==self.resources[-1]==self.registered[-1]
  return {'mode_a8':self.r32(p+0xa8),'world_94':self.r32(p+0x94),
    'bound_98':self.r8(p+0x98),'flags_08':self.r32(p+8),'registered_same_allocation':True,
    'events':self.events.copy()},p
 def intervals(self,p,initial,mode):
  self.w32(p+0xa8,initial);frame=self.alloc(0x80)+0x40
  self.u.reg_write(UC_X86_REG_ESI,p);self.u.reg_write(UC_X86_REG_EDI,mode);self.u.reg_write(UC_X86_REG_EBP,frame)
  self.u.reg_write(UC_X86_REG_ESP,h.STACK)
  self.u.emu_start(0x5a2bdf,0x5a2bf7,count=20)
  assert self.u.reg_read(UC_X86_REG_EIP)==0x5a2bf7
  during=self.r32(p+0xa8);saved=self.r32(frame-0x24)
  # Exact original normal-return restore tail; frame is explicit transport.
  self.w32(frame-12,0);self.w32(frame,0x778899aa,h.STOP,0x1111,mode)
  self.w32(h.STACK,0,0,0,0,0,0x55667788,0x33445566,0x11223344)
  self.u.reg_write(UC_X86_REG_ESP,h.STACK)
  self.u.emu_start(0x5a3110,h.STOP,count=30)
  assert self.u.reg_read(UC_X86_REG_EIP)==h.STOP
  return {'initial':initial,'load_mode':mode,'saved_prior':saved,'during':during,'after':self.r32(p+0xa8)}
 def flags(self,p,initial):
  self.w32(p+8,initial);self.call(0x5aa6f0,p);after6=self.r32(p+8)
  self.u.reg_write(UC_X86_REG_ESI,p);self.u.reg_write(UC_X86_REG_EBP,self.alloc(0x80)+0x40)
  self.u.reg_write(UC_X86_REG_ESP,h.STACK)
  self.u.emu_start(0x5a30e2,0x5a310b,count=20)
  assert self.u.reg_read(UC_X86_REG_EIP)==0x5a310b
  return {'initial':initial,'after6':after6,'after1':self.r32(p+8)}

def main():
 rows=[];seams=set()
 for pc,image in itertools.product(h.PRECISION,('original','v14')):
  m=Machine(image,pc);factory,p=m.factory();seams|=m.seam_entries
  modes=[m.intervals(p,prior,mode) for prior,mode in itertools.product((0,2,0xdeadbeef),(1,2,4))]
  flags=[m.flags(p,initial)for initial in (0,1,0x18,0xdeadbeef)]
  # Ctor4F7D60 actual resource identity store; parameter and register supplied.
  world=m.alloc(0x300);m.u.reg_write(UC_X86_REG_ESI,world)
  m.u.reg_write(UC_X86_REG_EAX,p)
  m.u.emu_start(0x4f7d60,0x4f7d63,count=1)
  assert m.r32(world+0x28)==p
  rows.append({'precision':pc,'image':image,'factory':factory,'mode_intervals':modes,
    'world_constructor_same_resource':True,'flag_intervals':flags})
 executed=[{'va':hex(a),'bytes':b} for a,b in sorted(h.VISITED.items()) if a not in seams]
 metadata=[{'va':hex(a),'bytes':h.P.range(a,n).hex()} for a,n in ((0x5a3eb4,37),)]
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
   'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
   'seams_not_executed':[hex(a) for a in sorted(seams) if a<0x1000000],
   'executed_instructions':executed,'separate_static_metadata':metadata,'cases':rows}
 for x in metadata:assert ORIGINAL.range(int(x['va'],16),len(bytes.fromhex(x['bytes'])))==h.V.range(int(x['va'],16),len(bytes.fromhex(x['bytes'])))
 path=HERE/'native-map-resource.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'cases':len(rows),'intervals':sum(len(r['mode_intervals']) for r in rows),'executed_pins':len(executed),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
