"""Original GTID registry operations, unmodified x86, no live process or game writes."""
from pathlib import Path
import hashlib,json,struct,sys
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/engine-baseline'))
sys.path.insert(0,str(ROOT.parent/'reconstruction/tools/python-packages'))
from audit_baseline import PE
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
P=PE(ROOT.parent/'AA.exe');V=PE(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe')
H=lambda b:hashlib.sha256(b).hexdigest()
assert H(P.b)=='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
assert H(V.b)=='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'

class Machine:
 def __init__(self,initial):
  self.u=Uc(UC_ARCH_X86,UC_MODE_32);self.u.mem_map(P.base,(P.image_size+4095)&~4095)
  for vs,r,size,off in P.sections:self.u.mem_write(P.base+r,P.b[off:off+size])
  self.u.mem_map(0,4096);self.u.mem_map(0x2000000,0x80000);self.u.mem_map(0x2100000,0x20000);self.u.mem_map(0x3000000,0x20000)
  self.manager=0x2000000;self.bits=0x2070000;self.stack=0x3010000;self.stop=0x301f000;self.out=0x301e000;self.keys={};self.actor={};self.error=None
  self.w(self.manager+0x60010,self.bits);self.w(self.manager+0x60018,2048);self.w(0xb364c0,self.manager)
  self.u.hook_add(UC_HOOK_CODE,self.hook);self.call(0x4eff60,[])
  self.w(self.manager+0x60008,initial.get('next_slot',1));self.w(self.manager+0x6000c,initial.get('highest_slot',0)&0xffffffff)
  for slot,generation in initial.get('generations',[]):self.u.mem_write(self.manager+4+slot*2,struct.pack('<H',generation))
  for e in initial.get('entries',[]):
   ptr=self.object(e['key'],e['is_actor']);id=e['id'];slot=id&65535;self.w(ptr+20,id);self.w(self.manager+0x20004+4*slot,ptr);self.w(self.bits+(slot>>5)*4,self.r(self.bits+(slot>>5)*4)|(1<<(slot&31)))
  self.w(self.manager+0x60004,len(initial.get('entries',[])))
 def w(self,p,v):self.u.mem_write(p,struct.pack('<I',v))
 def r(self,p):return struct.unpack('<I',self.u.mem_read(p,4))[0]
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+4*i)
 def ret(self,value=0,pop=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop)
 def object(self,key,is_actor=True):
  if key not in self.keys:
   ptr=0x2100000+len(self.keys)*0x100;self.keys[key]=ptr;self.actor[ptr]=is_actor;self.w(ptr,0x211f000);self.w(0x211f000+0xb4,0x301f100)
  return self.keys[key]
 def text(self,p):
  b=bytearray()
  while True:
   pair=bytes(self.u.mem_read(p,2));p+=2
   if pair==b'\0\0':break
   b.extend(pair)
  return b.decode('utf-16-le')
 def hook(self,u,pc,size,user):
  if pc==0x301f100:self.ret(int(self.actor[self.u.reg_read(UC_X86_REG_ECX)]))
  elif pc==0x4f0110:
   message=self.text(self.arg());self.error='Occupied' if 'already' in message else 'GenerationMismatch';u.emu_stop()
  elif pc==0x404f60:self.error='Exhausted';self.ret()
 def call(self,pc,args):
  self.error=None;self.w(self.stack,self.stop)
  for i,v in enumerate(args):self.w(self.stack+4+i*4,v)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.manager)
  self.u.emu_start(pc,self.stop,count=2000000)
  if self.error not in ['Occupied','GenerationMismatch']:assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
  return self.u.reg_read(UC_X86_REG_EAX)
 def snapshot(self):
  generations=struct.unpack('<65536H',self.u.mem_read(self.manager+4,0x20000));entries=struct.unpack('<65536I',self.u.mem_read(self.manager+0x20004,0x40000));bits=struct.unpack('<2048I',self.u.mem_read(self.bits,8192));reverse={p:k for k,p in self.keys.items()};high=self.r(self.manager+0x6000c)
  return {'active':self.r(self.manager+0x60004),'next_slot':self.r(self.manager+0x60008),'highest_slot':high if high<0x80000000 else high-0x100000000,'generations':[[i,g] for i,g in enumerate(generations) if g],'entries':[{'slot':i,'id':self.r(p+20),'key':reverse[p],'is_actor':self.actor[p]} for i,p in enumerate(entries) if p],'occupied':[[i,b] for i,b in enumerate(bits) if b]}
 def operation(self,op):
  kind=op['op'];result=None
  if kind=='allocate':
   ptr=self.object(op['key'],op['is_actor']);self.w(self.out,0xdeadbeef);self.call(0x4efdb0,[self.out,ptr]);result=self.r(self.out)
  elif kind=='seed':self.call(0x4efb70,[op['id']])
  elif kind=='register':self.call(0x4f0190,[self.object(op['key'],op['is_actor']),op['id']])
  elif kind=='remove':self.call(0x4efec0,[self.keys[op['key']]])
  elif kind=='lookup':
   ptr=self.call(0x4efcf0,[op['id']]);result=next((k for k,p in self.keys.items() if p==ptr),None)
  elif kind=='swap':self.call(0x4efb90,[self.keys[op['a']],self.keys[op['b']],0])
  elif kind=='enumerate':
   self.w(self.out,0);result=[]
   while True:
    ptr=self.call(0x4efbe0,[self.out])
    if not ptr:break
    result.append({'id':self.r(self.out),'key':next(k for k,p in self.keys.items() if p==ptr)})
  else:raise ValueError(kind)
  return {'operation':op,'result':result,'error':self.error,'state':self.snapshot()}

def main():
 cases=[]
 sequences=[('fresh-and-stale',{},[
  {'op':'allocate','key':10,'is_actor':True},{'op':'allocate','key':11,'is_actor':False},
  {'op':'lookup','id':1},{'op':'lookup','id':2},{'op':'lookup','id':65537},
  {'op':'remove','key':10},{'op':'lookup','id':1},{'op':'allocate','key':12,'is_actor':True},
  {'op':'enumerate'},{'op':'remove','key':11},{'op':'remove','key':12},{'op':'enumerate'}]),
 ('wrap-and-occupied',{'next_slot':65535,'highest_slot':65535,'generations':[[65535,7]],'entries':[{'id':7*65536+65535,'key':21,'is_actor':True},{'id':1,'key':22,'is_actor':True}]},[
  {'op':'allocate','key':23,'is_actor':True},{'op':'enumerate'},{'op':'remove','key':21},{'op':'lookup','id':7*65536+65535}]),
 ('saved-and-swap',{},[
  {'op':'seed','id':0xffffffff},{'op':'register','id':0xffffffff,'key':30,'is_actor':True},
  {'op':'allocate','key':31,'is_actor':True},{'op':'swap','a':30,'b':31},{'op':'lookup','id':0xffffffff},
  {'op':'remove','key':31},{'op':'enumerate'},{'op':'remove','key':30}]),
 ('load-errors',{},[
  {'op':'register','id':0x12340005,'key':40,'is_actor':True},{'op':'seed','id':0x12340005},
  {'op':'register','id':0x12340005,'key':40,'is_actor':True},{'op':'register','id':0x12340005,'key':41,'is_actor':True}]),
 ('generation-overflow',{'next_slot':65535,'highest_slot':0,'generations':[[65535,65535]]},[
  {'op':'allocate','key':50,'is_actor':True},{'op':'lookup','id':0xffffffff},{'op':'remove','key':50},{'op':'lookup','id':0xffffffff},{'op':'allocate','key':51,'is_actor':True}])]
 for name,initial,ops in sequences:
  m=Machine(initial);cases.append({'name':name,'initial':initial,'steps':[m.operation(op) for op in ops]})
 m=Machine({});m.w(m.manager+0x60004,32768);m.w(m.out,0xdeadbeef);m.call(0x4efdb0,[m.out,m.object(99)]);assert m.r(m.out)==0 and m.error=='Exhausted'
 ranges={}
 for name,a,z in [('seed',0x4efb70,0x4efb86),('swap',0x4efb90,0x4efbd8),('iterate',0x4efbe0,0x4efc42),('lookup_actor',0x4efcf0,0x4efd2c),('allocate',0x4efdb0,0x4efeba),('remove',0x4efec0,0x4eff5f),('reset',0x4eff60,0x4effe1),('register_saved',0x4f0190,0x4f0230)]:
  b=P.range(a,z-a);assert b==V.range(a,z-a);ranges[name]={'start':hex(a),'end':hex(z),'sha256':H(b)}
 report={'schema':1,'passed':True,'original_sha256':H(P.b),'v14_sha256':H(V.b),'generator_sha256':H(Path(__file__).read_bytes()),'cases':cases,'operation_count':sum(len(c['steps']) for c in cases),'capacity_gate':{'active_count':32768,'returned_id':0,'error':'Exhausted','scope':'isolated count-gate fixture; not32768 native constructor executions'},'ranges':ranges,'seams':['Object IsActor virtual predicate is a fixture leaf.','Loading exception is captured at its boundary before mutation; formatting/throw machinery omitted.','Object pointers are stable fixture keys; no component constructors or live process.']}
 p=Path(__file__).with_suffix('.json');p.write_text(json.dumps(report,indent=2)+'\n');print(p,H(p.read_bytes()),report['operation_count'])
if __name__=='__main__':main()
