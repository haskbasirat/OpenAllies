"""Execute original typed integer/point field assignment."""
from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));from native_strings import P,V,H
from probe_lists import Machine as Base
from unicorn.x86_const import UC_X86_REG_FPCW
class Machine(Base):
 def hook(self,u,pc,n,z):
  if pc==0x5b3f00:self.value=[self.r(self.arg()+16+4*i)for i in range(self.words)];self.ret(pop=4)
  else:super().hook(u,pc,n,z)
 def field(self,raw,kind,pc,words):
  parser,token,node=0x2256000,0x2257000,0x2258000;self.words=words
  self.w(parser,node);self.w(parser+0x40,self.string([ord(c)for c in'field']));self.w(parser+0x38,kind)
  self.w(token+0x18,self.string([ord(c)for c in raw]));self.w(token+0x10,7);self.value=None
  self.invoke(pc,parser,[token]);return self.value

def main():
 rows=[];visited={}
 for raw in ['0','1','-1','+42','2147483647','-2147483648','0007']:
  m=Machine();value=m.field(raw,2,0x5bbe70,1);assert value==[int(raw)&0xffffffff],(raw,value);rows.append({'kind':'integer','raw':raw,'words':value});visited.update(m.visited)
 for name,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  for raw in ['0,0','1,-2','0.35,25%','-0,1.0000000596046448','+0.0001,0.1']:
   for words,kind,pc in [(2,9,0x5bbd00),(3,8,0x5bbb90)]:
    text=raw if words==2 else raw+',3.125'
    m=Machine();m.u.reg_write(UC_X86_REG_FPCW,cw);value=m.field(text,kind,pc,words);rows.append({'kind':'point'+str(words),'precision':name,'raw':text,'words':value});visited.update(m.visited)
 spans=[]
 for a,n in sorted(visited.items()):
  if not P.base<=a<P.base+P.image_size:continue
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 pins=[]
 for a,b in spans:
  raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
 doc={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'ranges':pins,'seams':['Original assignment and typed field constructor execute; successful allocation and constructed field insertion are fixture seams. C locale is supplied to native CRT.','Only canonical signed decimal i32 values are exposed in Rust; overflow, permissive suffix parsing and expressions remain explicitly unsupported.']}
 (HERE/'native-values.json').write_text(json.dumps(doc,indent=2));print('passed',len(rows))
if __name__=='__main__':main()

