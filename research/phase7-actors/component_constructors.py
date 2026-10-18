"""Execute original ground component constructors and their native helper chain.

Only successful heap allocation and Character's resolved view selection are
seams. Numeric definition getters and Organization layout copy execute natively.
Unwritten object bytes stay poisoned and are recorded as uninitialized.
"""
from pathlib import Path
import hashlib,json,random,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-mapworld'))
from native_load_oracle import Machine as Base,P,V,H,W
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
F=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
CTOR={'Body':(0x561960,0x44),'Mover':(0x576920,0x8c),'Character':(0x563670,0x30),'Organization':(0x579bd0,0xdc),'Element':(0x570930,0x34),'Economy':(0x56f370,0x54),'Detection':(0x56eab0,0x18)}

class Machine(Base):
 def __init__(self,kind,case,pc):
  super().__init__(b'');self.kind=kind;self.case=case;self.entry,self.size=CTOR[kind]
  self.component=0x2090000;self.heap=0x20c0000;self.written=set();self.executed={};self.allocations=[]
  self.u.mem_write(self.component,bytes([0xcd])*self.size)
  self.w(self.thing+4,self.definition);self.w(0xb36534,self.world)
  for offset,name in [(0x284,'health'),(0x288,'defense'),(0x228,'speed'),(0x230,'acceleration'),(0x234,'turn_radius'),(0x238,'turn_type'),(0x308,'organization_value'),(0x1cc,'detection_radius')]:self.w(self.definition+offset,case[name]&0xffffffff)
  self.w(self.world+0xe0,case['world_time']);self.w(0xb3a660,0x20a4000);self.w(0x20a4028,case['resource_count'])
  for i,v in enumerate(case['position']):self.w(self.thing+0x20+i*4,v)
  self.layout=0x20a0000;self.layoutdef=0x20a1000;self.parameters=0x20a2000
  self.w(self.definition+0x2f0,self.layout);self.w(self.layout+8,self.layoutdef);self.w(self.layout+0xc,self.parameters)
  self.w(self.layoutdef+0x20,len(case['layout_parameters']));self.w(self.layoutdef+0x2c,case['layout_slots'])
  for i,p in enumerate(case['layout_parameters']):self.w(self.parameters+i*4,p)
  self.view=0x20a3000;self.views=0x20a3100;self.w(self.view+0xc,self.views);self.w(self.view+0x10,len(case['character_animations']))
  for i,p in enumerate(case['character_animations']):self.w(self.views+i*4,p)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[pc])
  self.u.hook_add(UC_HOOK_MEM_WRITE,self.write_hook)
 def write_hook(self,u,access,address,size,value,user):
  for offset in range(max(0,address-self.component),min(self.size,address+size-self.component)):self.written.add(offset)
 def hook(self,u,pc,size,user):
  if P.base<=pc<P.base+P.image_size:self.executed[pc]=bytes(u.mem_read(pc,size))
  if pc==0x6f7810:
   size=self.arg();address=self.heap;self.heap+=(max(size,1)+15)&~15;assert self.heap<0x2100000
   self.u.mem_write(address,bytes([0xa5])*max(size,1));self.allocations.append({'bytes':size,'address':address});self.ret(address)
  elif pc==0x4d0720:self.ret(self.view)
  else:super().hook(u,pc,size,user)
 def run(self):
  self.w(self.stack,self.stop);self.w(self.stack+4,self.thing);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.component)
  try:self.u.emu_start(self.entry,self.stop,count=100000)
  except Exception as e:raise RuntimeError((self.kind,hex(self.u.reg_read(UC_X86_REG_EIP)))) from e
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop and self.u.reg_read(UC_X86_REG_ESP)==self.stack+8
  assert self.u.reg_read(UC_X86_REG_EAX)==self.component
  fields={str(o):self.r(self.component+o) for o in range(0,self.size,4) if o+4<=self.size and all(x in self.written for x in range(o,o+4))}
  byte_fields={str(o):self.u.mem_read(self.component+o,1)[0] for o in sorted(self.written) if not all(x in self.written for x in range(o//4*4,o//4*4+4))}
  vectors={}
  if self.kind=='Organization':
   vectors['layout_parameters']=[self.r(self.r(self.component+0x1c)+4*i) for i in range(self.r(self.component+0x20))]
   vectors['vector_78_capacity_values']=[list(struct.unpack('<2I',self.u.mem_read(self.r(self.component+0x78)+8*i,8))) for i in range(self.r(self.component+0x80))]
  if self.kind=='Economy':
   for offset in [0x14,0x24,0x34,0x44]:vectors[f'resources_{offset:x}']=[self.r(self.r(self.component+offset+4)+4*i) for i in range(self.r(self.component+offset+8))]
  if self.kind=='Element':vectors['vector_28_capacity_values']=[list(struct.unpack('<7I',self.u.mem_read(self.r(self.component+0x28)+28*i,28))) for i in range(self.r(self.component+0x30))]
  return {'initialized_words':fields,'initialized_bytes':byte_fields,'uninitialized_byte_offsets':[o for o in range(self.size) if o not in self.written], 'vectors':vectors,'allocation_sizes':[a['bytes'] for a in self.allocations]}

def cases():
 result=[];rng=random.Random(0x576920)
 for index in range(16):
  result.append({'health':F([1,25,100,0.5][index%4]),'defense':F([-3,0,12.25,1.333][index%4]),'speed':F(rng.uniform(.1,100)),'acceleration':F(rng.uniform(0,20)),
   'turn_radius':F(rng.uniform(.1,30)),'turn_type':[0,1,2,-1][index%4], 'organization_value':F(rng.uniform(0,30)),'world_time':F(index*.333),
   'position':[F(rng.uniform(-100,100)) for _ in range(3)],'layout_parameters':[0x12340000+i for i in range(index%5)],'layout_slots':index%6,
   'character_animations':[0x76540000+i for i in range(index%3)],'detection_radius':F(index*1.25),'resource_count':index%6})
 return result
def main():
 rows=[];observed={}
 for pc in ['Single24','Double53','Extended64']:
  for kind in CTOR:
   for case in cases():
    machine=Machine(kind,case,pc);output=machine.run();observed.update(machine.executed)
    rows.append({'kind':kind,'precision':pc,'input':case,'output':output})
 windows=[]
 for pc,data in sorted(observed.items()):
  assert data==V.range(pc,len(data));windows.append({'address':hex(pc),'bytes':data.hex()})
 report={'scope':__doc__,'original_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'cases':rows,'observed_windows':windows,
 'limits':['Native allocation addresses/vtables are evidence, not portable runtime identities.','Character view selection is a fixture lookup; actual default animation pointer copy is native.','No component InitCreate, scheduling, actor publication or live-world success claimed by constructor fixtures.','Turn types outside0..2 leave Mover+34 uninitialized; poison must not become a new default.']}
 path=HERE/'component-constructors-native.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
