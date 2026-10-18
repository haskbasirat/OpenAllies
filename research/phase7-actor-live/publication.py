"""Original full4CF300/4EBA40 with actual Properties581880 ground leaf."""
from pathlib import Path
import sys,json,struct,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];sys.path.insert(0,str(ROOT/'research/phase4-actors'))
from trace_components import Machine as Base,P,V,H
from unicorn.x86_const import *
F=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
class Machine(Base):
 def __init__(self,c):
  super().__init__({**c,'mode':'full'});self.c=c;self.world=0x2040000;self.settings=0x2041000;self.record=0x2042000;self.owner=0x2043000
  self.w(0xb36534,self.world);self.w(self.world+0x28,self.settings);self.w(self.settings+0xa8,c['mapmode']);self.w(self.record+0x34,123);self.w(self.actor+0xe8,self.owner if c['owner']else 0);self.w(self.definition+0x5b4,F(c['city_value']))
  self.w(self.definition+0x1d4,0x2050000 if c['properties']else 0)
  for i,value in enumerate(c['properties']):self.w(0x2050000+i*8,value if value is not None else 0);self.w(0x2050004+i*8,0x2050000+(i+1)*8 if i+1<len(c['properties'])else 0)
  if 6 in c['present']:
   self.w(0x2010604,self.actor);self.w(0x202060c,0x581880)
 def hook(self,u,pc,size,data):
  self.visited[pc]=size
  if pc==0x4eba40:return
  if pc==0x4eb500:self.events.append({'kind':'ground'});self.ret(pop=4);return
  if pc==0x581880:self.events.append({'kind':'properties'});return
  if pc==0x5817a0:self.events.append({'kind':'add_property','property':self.arg()or None,'property_kind':self.arg(1),'source':self.arg(2)});self.ret(pop=12);return
  if pc==0x4e6180:
   self.events.append({'kind':'owner','owner':1});
   if self.c['owner_changes_city']:self.w(self.definition+0x174,1<<29)
   self.ret(pop=4);return
  if pc==0x5644f0:self.events.append({'kind':'city','argument':self.arg(1),'zero':self.arg(2)});self.ret();return
  super().hook(u,pc,size,data)
 def run(self):
  self.w(self.stack,self.stop);self.w(self.stack+4,self.record);self.w(self.stack+8,self.c['flag']);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.actor);self.u.emu_start(0x4cf300,self.stop,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop and self.u.reg_read(UC_X86_REG_ESP)==self.stack+12
  events=self.events
  for e in events:
   if 'record'in e:e['record']=1
  return {'events':events,'present':[s for s in range(31)if self.r(self.actor+0x60+4*s)],'mask':self.r(self.definition+0x174)}
def main():
 rows=[];visited={}
 for i,(flag,mapmode,owner,city_value)in enumerate(itertools.product([0,1,256],[0,1],[False,True],[-1,0,1])):
  c={'flag':flag,'mapmode':mapmode,'owner':owner,'city_value':city_value,'owner_changes_city':i%3==0,'mask':(1<<29)if i%2 else 0,'present':[[0,2,3,4,5,6,8,9,22],[6,7,17],[]][i%3],'properties':[[4,5,4],[None],[]][i%3],'mutations':[{'kind':'post_create','slot':0,'add':[1],'remove':[3]}]if i%4==0 else[]}
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  if P.base<=a<P.base+P.image_size:raw=P.range(a,n);assert raw==V.range(a,n);windows.append({'va':hex(a),'bytes':raw.hex()})
 d={'schema':1,'cases':rows,'source_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'instructions':windows,'scope':'Full4CF300/4EBA40, actual ThingRET8,45D6A0,Properties581880 source list and45EED0. Ground deformation, owner4E6180, City5644F0 and5817A0 are observed required boundaries. Other component virtuals are mutation-aware fixtures, not claims of their implementation. Null property forwarding is observed; actual insertion must reject unbound null rather than omit.'}
 path=HERE/'native-publication.json';path.write_text(json.dumps(d,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
