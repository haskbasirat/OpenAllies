"""Actual VFX vf14 dispatch from constructor-pinned vtables, 494380/494480.
The Data allocation/current +20/+40 fields are explicit bounded inputs. The
optional Resource vf10 child is an explicit service or required-error stop;
this is not source loading, a Resource implementation, or GPU completion.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
RECEIPT=ROOT/'research/skirmish-20261017-view-asset-backend/vfx-request-vtables.json'
class Required(Exception):pass

def cases():
 for row in json.loads(RECEIPT.read_text())['rows']:
  if row['native_kind']==0:continue
  for initial,delta,mode in [(0,0,0),(0,1,0),(7,5,1),(0x7fffffff,1,0xffffffff),(0xffffffff,2,7)]:
   for resource in ['null','present','required']:
    yield dict(kind=row['native_kind'],vtable=row['actual_constructor_vtable'],initial=initial,delta=delta,mode=mode,resource=resource)

def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 vt=int(c['vtable'],16);method=m.r(vt+0x14)
 assert method==(0x494480 if c['kind'] in [10,11,14,17] else 0x494380)
 assert nm.original.range(vt+0x14,4)==nm.modified.range(vt+0x14,4)
 data=m.alloc(0x44);m.u.mem_write(data,b'\xcd'*0x44);m.w(data,vt);m.w(data+0x20,c['initial'])
 trace=[]
 def child(mm):
  trace.append(dict(args=[mm.arg(1),mm.arg(2)],word=mm.r(data+0x20)))
  if c['resource']=='required':raise Required()
  mm.ret(0xabcdef01,8)
 if c['resource']=='null':m.w(data+0x40,0)
 else:
  resource=m.alloc(4);rvt=m.alloc(0x14);m.w(resource,rvt);m.w(rvt+0x10,m.stub(child));m.w(data+0x40,resource)
 status='ok'
 try:m.call(method,data,[c['delta'],c['mode']])
 except Required:status='required'
 return dict(word=m.r(data+0x20),trace=trace,status=status),m.visited

def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases():
   pair=[]
   for image in ['original','v14']:
    output,visited=run(c,image,pc);pair.append(output);pins.update(visited)
   assert pair[0]==pair[1];rows.append(dict(precision=pc,input=c,output=pair[0]))
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),'constructor_vtable_receipt_sha256':hashlib.sha256(RECEIPT.read_bytes()).hexdigest(),'harness_sha256':hashlib.sha256((ROOT/'research/phase8-ai/native_machine.py').read_bytes()).hexdigest()}
 path=HERE/'native-vfx-request.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
