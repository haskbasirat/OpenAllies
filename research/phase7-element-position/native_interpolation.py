"""Original57C080 interpolation math/loop plus51D510 Manhattan test.
Collision query construction/binding552DD0/5531F0, position setter552250 and
probe553BA0 are required world seams. Actual probe masks after OR0x400 and every
candidate coordinate are captured. World movement predicates, Mind point and
relocation use explicit leaves. No positive-step iteration cap is added.
"""
from pathlib import Path
import copy,json,struct,sys,random
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from native_position import *
CACHE,VALUE,FLOATRET=0x4700000,0x4710000,0x4720000
class InterpolationMachine(PositionMachine):
 def __init__(self,c):
  super().__init__(c);self.probes=0;self.w(0xb39ab0,CACHE);self.w(CACHE+0x48,VALUE);self.w(VALUE,c['speed_multiplier']);self.w(ORG+0xd8,c['local_speed']);self.w(WORLD+0xe8,c['timestep']);self.w(0xb3d6c0,1);self.w(VT+0x80,STUB+32)
  self.u.mem_write(VALUE+8,struct.pack('<Q',c['minimum_speed_bits']));self.u.mem_write(FLOATRET,b'\xdd\x05'+struct.pack('<I',VALUE+8)+b'\xc3')
 def hook(self,u,pc,size,data):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  cx=u.reg_read(UC_X86_REG_ECX);s=u.reg_read(UC_X86_REG_ESP)
  if pc==0x57c080:return
  if pc==0x4be0f0:self.event('minimum',[]);u.reg_write(UC_X86_REG_EIP,FLOATRET)
  elif pc==0x4cede0:self.event('moving',[identity(ACTORS,cx)]);self.ret(int(self.c['moving']))
  elif pc==STUB+32:
   self.event('mind_point',[identity(MINDS,cx)]);out=self.r(s+4)
   for i,v in enumerate(self.c['mind_point']):self.w(out+i*4,v)
   self.ret(out,4)
  elif pc==0x552dd0:self.u.mem_write(cx,bytes(0x58));self.ret(cx)
  elif pc==0x5531f0:
   assert self.r(s+8)==1;self.event('prepare',[identity(ACTORS,self.r(s+4)),0x400]);self.w(cx+0x38,self.c['query_flags'][0]);self.w(cx+0x3c,self.c['query_flags'][1]);self.ret(pop=8)
  elif pc==0x552250:self.w(cx+0x4c,self.r(s+4));self.w(cx+0x50,self.r(s+8));self.ret(pop=8)
  elif pc==0x553ba0:
   out=self.r(s+4);query=self.r(s+8);self.event('probe',[[self.r(query+0x4c),self.r(query+0x50)],[self.r(query+0x38),self.r(query+0x3c)]]);values=self.c['blocked'];blocked=values[min(self.probes,len(values)-1)];self.probes+=1;self.w(out,int(blocked));self.ret(out,8)
  else:super().hook(u,pc,size,data)
 def run_interpolation(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[self.c['precision']]);self.u.reg_write(UC_X86_REG_ECX,ORG)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x57c080,self.STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  assert[self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  assert not self.c['mutations'],self.c['name'];return {'complete':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'goal':[self.r(ORG+0xbc),self.r(ORG+0xc0)],'events':self.events}
def interpolation_cases():
 base=copy.deepcopy(cases()[1]);base.update(goal=[bits(30),bits(40)],local_speed=bits(5),speed_multiplier=bits(2),minimum_speed_bits=struct.unpack('<Q',struct.pack('<d',3.))[0],timestep=bits(.2),moving=False,mind_point=[bits(100),bits(100)],query_flags=[0x12,0x30001],blocked=[False])
 rows=[]
 def add(name,change):c=copy.deepcopy(base);c['name']=name;change(c);rows.append(c)
 for goal in [[10,20],[10,20.00005],[10,20.0002],[11,20],[30,40],[-.11111,-.333333],[-100,2.5555]]:
  for blocked in [[False],[True,False],[True],[True,True,False]]:add(f'goal{goal}_blocked{blocked}',lambda c,g=goal,b=blocked:c.update(goal=list(map(bits,g)),blocked=b))
 for speed in [1,4.7,200]:
  for minimum in [0,3.1111,100]:add(f'speed{speed}_min{minimum}',lambda c,s=speed,m=minimum:c.update(local_speed=bits(s),minimum_speed_bits=struct.unpack('<Q',struct.pack('<d',m))[0]))
 for point in [[10,20],[10.0001,20],[10.0003,20],[50,50]]:add(f'moving_point{point}',lambda c,p=point:c.update(moving=True,mind_point=list(map(bits,p))))
 add('minimum_changes_goal',lambda c:c.update(mutations=[{'event':'minimum','goal':list(map(bits,[11,21]))}],local_speed=bits(200)))
 add('probe_changes_goal',lambda c:c.update(blocked=[True],mutations=[{'event':'probe','goal':list(map(bits,[1,2]))}]))
 add('moving_changes_selected',lambda c:c.update(moving=True,mutations=[{'event':'moving','selected_member':3}]))
 for p in ['Single24','Double53','Extended64']:
  for c in rows:yield {**copy.deepcopy(c),'name':c['name']+'_'+p,'precision':p}
def main():
 rows=[];visited={}
 for c in interpolation_cases():
  try:m=InterpolationMachine(c);result=m.run_interpolation()
  except Exception:print(c['name']);raise
  rows.append({'input':c,'output':result});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
 d={'schema':1,'method':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'cases':rows}
 p=HERE/'native-interpolation.json';p.write_text(json.dumps(d,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
