"""Actual8792F0 external texture use counts and same-SourceTexture otn.

Descriptors, SourceTexture/AVObject fields and initially empty pointer/count/
capacity vectors are explicit inputs. Real RTTI, vector growth, name lookup,
NiStringExtra construction and insertion execute. Allocator/ASCII strings and
the positive internal-texture diagnostic are declared service transports.
No NiStream loader, default property, renderer or readiness claim.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parents[1]/'skirmish-20261017-model-resource-load'))
import native_model_base_texture as h
class Machine(h.Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'armed',False)and pc==0x404ed0:self.events.append('internal-texture-diagnostic');self.ret(value=0);return
  super().hook(u,pc,size,user)
def run(m,rows,old):
 names=m.alloc(12);counts=m.alloc(12);callback=m.alloc(0x20);m.w(callback+0x18,counts);metadata=m.alloc(36);filename=m.alloc(4);m.cs(filename,'actual.nif');sources=[];m.events=[]
 for name,particles in rows:
  descriptor=m.alloc(8);source=m.alloc(0x50);node=m.alloc(0x100);m.w(source,h.type_vtable(h.PE0,0xb43f24));m.w(source+4,100);m.w(source+0x38,0 if name is None else m.narrow(name));m.w(descriptor+4,source);m.w(node,h.type_vtable(h.PE0,0xb43ce0 if particles else 0xb43cf0));m.w(node+0xc,m.narrow('texture owner'))
  if old:h.extra(m,source,'otn',77)
  m.armed=True
  try:m.call(0x8792f0,[descriptor,names,callback,metadata,node,filename])
  finally:m.armed=False
  extra=m.call(0x86ab40,[m.narrow('otn')],this=source,callee_pop=4);sources.append(None if not extra else dict(kind='existing-boolean'if m.r(extra)==0xa44ec8 else'string',value=m.r(extra+0x10)if m.r(extra)==0xa44ec8 else m.cascii(m.r(extra+0x10))))
 return dict(names=[m.cascii(m.r(m.r(names)+4*i))for i in range(m.r(names+4))],counts=[[m.r(m.r(counts)+8*i),m.r(m.r(counts)+8*i+4)]for i in range(m.r(counts+4))],internal=bool(m.r(metadata+0x10)&255),internal_ordinary=m.r(metadata+0x14),source_extras=sources,events=m.events.copy())
def main():
 rows=[];pins={}
 cases=[[],[['A.dds',False]],[['A.dds',False],['a.DDS',False],['B.dds',False]],[['A.dds',True],['a.DDS',False],['B.dds',True]],[[None,False]],[[None,True]],[['base.dds',False],[None,True],['BASE.dds',True]]]
 for image in ['original','v14']:
  m=Machine(image);m.init_type(0xb45f6c)
  for precision in h.PC:
   m.fpu(precision)
   for source,old in itertools.product(cases,[False,True]):
    try:output=run(m,source,old)
    except Exception as e:
     from unicorn.x86_const import UC_X86_REG_EIP
     raise RuntimeError((source,old,hex(m.u.reg_read(UC_X86_REG_EIP)),[hex(x)for x in list(m.actual)[-20:]]))from e
    rows.append(dict(image=image,precision=precision,input=source,old_otn=old,output=output))
  pins.update(m.actual)
 assert[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='original']==[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='v14']
 instructions=[]
 for address,size in sorted(pins.items()):
  if h.PE0.base<=address<h.PE0.base+h.PE0.image_size:
   raw=h.PE0.range(address,size);assert raw==h.PE1.range(address,size);instructions.append(dict(va=hex(address),bytes=raw.hex()))
 doc=dict(scope=__doc__,rows=rows,instructions=instructions,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest());path=HERE/'native-model-texture-usage.json'
 if '--create'in sys.argv:path.open('x').write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_bytes())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
