"""Actual component+20 requirement virtuals; Structure executes4D1050."""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
TABLE=[0x580f70,0x580f70,0x580f70,0x580f70,None,0x580f70,0x580f70,0x580f70,0x580f70,0x580f70,0x580f70,0x580f70,0x580f70,0x589090,0x580f70,0x4c9fa0,0x580f70,0x581370,0x7c53e0,0x580f70,0x6b2450,0x580f70,0x580f70,0x6b2450,0x7c7840,0x580f70,0x580f70,0x580f70,0x580f70,0x580f70,0x580f70]
def one(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 component=m.alloc(0x40);actor=m.alloc(0x180);m.w(component+4,actor)
 m.w(actor+0x100,c['state'][0]);m.w(actor+0x104,c['state'][1])
 n=m.call(c['entry'],component)
 return None if n==0xffffffff else n,m.visited
def main():
 corpus=[dict(slot=i,kind='standard',entry=a,state=[0,0])for i,a in enumerate(TABLE)if a is not None]
 corpus += [dict(slot=4,kind=kind,entry=entry,state=[0,0])for kind,entry in [('organization',0x7c53e0),('element',0x5b6830),('aircraft',0x55ea50)]]
 corpus += [dict(slot=13,kind='standard',entry=0x589090,state=s)for s in [[1<<26,0],[0,1<<4],[1<<26,1<<4]]]
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in corpus:
   a,va=one(c,'original',pc);b,vb=one(c,'v14',pc);assert a==b and va==vb
   for address,raw in va.items():assert raw==nm.original.range(address,len(raw))==nm.modified.range(address,len(raw));pins[address]=raw
   rows.append(dict(precision=pc,input=c,required=a))
 out=dict(schema=1,scope=__doc__,rows=rows,instructions=nm.instructions(pins),producer_sha256=nm.sha(Path(__file__).read_bytes()))
 raw=(json.dumps(out,indent=2,sort_keys=True)+'\n').encode();path=HERE/'native-requirements.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw,'immutable requirements fixture differs'
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,instructions=len(pins),sha256=nm.sha(raw))))
if __name__=='__main__':main()
