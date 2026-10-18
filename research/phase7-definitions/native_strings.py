"""Execute native plain-string assignment, including NULL->empty conversion."""
from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase5-actors'))
from original_creation import Machine as Base,P,V,H
from unicorn.x86_const import *
class Machine(Base):
 def __init__(self):
  super().__init__(0x27f);self.w(0x2255064,self.r(0xb35444));self.value=None
 def hook(self,u,pc,n,z):
  if pc==0x9738a9:self.ret(0x2255000)
  elif pc==0x85c680:self.ret(self.alloc(256))
  elif pc==0x5b3f00:
   p=self.r(self.arg()+16);n=self.r(p-12);self.value=list(struct.unpack('<%dH'%n,u.mem_read(p,n*2)));self.ret(pop=4)
  else:super().hook(u,pc,n,z)
 def string_field(self,text):
  parser,token,node=0x2256000,0x2257000,0x2258000
  self.w(parser,node);self.w(parser+0x40,self.string([ord(c)for c in'field']));self.w(parser+0x38,6)
  self.w(token+0x18,self.string([ord(c)for c in text]));self.w(token+0x10,7);self.value=None
  self.invoke(0x5bb0b0,parser,[token]);return self.value

def main():
 rows=[];visited={}
 for text in ['null','NULL','NuLl','POINT','INSTANT','usa_infantry','None','null_thing']:
  m=Machine();value=m.string_field(text);assert value==([]if text.lower()=='null'else[ord(c)for c in text]);rows.append({'raw':text,'value_utf16':value});visited.update(m.visited)
 spans=[]
 for a,n in sorted(visited.items()):
  if not P.base<=a<P.base+P.image_size:continue
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 pins=[]
 for a,b in spans:
  raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
 doc={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'ranges':pins,'seams':['Original5BB0B0 assignment and String field constructor execute; string-pool allocation/refcounts use existing successful allocation fixture seams.','Field insertion captures UTF16 from constructed field+10. C-locale TLS pointer supplied to original CRT.','Quoted/localized, escape expansion and string-valued expressions are outside this plain ASCII identifier subset.']}
 (HERE/'native-strings.json').write_text(json.dumps(doc,indent=2));print('passed',len(rows))
if __name__=='__main__':main()
