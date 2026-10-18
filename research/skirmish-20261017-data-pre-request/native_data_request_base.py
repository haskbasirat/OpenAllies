"""Unchanged4BBE70 Data constructor+20 and4BB040/4BB050/4BB060 words.
CString reference transport and string allocator are explicit services. No
Object/Actor child request, ResourceManager or decoded asset completion claim.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
def cases():
 return [[],[['request',0,0]],[['request',1,0]],[['request',1,1],['request',2,0],['release',1]],
 [['request',0x7fffffff,0],['request',1,0],['release',1]],
 [['request',0xffffffff,0],['request',1,0]],
 [['release',1],['request',1,0]],
 [['request',0x80000000,0],['release',0x80000001]],
 [['request',5,0xffffffff],['release',7],['request',3,0]],
 [['request',0xffffffff,0],['request',0xffffffff,0],['request',3,0]],
 [['request',0x80000000,1],['request',0x80000000,0]],
 [['request',1,0],['release',0],['release',1]]]
def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 header=m.alloc(32);m.w(header+12,100);m.u.mem_write(header+16,b'a\0\0\0');ids=m.array([header+16])
 helper=m.alloc(4);vt=m.alloc(16);m.w(helper,vt);m.w(vt+12,m.stub(lambda mm:mm.ret(header)))
 m.seam(0x4012c0,lambda mm:mm.ret(mm.arg(1)))
 m.seam(0x856370,lambda mm:mm.ret(helper))
 data=m.alloc(0x24);m.u.mem_write(data,b'\xcd'*0x24);m.call(0x4bbe70,data,[ids,0x76543210])
 result=[dict(word=m.r(data+0x20),requested=bool(m.call(0x4bb060,data)))]
 assert result[0]['word']==0
 for operation in c:
  if operation[0]=='request':m.call(0x4bb040,data,operation[1:])
  else:m.call(0x4bb050,data,operation[1:])
  result.append(dict(word=m.r(data+0x20),requested=bool(m.call(0x4bb060,data))))
 return result,m.visited
def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases():
   pair=[]
   for image in('original','v14'):
    output,visited=run(c,image,pc);pair.append(output);pins.update(visited)
   assert pair[0]==pair[1];rows.append(dict(precision=pc,input=c,output=pair[0]))
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),'harness_sha256':hashlib.sha256((ROOT/'research/phase8-ai/native_machine.py').read_bytes()).hexdigest()}
 path=HERE/'native-data-request-base.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
