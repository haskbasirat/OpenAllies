"""Original46E150 shape projection and4EEE60/4EEEE0 target-hit surface.
Shape math executes; actor radius/height/shape virtual getters are explicit
mutable fixtures and return x87 values without forcing an early f32 store.
"""
from pathlib import Path
import copy,hashlib,json,random,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ACTOR,SHAPE,OUT,VT,RADIUS,HEIGHT,GETSHAPE,SVARS,TRAMP=range(0x4000000,0x4009000,0x1000)
bits=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
f64=lambda v:struct.unpack('<Q',struct.pack('<d',v))[0]
class Machine(Base):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=copy.deepcopy(c);self.events=[];self.visited={}
  self.u.mem_map(0x4000000,0x10000);self.w(ACTOR,VT);self.w(VT,RADIUS);self.w(VT+8,HEIGHT);self.w(VT+0x14,GETSHAPE)
  self.put(SHAPE,c['shape']['kind'],c['shape']['radius_bits'],*c['shape']['bounds']);self.put(ACTOR+0x20,*c['position']);self.put(ACTOR+0x2c,*c['facing']);self.put(0xb279c0,*c['default_point']);self.w(0xb39ab0,SVARS);self.w(SVARS+0x34,SVARS+0x100);self.w(SVARS+0x100,c['ratio'])
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,a,*v):self.u.mem_write(a,struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v)))
 def ret(self,value=0):
  s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4);self.u.reg_write(UC_X86_REG_EAX,value)
 def fp_return(self,value):
  self.u.mem_write(TRAMP+0x100,struct.pack('<Q',value));self.u.mem_write(TRAMP,b'\xdd\x05'+struct.pack('<I',TRAMP+0x100)+b'\xc3');self.u.reg_write(UC_X86_REG_EIP,TRAMP)
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.visited[pc]=raw.hex()
  if pc==RADIUS:
   self.events.append('radius')
   if self.c['mutations']:self.put(ACTOR+0x2c,*map(bits,[.3,-.7]))
   self.fp_return(self.c['radius'])
  elif pc==GETSHAPE:
   self.events.append('shape')
   if self.c['mutations']:self.put(ACTOR+0x20,*map(bits,[11.7,-8.3,4.19]));self.put(ACTOR+0x2c,*map(bits,[-.8,.6]));self.w(SVARS+0x100,bits(.37))
   self.ret(SHAPE)
  elif pc==HEIGHT:
   self.events.append('height')
   if self.c['mutations']:self.w(ACTOR+0x28,bits(999));self.w(SVARS+0x100,bits(777))
   self.fp_return(self.c['height'])
 def run(self):
  c=self.c;self.reset(0)
  if c['operation']=='shape':entry,receiver,args=0x46e150,SHAPE,[OUT,*c['facing'],*c['direction'][:2]]
  else:entry,receiver,args=0x4eeee0,ACTOR,[OUT,*c['direction']]
  self.u.reg_write(UC_X86_REG_ECX,receiver);self.put(self.STACK+4,*args)
  regs={r:0x11bb0000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
  for r,v in regs.items():self.u.reg_write(r,v)
  self.u.emu_start(entry,self.STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4+4*len(args)
  assert all(self.u.reg_read(r)==v for r,v in regs.items())
  return {'result':[self.r(OUT+i*4)for i in range(2 if c['operation']=='shape' else 3)],'events':self.events,'position':[self.r(ACTOR+0x20+i*4)for i in range(3)],'facing':[self.r(ACTOR+0x2c+i*4)for i in range(2)],'ratio':self.r(SVARS+0x100)}
def cases():
 rows=[];rng=random.Random(0x4eeee0)
 for kind in [0,1,2,3,4,0xffffffff]:
  for facing in [[0,-1],[1,0],[.6,-.8],[.2,.717]]:
   for direction in [[0,0,0],[.00001,0,0],[.0001,0,0],[3,4,99],[-1.7,2.33,-2],[1e-30,-1e-30,0]]:
    rows.append({'shape':{'kind':kind,'radius_bits':bits(2.17),'bounds':list(map(bits,[-2,-1,3,.5]))},'facing':list(map(bits,facing)),'direction':list(map(bits,direction)),'position':list(map(bits,[13.7,-2.9,1.17])),'default_point':list(map(bits,[.317,-.77])),'radius':f64(4.170000031),'height':f64(2.777777719),'ratio':bits(.33333333),'mutations':False})
 for i in range(40):
  r=lambda: rng.uniform(-100,100)
  rows.append({'shape':{'kind':rng.choice([1,2,3]),'radius_bits':bits(r()),'bounds':list(map(bits,[r(),r(),r(),r()]))},'facing':list(map(bits,[r(),r()])),'direction':list(map(bits,[r(),r(),r()])),'position':list(map(bits,[r(),r(),r()])),'default_point':[0,0],'radius':f64(r()),'height':f64(r()),'ratio':bits(r()),'mutations':i%2==0})
 return [{**copy.deepcopy(c),'precision':pc,'operation':op,'name':f'{i}_{pc}_{op}'}for pc in ['Single24','Double53','Extended64']for op in ['shape','hit']for i,c in enumerate(rows)]
def main():
 rows=[];visited={}
 for c in cases():
  try:m=Machine(c);out=m.run()
  except Exception as e:raise RuntimeError(c['name'])from e
  rows.append({'input':c,'output':out});visited.update(m.visited)
 doc={'schema':1,'passed':True,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),'method':__doc__,'cases':rows,'visited_windows':[{'address':hex(a),'bytes':b}for a,b in sorted(visited.items())]}
 (HERE/'native-surface-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
