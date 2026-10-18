"""Actual878F30 version lookup/removal and lazy named-AVar warning order.

Real RTTI initializers, typed extra lookup and remove execute. Source extra
allocations are explicit fields;405370 namespace lookup and404ED0 diagnostic
delivery are declared reached services recording name/current byte/callorder.
No false AVar default, NiStream loader or full87AA10 success is supplied.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parents[1]/'skirmish-20261017-model-resource-load'))
import native_model_base_texture as h
class Machine(h.Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'armed',False):
   if pc==0x405370:
    name=self.text(self.r(self.arg()));self.events.append(dict(read=name,value=self.warning));self.ret(4,self.warning_ref);return
   if pc==0x404ed0:self.events.append(dict(diagnostic=True));self.ret(value=0);return
  super().hook(u,pc,size,user)
def put(m,node,name,kind,value):
 p=m.alloc(0x18);m.call(0x87b200,this=p);m.w(p,h.type_vtable(h.PE0,kind));m.w(p+0x10,value)
 if kind==0xb45f74:m.w(p+0x14,40)
 m.call(0x87b250,[m.narrow(name)],this=p,callee_pop=4);assert m.call(0x86ac80,[p],this=node,callee_pop=4)&255;return p
def main():
 rows=[];pins={}
 for image in ['original','v14']:
  m=Machine(image);m.init_type(0xb43f04);m.init_type(0xb45f74)
  for pc in h.PC:
   m.fpu(pc)
   for version in [None,-1,0,6,7,'wrong-type']:
    for binary in [False,True]:
     for warning in [0,1]:
      node=m.alloc(0x100);payload=m.alloc(40)
      if version is not None:put(m,node,'ppdv',0xb43f04 if isinstance(version,int)else 0xb45f74,version&0xffffffff if isinstance(version,int)else payload)
      if binary:put(m,node,'ppd',0xb45f74,payload)
      text=m.alloc(4);m.cs(text,'actual-model.nif');m.events=[];m.warning=warning;cell=m.alloc(4);m.w(cell,warning);m.warning_ref=m.alloc(4);m.w(m.warning_ref,cell);m.armed=True
      result=m.call(0x878f30,[node,text]);m.armed=False
      present=[name for name in ['ppdv','ppd']if m.call(0x86ab40,[m.narrow(name)],this=node,callee_pop=4)]
      assert result in [0,payload];rows.append(dict(image=image,precision=pc,version=version,binary=binary,warning=warning,present=present,reused_payload=result==payload,events=m.events.copy()))
  pins.update(m.actual)
 left=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='original'];right=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='v14'];assert left==right
 instructions=[]
 for address,size in sorted(pins.items()):
  if h.PE0.base<=address<h.PE0.base+h.PE0.image_size:
   raw=h.PE0.range(address,size);assert raw==h.PE1.range(address,size);instructions.append(dict(va=hex(address),bytes=raw.hex()))
 doc=dict(scope=__doc__,rows=rows,instructions=instructions,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest());path=HERE/'native-model-preprocessed.json'
 if '--create'in sys.argv:path.open('x').write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_bytes())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
