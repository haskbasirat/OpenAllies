"""Execute original578140 finite status comparisons and conditional Mind query."""
from pathlib import Path
import sys,json,struct,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];sys.path.insert(0,str(ROOT/'research/phase7-actors'));from actor_states import Machine as Base,P,V,H
from unicorn.x86_const import *
F=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
class Machine(Base):
 def __init__(self,c):
  super().__init__();self.c=c;self.calls=0;self.org=0x2085000;self.mind=0x2086000
  self.w(self.org+4,self.thing);self.w(self.thing+0x70,self.mind if c['mind'] else 0);self.w(self.mind,0x2087000);self.w(0x208705c,0x301f700)
  self.w(self.mind+0x28,c['words'][0]);self.w(self.mind+0x2c,c['words'][1]);self.w(self.org+0x60,c['word60']);self.w(self.org+0x5c,c['word5c']);self.w(self.org+0xb8,c['threshold'])
 def hook(self,u,pc,size,data):
  if pc==0x301f700:self.calls+=1;self.ret(int(self.c['exhausted']))
  else:super().hook(u,pc,size,data)
 def run(self):self.invoke(0x578140,self.org);return {'status':self.u.reg_read(UC_X86_REG_EAX),'calls':self.calls}
def main():
 rows=[];observed={}
 for mind,words,timer,morale,threshold,exhausted in itertools.product([False,True],[[0,0],[1,0],[0,1]],[-1,0,1],[0,1,20,21],[0,20],[False,True]):
  if not mind and timer>0 and morale!=0 and morale<=threshold:continue
  c={'mind':mind,'words':words,'word60':F(timer),'word5c':F(morale),'threshold':F(threshold),'exhausted':exhausted};m=Machine(c);rows.append({'input':c,'output':m.run()});observed.update(m.observed)
 for a,b in observed.items():assert b==P.range(a,len(b))==V.range(a,len(b))
 out={'schema':1,'cases':rows,'source_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(observed.items())],'scope':'Complete578140 in finite inputdomain. Original conditional Mind+5C virtual isfixturepredicate; its concrete Mind exhausted-stack implementation isseparatelyownedphase7-mind. NullMind paths that woulddereference areexcludedfromnativeexecution andexpliciterrorsinRust.'}
 p=HERE/'native-organization-status.json';p.write_text(json.dumps(out,indent=2));print(json.dumps({'cases':len(rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
