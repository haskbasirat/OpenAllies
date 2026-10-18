"""Original 4096-entry trig initialization, quantized lookup, angle and rotation."""
from pathlib import Path
import hashlib, json, math, struct, sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
sha=lambda b:hashlib.sha256(b).hexdigest()
bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
number=lambda b:struct.unpack('<f',struct.pack('<I',b))[0]

class TrigMachine(Machine):
 def __init__(self,cw=0x37f):
  super().__init__(cw);self.u.mem_map(0,4096);self.heap=0x2004000
  self.u.hook_add(UC_HOOK_CODE,self.hook)
  self.output=0x2003800
  self.u.mem_write(self.STOP,b'\xd9\x1d'+struct.pack('<I',self.output))
 def hook(self,u,pc,size,user):
  if pc in [0x85da54,0x85da6e]:
   self.lookup_indices.append(u.reg_read(UC_X86_REG_EAX))
  if pc==0x6f7810:
   sp=u.reg_read(UC_X86_REG_ESP);size=self.r(sp+4);pointer=self.heap;self.heap+=size
   assert self.heap<0x2010000
   u.reg_write(UC_X86_REG_EAX,pointer);u.reg_write(UC_X86_REG_EIP,self.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4)
 def run(self,pc,ecx=0,args=(),floating=False):
  self.lookup_indices=[];self.u.context_restore(self.initial);self.w(self.STACK,self.STOP)
  for i,arg in enumerate(args):self.w(self.STACK+4+4*i,arg)
  self.u.reg_write(UC_X86_REG_ESP,self.STACK);self.u.reg_write(UC_X86_REG_ECX,ecx)
  try:self.u.emu_start(pc,self.STOP+(6 if floating else 0),count=2000000)
  except Exception as e:raise RuntimeError(f'{hex(pc)} failed at {self.u.reg_read(UC_X86_REG_EIP):08x}: {e}') from e
  return self.r(self.output) if floating else self.u.reg_read(UC_X86_REG_EAX)
 def initialize(self):
  for pc,obj in [(0x409040,0xb39a48),(0x4090e0,0xb39a60)]:self.run(pc,obj,[4096])
  return {name:list(struct.unpack('<4096I',self.u.mem_read(self.r(obj+8),4096*4))) for name,obj in [('cos',0xb39a48),('sin',0xb39a60)]}
 def angle(self,a,b):
  self.u.mem_write(0x2003000,struct.pack('<2I',*a));self.u.mem_write(0x2003010,struct.pack('<2I',*b))
  return self.run(0x445400,args=[0x2003000,0x2003010],floating=True)
 def rotate(self,a,angle,clockwise):
  self.u.mem_write(0x2003000,struct.pack('<2I',*a))
  self.run(0x4454e0 if clockwise else 0x4454a0,args=[0x2003000,angle])
  return list(struct.unpack('<2I',self.u.mem_read(0x2003000,8)))

def main():
 tables=[];lookups=[];angles=[];rotations=[]
 values=[bits(n) for n in [0.0,-0.0,1e-7,-1e-7,0.001,-0.001,0.1,-0.1,0.5,-0.5,1.0,-1.0,math.pi,-math.pi,math.tau,-math.tau,10.0,-10.0,100.0,-100.0]]
 values+= [bits(i*math.tau/4096) for i in range(-4096,4097,37)]
 vector_pairs=[([1.0,0.0],[1.0,0.0]),([1.0,0.0],[0.0,1.0]),([1.0,0.0],[0.0,-1.0]),([1.0,0.0],[-1.0,0.0]),([0.0,1.0],[0.0,-1.0]),([1.0,0.0],[2.0,2.0]),([0.0,0.0],[1.0,0.0])]
 vector_pairs += [([math.cos(a),math.sin(a)],[math.cos(b),math.sin(b)]) for a in [-3.0,-1.5,-0.5,0.0,0.1,0.9,2.0] for b in [-3.0,-1.5,-0.5,0.0,0.1,0.9,2.0]]
 for precision,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  m=TrigMachine(cw);table=m.initialize();tables.append({'precision':precision,**table})
  for value in values:
   for name,pc in [('sin',0x85daa0),('cos',0x85da80)]:
    result=m.run(pc,args=[value],floating=True)
    assert len(m.lookup_indices)==1
    lookups.append({'precision':precision,'function':name,'angle_bits':value,'result_bits':result,'index':m.lookup_indices[0],'in_table':m.lookup_indices[0]<4096})
  for a,b in vector_pairs:
   a=list(map(bits,a));b=list(map(bits,b));angles.append({'precision':precision,'from_bits':a,'to_bits':b,'result_bits':m.angle(a,b)})
  for a in [[1.0,0.0],[0.0,1.0],[0.3,-0.7],[-1.0,0.0]]:
   for angle in values[:20]:
    for clockwise in [False,True]:
     result=m.rotate(list(map(bits,a)),angle,clockwise)
     assert len(m.lookup_indices)==2
     rotations.append({'precision':precision,'from_bits':list(map(bits,a)),'angle_bits':angle,'clockwise':clockwise,'result_bits':result,'indices':m.lookup_indices.copy(),'in_table':all(i<4096 for i in m.lookup_indices)})
 assert tables[0]['sin']==tables[1]['sin']==tables[2]['sin']
 assert tables[0]['cos']==tables[1]['cos']==tables[2]['cos']
 ranges={'cos_startup':(0x9c8620,0x9c863b),'sin_startup':(0x9c8640,0x9c865b),'cos_constructor':(0x409040,0x4090b9),'sin_constructor':(0x4090e0,0x409159),'table_constructor':(0x408de0,0x408e30),'cos_instruction':(0x4090c0,0x4090c9),'sin_instruction':(0x409160,0x409169),'lookup':(0x85da10,0x85da76),'wrappers':(0x85da80,0x85dab1),'truncate':(0x96cc4c,0x96ccc1),'angle':(0x445400,0x445495),'rotations':(0x4454a0,0x445529),'asin':(0x96d780,0x96d843),'asin_helpers':(0x977837,0x9779a0)}
 for a,b in ranges.values():assert original.range(a,b-a)==modified.range(a,b-a)
 report={'schema':1,'producer_sha256':sha(Path(__file__).read_bytes()),'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'ranges':{name:{'start':hex(a),'end_exclusive':hex(b),'sha256':sha(original.range(a,b-a)),'unchanged_v14':True} for name,(a,b) in ranges.items()},'tables':tables[0], 'tables_identical_across_precision_modes':True,'lookups':lookups,'angles':angles,'rotations':rotations,'scope':'Unchanged original x86 including FSIN/FCOS/FSQRT/FPATAN in Unicorn; allocator is the only mocked function. Each call starts with a clean emulated FPU status. Not whole native process/FPU lifecycle parity.'}
 path=HERE/'native-trig-vectors.json';path.write_text(json.dumps(report,indent=2),encoding='utf-8')
 print(json.dumps({'path':str(path),'sha256':sha(path.read_bytes()),'lookup_cases':len(lookups),'angle_cases':len(angles),'rotation_cases':len(rotations)}))

if __name__=='__main__':main()
