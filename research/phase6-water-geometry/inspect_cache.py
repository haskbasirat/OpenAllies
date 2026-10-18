from pathlib import Path
import json,struct,sys
sys.path.insert(0,str(Path.cwd().parent/'reconstruction/tools'))
from map_extract import read_chunks
class R:
 def __init__(self,b,at):self.b=b;self.at=at
 def take(self,n):a=self.at;self.at+=n;assert self.at<=len(self.b),(a,n,len(self.b));return self.b[a:self.at]
 def u32(self):return struct.unpack('<I',self.take(4))[0]
 def count(self,stride):n=self.u32();assert n*stride<=len(self.b)-self.at;return n
 def ver(self):v=self.u32();assert v==0,(self.at,v)
for path,row in json.load(open('research/phase6-spatial/native-real-map-terrain.json'))['maps'].items():
 b=read_chunks(Path('../reconstruction/assets/original/Data')/path)['TERRAIN'][0];r=R(b,row['suffix_offset']);r.ver();enabled=r.take(1)[0]
 if enabled:
  for _ in range(r.count(16)):
   r.ver();r.take(8);r.take(8*r.count(8))
  for _ in range(r.count(36)):
   r.ver();r.take(8)
   for side in range(4):
    r.take(2)
    for _ in range(r.count(16)):r.ver();r.take(12)
 cache=r.at;r.ver();nv=r.count(8);vertices=r.take(nv*8);ni=r.count(4);indices=struct.unpack('<'+str(ni)+'I',r.take(ni*4));nc=r.count(16);cells=[struct.unpack('<4I',r.take(16))for _ in range(nc)];assert r.at==len(b),(path,r.at,len(b))
 assert all(i<nv for i in indices)
 assert all(x<row['layout']['width'] and y<row['layout']['height'] and s<=e<ni and(e-s+1)%3==0 for x,y,s,e in cells)
 print(path,cache,nv,ni,nc)
