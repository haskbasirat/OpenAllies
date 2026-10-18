"""Original case-insensitive Text manager lookup. No lookup/comparison hooks."""
from pathlib import Path
import sys,json,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-definitions'))
from native_strings import Machine as Base,P,V,H
from unicorn.x86_const import *

def units(s):return list(s.encode('utf-16le')[i]|(s.encode('utf-16le')[i+1]<<8) for i in range(0,len(s.encode('utf-16le')),2))
class Machine(Base):
 def __init__(self):
  super().__init__();self.manager=self.alloc(0x80);self.table=self.alloc(4)
  self.w(self.manager+0x4c,self.table);self.w(self.manager+0x50,1);self.w(self.table,0)
 def add(self,key,value):
  row=self.alloc(0x20);self.w(row,0xa12cb4);self.w(row+4,1)
  self.w(row+12,self.string(units(key)));self.w(row+16,self.string(units(value)))
  link=self.alloc(8);self.w(link,row);self.w(link+4,self.r(self.table));self.w(self.table,link)
 def lookup(self,key,fallback):
  a=self.alloc(4);b=self.alloc(4);self.w(a,self.string(units(key)));self.w(b,self.string(units(fallback)))
  result=self.invoke(0x4b9f40,self.manager,[a,b]);p=self.r(result);n=self.r(p-12)
  return list(struct.unpack('<%dH'%n,self.u.mem_read(p,n*2))) if n else []
def main():
 cases=[];visited={}
 entries=[['Side1','United States'],['SIDE2','Germany'],['blank',''],['kingdom_name_usa','U.S.A.']]
 for key in ['side1','Side1','SIDE1','SiDe1','side2','SIDE2','blank','BLANK','kingdom_name_usa','KINGDOM_NAME_USA','missing']:
  m=Machine()
  for k,v in entries:m.add(k,v)
  out=m.lookup(key,'fallback')
  expected=next((v for k,v in entries if k.lower()==key.lower()),'fallback')
  assert out==units(expected),(key,out,expected)
  cases.append({'key_utf16':units(key),'fallback_utf16':units('fallback'),'output_utf16':out})
  visited.update(m.visited)
 spans=[]
 for a,n in sorted(visited.items()):
  if not P.base<=a<P.base+P.image_size:continue
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 pins=[]
 for a,b in spans:
  raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
 data={'passed':True,'entries':entries,'cases':cases,'ranges':pins,'producer_sha256':H(Path(__file__).read_bytes()),
 'original_sha256':H(P.b),'v14_sha256':H(V.b),'scope':'Original4B9F40/4B9D40/hash/CRTcomparison execute with original CString fields and collision-linked table. Native allocation/TLS string fixture services only. Registration order and missing-key empty-fallback diagnostic configuration are excluded; portable source adapter rejects those cases.'}
 target=HERE/'native-localization.json';target.write_text(json.dumps(data,indent=2)+'\n');print('passed',len(cases),H(target.read_bytes()))
if __name__=='__main__':main()
