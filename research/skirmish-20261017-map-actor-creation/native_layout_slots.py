"""Native4C2BC0 layout primary/fallback search; assertion transport explicit."""
from pathlib import Path
import json,sys,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
def one(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 layout=m.alloc(0x18);data=m.alloc(0x60);m.w(layout+8,data)
 m.w(data+0x18,c['primary']);m.w(data+0x28,m.array(c['groups']));m.w(data+0x2c,len(c['groups']))
 m.w(layout+0xc,m.array([0 if v is None else m.alloc(4)for v in c['parameters']]))
 calls=[]
 def assertion(mm):calls.append(dict(expression=mm.arg(1),file=mm.arg(2),line=mm.arg(3)));mm.ret(0)
 m.seam(0x855d30,assertion);m.u.mem_write(0xb3b6f5,b'\0')
 result=m.call(0x4c2bc0,layout)
 return None if result==0xffffffff else result,m.visited,calls
def main():
 corpus=[]
 for groups in [[],[0],[0,1],[1,0,1]]:
  for present in range(4):
   parameters=['actual_actor0'if present&1 else None,'actual_actor1'if present&2 else None]
   for primary in [-1]+list(range(len(groups))):
    corpus.append(dict(primary=primary,groups=groups,parameters=parameters))
 actual=json.loads((HERE/'actual-constructor-inputs.json').read_text()).get('layouts',[])
 corpus += [dict(primary=c['primary'],groups=c['groups'],parameters=c['parameters'],actual_actor=c['actor'],actual_layout=c['layout'])for c in actual]
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in corpus:
   a,va,calls=one(c,'original',pc);b,vb,bcalls=one(c,'v14',pc)
   assert a==b and calls==bcalls and va==vb
   for address,raw in va.items():assert raw==nm.original.range(address,len(raw))==nm.modified.range(address,len(raw));pins[address]=raw
   rows.append(dict(precision=pc,input=c,selected=a,assertion_reached=bool(calls)))
 out=dict(schema=1,scope=__doc__,rows=rows,instructions=nm.instructions(pins),producer_sha256=nm.sha(Path(__file__).read_bytes()))
 raw=(json.dumps(out,indent=2,sort_keys=True)+'\n').encode();path=HERE/'native-layout-slots.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw,'immutable layout fixture differs'
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,instructions=len(pins),sha256=nm.sha(raw))))
if __name__=='__main__':main()
