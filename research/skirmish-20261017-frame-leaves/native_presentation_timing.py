"""Finite41C180 parity + whole41C1F0 ctor;866060 is typed f64 input seam.
Original/v14/allPC; explicit time/factor required boundaries retain stores.
Does not certify a native clock backend or enabling writer for stats byte4.
"""
from pathlib import Path
import sys,json,struct,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as n
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

def b32(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def b64(x):return struct.unpack('<Q',struct.pack('<d',x))[0]
class Machine(n.Machine):
 def __init__(self,image,p):
  super().__init__(p)
  pe=n.original if image=='original' else n.modified
  if image!='original':
   old=(n.original.image_size+4095)&~4095;new=(pe.image_size+4095)&~4095
   if new>old:self.u.mem_map(pe.base+old,new-old)
   for _,rva,raw,off in pe.sections:self.u.mem_write(pe.base+rva,pe.b[off:off+raw])
  self.stats=self.alloc(0x160);self.events=[];self.stop=None
 def capture(self,u,access,a,size,value,user):
  if a in [self.stats+0x154,self.stats+0x158,self.stats+0x15c]:self.events.append({'write':hex(a-self.stats),'bits':value&0xffffffff})
 def output(self):return {hex(o):self.rb(self.stats+o) if o==4 else self.r(self.stats+o) for o in [4,0x154,0x158,0x15c]}
 def run(self,c):
  self.wb(self.stats+4,c['enabled']);self.w(self.stats+0x154,c['mean']);self.w(self.stats+0x158,c['rate']);self.w(self.stats+0x15c,c['previous'])
  av=self.alloc(0x70);cell=self.alloc(4);self.w(av+0x68,cell);self.w(cell,c['factor']);self.w(0xb39990,av)
  def clock(m):
   m.events.append({'clock':c['time']})
   if c['failure']=='time':m.stop='time';m.u.emu_stop()
   else:m.ret_st0_f64(c['time'])
  self.seam(0x866060,clock)
  if c['failure']=='factor':
   def stop(m):m.stop='factor';m.u.emu_stop()
   self.seam(0x41c19f,stop)
  self.u.hook_add(UC_HOOK_MEM_WRITE,self.capture)
  try:self.call(0x41c180,ecx=self.stats)
  except AssertionError:
   if self.stop is None:raise
  return {'fields':self.output(),'events':self.events,'stop':self.stop}

def cases():
 out=[]
 for enabled in [0,1,128]:
  for mean,factor,delta in itertools.product([0.0,.1,-.1,.000099,.0001,.000101,1000000.0],[0.0,.25,.9,1.0,-.1,1.1],[0.0,.0001,1/60,1.0]):
   out.append({'enabled':enabled,'mean':b32(mean),'rate':0x41200000,'previous':b32(1.0),'factor':b32(factor),'time':b64(1.0+delta),'failure':None})
 for enabled,failure in itertools.product([0,1],['time','factor']):
  out.append({'enabled':enabled,'mean':b32(.1),'rate':0x41200000,'previous':0,'factor':b32(.9),'time':b64(.25),'failure':failure})
 return out
def main():
 rows=[];ctors=[];visited={}
 for image,p in itertools.product(['original','v14'],n.PRECISIONS):
  m=Machine(image,p);m.u.mem_write(m.stats,b'\xa5'*0x160);m.call(0x41c1f0,ecx=m.stats)
  assert m.r(0xb39bb8)==m.stats
  assert m.r(0xb40934)==0,'native initial process counter is not zero'
  ctors.append({'image':image,'precision':p,'fields':m.output(),'counter':m.r(0xb40934)})
  visited.update(m.visited)
  for c in cases():
   m=Machine(image,p);result=m.run(c);rows.append({'image':image,'precision':p,'input':c,'output':result});visited.update(m.visited)
 lhs=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];rhs=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert lhs==rhs
 pins=[]
 for a,b in sorted(visited.items()):
  assert n.original.range(a,len(b))==b==n.modified.range(a,len(b));assert a!=0x866060
  pins.append({'address':f'{a:08X}','bytes':b.hex()})
 out={'scope':'Whole ctor41C1F0 and finite41C180 arithmetic;866060 typed f64 input, not real timing backend; precise required time/factor cuts','producer_sha256':n.sha(Path(__file__).read_bytes()),'original_sha256':n.sha(n.original.b),'v14_sha256':n.sha(n.modified.b),'rows':rows,'constructors':ctors,'executed_instruction_pins':pins,'clock_seam':'866060 supplied f64 ST0','required_factor_entry':'41C19F before AVar manager read'}
 data=(json.dumps(out,indent=2)+'\n').encode();path=HERE/'native-presentation-timing.json'
 if '--write' in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data,'immutable fixture changed'
 print('PASS',len(rows),'timing rows,',len(ctors),'ctors,',len(pins),'executed instruction pins')
if __name__=='__main__':main()
