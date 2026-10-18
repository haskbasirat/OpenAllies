"""Execute unchanged native overlap dispatcher, footprint constructor and SAT.
No geometry/branch/FPU leaves are hooked. Point origin global is the PE zero state.
"""
from pathlib import Path
import json,math,random,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified,sha
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
F=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
FLOAT=lambda x:struct.unpack('<f',struct.pack('<I',x))[0]
def shape(kind,radius,bounds):return dict(kind=kind,radius_bits=F(radius),bounds=list(map(F,bounds)))
SHAPES=[shape(0,0,[0]*4),shape(1,1.25,[-1.25,-1.25,1.25,1.25]),shape(2,math.sqrt(2),[-1,-1,1,1]),shape(3,math.sqrt(5),[-2,-1,2,1])]
def placement(s,position,facing):return dict(shape=s,position=list(map(F,position)),facing=list(map(F,facing)))
class Machine(Base):
 def __init__(self,cw):super().__init__(cw);self.executed={};self.u.hook_add(UC_HOOK_CODE,self.hook)
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.executed[pc]=raw.hex()
 def put(self,a,*words):self.u.mem_write(a,struct.pack('<'+'I'*len(words),*[v&0xffffffff for v in words]))
 def descriptor(self,a,saddr,value):
  s=value['shape'];self.put(saddr,s['kind'],s['radius_bits'],*s['bounds']);self.put(a,saddr,*value['facing'],*value['position'],0)
 def overlap(self,a,b):
  self.reset(0);self.descriptor(self.STATE,self.STATE+0x100,a);self.descriptor(self.STATE+0x200,self.STATE+0x300,b)
  self.w(self.STACK+4,self.STATE+0x200);self.u.emu_start(0x46ef30,self.STOP,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP
  assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  return bool(self.u.reg_read(UC_X86_REG_EAX)&255)
 def footprint(self,a):
  self.reset(0);self.put(self.STATE+0x100,*a['position']);self.put(self.STATE+0x200,*a['facing'])
  width=F(FLOAT(a['shape']['bounds'][2])*2);length=F(FLOAT(a['shape']['bounds'][3])*2)
  self.put(self.STACK,self.STOP,self.STATE+0x100,self.STATE+0x200,length,width,0)
  self.u.emu_start(0x46e7d0,self.STOP,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+24
  return [list(struct.unpack('<2I',self.u.mem_read(self.STATE+i*8,8)))for i in range(3)]
 def sat(self,a,b):
  self.reset(0);self.put(self.STATE,*sum(a,[]));self.put(self.STATE+0x100,*sum(b,[]));self.w(self.STACK+4,self.STATE+0x100)
  self.u.emu_start(0x46e810,self.STOP,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  return bool(self.u.reg_read(UC_X86_REG_EAX)&255)

def main():
 cases=[];footprints=[];sat=[];instructions={};randomgen=random.Random(0x46ef30)
 positions=[[0,0],[.5,.5],[1,0],[2,0],[2.0001,0],[2.00001,0],[1.9999,0],[3,0],[0,2],[2,2],[4,0],[-4,0],[-2,-2],[.00001,0],[4.9999,-.5],[-1.1,2.3]]
 faces=[[0,-1],[1,0],[.6,-.8],[-.8,.6],[-0.,-1.]]
 for precision,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  m=Machine(cw)
  for s in SHAPES:
   for t in SHAPES:
    for index,fa in enumerate(faces):
     for fb in [fa,faces[(index+1)%len(faces)]]:
      for position in positions:
       a=placement(s,[0,0],fa);b=placement(t,position,fb)
       cases.append(dict(precision=precision,a=a,b=b,result=m.overlap(a,b)))
  for i in range(800):
   angle=randomgen.uniform(-math.pi,math.pi);angle2=randomgen.uniform(-math.pi,math.pi)
   a=placement(randomgen.choice(SHAPES),[randomgen.uniform(-5,5),randomgen.uniform(-5,5)],[math.sin(angle),-math.cos(angle)])
   b=placement(randomgen.choice(SHAPES),[randomgen.uniform(-5,5),randomgen.uniform(-5,5)],[math.sin(angle2),-math.cos(angle2)])
   cases.append(dict(precision=precision,a=a,b=b,result=m.overlap(a,b)))
  #Probe contact at adjacent binary32 positions, including parallel physical
  #boxes represented by opposite facing vectors (native rotated dispatch).
  contact=random.Random(0x46e810)
  for i in range(180):
   angle=contact.uniform(-math.pi,math.pi);facing=[math.sin(angle),-math.cos(angle)]
   extent=contact.uniform(.1,3);width=contact.uniform(.1,2);s=shape(3,math.hypot(extent,width),[-width,-extent,width,extent])
   a=placement(s,[1.234567,-2.345678],facing)
   x=FLOAT(a['position'][0])+2*FLOAT(s['bounds'][3])*FLOAT(a['facing'][0]);y=FLOAT(a['position'][1])+2*FLOAT(s['bounds'][3])*FLOAT(a['facing'][1])
   b=placement(s,[x,y],[-v for v in facing])
   for delta in [-2,-1,0,1,2]:
    moved=dict(b);moved['position']=[(b['position'][0]+delta)&0xffffffff,b['position'][1]]
    cases.append(dict(precision=precision,a=a,b=moved,result=m.overlap(a,moved)))
  for exponent in [-50,-20,-5,5,20,50]:
   for facing in faces:
    magnitude=2.**exponent
    a=placement(shape(3,magnitude*2,[-magnitude*.7,-magnitude*1.1,magnitude*1.3,magnitude*1.7]),[magnitude*1.234567,magnitude*-2.345678],facing)
    footprints.append(dict(precision=precision,placement=a,result=m.footprint(a)))
  #Authored asymmetric bounds are not silently centered or normalized.
  for bounds in [[-.7,-1.2,1.3,2.5],[-2,-.3,2,.3],[0,0,1,2],[-1e-20,-2e-20,1e-20,2e-20]]:
   for fa in faces+[[0,0],[.3,-.4]]:
    a=placement(shape(3,4,bounds),[1.234567,-2.345678],fa)
    footprint=m.footprint(a);footprints.append(dict(precision=precision,placement=a,result=footprint))
    for fb in faces:
     b=placement(SHAPES[3],[1.4,-.12345],fb);second=m.footprint(b)
     cases.append(dict(precision=precision,a=a,b=b,result=m.overlap(a,b)))
     sat.append(dict(precision=precision,a=footprint,b=second,result=m.sat(footprint,second)))
  instructions.update(m.executed)
 ranges=[]
 for start in sorted(instructions):
  code=instructions[start];end=start+len(bytes.fromhex(code))
  if ranges and int(ranges[-1]['end'],16)==start:ranges[-1]['hex']+=code;ranges[-1]['end']=hex(end)
  else:ranges.append(dict(start=hex(start),end=hex(end),hex=code))
 for r in ranges:r['sha256']=sha(bytes.fromhex(r['hex']))
 witnesses={}
 for c in cases:
  key=json.dumps([c['a'],c['b']],sort_keys=True);witnesses.setdefault(key,{})[c['precision']]=c['result']
 precision_witnesses=[dict(input=json.loads(k),results=v)for k,v in witnesses.items()if len(set(v.values()))>1]
 report=dict(schema=1,original_sha256=sha(original.b),v14_sha256=sha(modified.b),ranges=ranges,cases=cases,footprints=footprints,sat=sat,precision_witnesses=precision_witnesses,
  seams=['No native arithmetic, comparator, branch or constructor call is hooked.','PC24/53/64 round-to-nearest explicitly selected; global point equality origin B279C0/B279C4 uses PE zero-initialized values.','Footprint helper only supports the mode0 reached by46EF30; other standalone constructor modes are outside this API.','Finite supported shape kinds0..3 only; no whole-world scheduling or collision ownership claim.'])
 path=HERE/'native-overlap.json';path.write_text(json.dumps(report,separators=(',',':'))+'\n');print(json.dumps(dict(cases=len(cases),footprints=len(footprints),sat=len(sat),precision_witnesses=len(precision_witnesses),sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
