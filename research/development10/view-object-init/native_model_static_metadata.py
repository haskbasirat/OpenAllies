"""Bounded879A90 static Node/Geometry metadata with actual property owners.

Real RTTI/property lookup, omi/opi extras, vertex-color removal, tag inheritance,
8ABEC0 normal removal/bounds and ordered recursive879A90 calls execute. The
controller visitor872650 is an explicit child boundary guarded by actual empty
node/property/effect controller fields. Namespace Ui52, diagnostic delivery,
allocator and OS locks are declared services. No NiStream/GPU/task claim.
"""
from pathlib import Path
import sys,json,hashlib,struct,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import native_model_property_state as state
import native_model_defaults as defaults
import native_model_preprocessed as pp
sys.path.insert(0,str(HERE.parents[1]/'skirmish-20261017-model-resource-load'))
import native_model_base_texture as h
from native_model_prelude import make_tree,f
from native_model_property_extras import binary
class Machine(state.Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'metadata_armed',False):
   if pc==0x872650:
    owner=self.arg();assert self.r(owner+0x10)==0
    node=self.r(owner+0xa4)
    while node:
     property=self.r(node+8);assert not property or self.r(property+0x10)==0;node=self.r(node)
    self.events.append(dict(empty_controller_visitor=True));self.ret();return
   if pc==0x404ed0:self.events.append(dict(diagnostic_line=self.arg(2)));self.ret(value=0);return
   if pc==0x6f7810 and self.arg()in(12,48):
    count=self.arg();pointer=self.alloc(count);u.mem_write(pointer,b'\xa5'*count);self.ret(value=pointer);return
  super().hook(u,pc,size,user)
def add_property(m,node,kind,fields):
 #86F733 publishes the actual intrusive-property-list vtable at AVObject+A0.
 #This explicit layout fixture must supply it for native44D660 removals.
 m.w(node+0xa0,0xab7484)
 descriptor={'NiMaterialProperty':0xb43f40,'NiAlphaProperty':0xb43f34,'NiVertexColorProperty':0xb43f14}[kind];m.init_type(descriptor)
 pointer=m.alloc(100)
 if kind=='NiMaterialProperty':m.call(0x87d280,this=pointer)
 else:m.call(0x86b240,this=pointer);m.w(pointer,h.type_vtable(h.PE0,descriptor))
 m.w(pointer+4,100)
 if kind=='NiMaterialProperty':
  for i,value in enumerate(fields):m.w(pointer+0x24+4*i,value)
 elif kind=='NiAlphaProperty':m.u.mem_write(pointer+0x20,struct.pack('<HB',*fields))
 else:m.w(pointer+0x24,fields[0]);m.w(pointer+0x28,fields[1])
 entry=m.alloc(12);head=m.r(node+0xa4);tail=m.r(node+0xa8);m.w(entry+8,pointer)
 if tail:m.w(tail,entry);m.w(entry+4,tail)
 else:m.w(node+0xa4,entry)
 m.w(node+0xa8,entry);m.w(node+0xac,m.r(node+0xac)+1);return pointer
def named(m,node,name,kind='bool',value=1):
 if kind=='integer':return pp.put(m,node,name,0xb43f04,value)
 if kind=='bool':return h.extra(m,node,name,value)
 pointer=m.alloc(0x14);m.call(0x87b200,this=pointer);m.w(pointer,h.type_vtable(h.PE0,0xb45f6c));m.w(pointer+0x10,m.narrow(''));m.call(0x87b250,[m.narrow(name)],this=pointer,callee_pop=4);assert m.call(0x86ac80,[pointer],this=node,callee_pop=4)&255
def run(m,case):
 points=make_tree(m,[dict(name='root',children=[1]),dict(name='quad',kind='shape',parent=0)]);root,geometry=points
 m.w(geometry,h.type_vtable(h.PE0,0xb43f5c));data=m.alloc(0x60);m.w(data,h.type_vtable(h.PE0,0xb43f54));m.w(data+4,100);m.w(data+0xc,4);m.w(data+0x3c,2);vertices=m.alloc(48);normal=m.alloc(48);uv=m.alloc(32)
 positions=[[f(x),f(y),f(z)]for x,y,z in [(-1,-2,-.5),(1,-2,1),(1,2,1),(-1,2,0)]]
 for i,v in enumerate(positions):
  for j,word in enumerate(v):m.w(vertices+i*12+j*4,word)
 m.w(data+0x20,vertices);m.w(data+0x24,normal);m.w(data+0x2c,uv);m.w(data+0x30,0x5a5a0001);m.w(geometry+0xcc,data)
 for i,v in enumerate([f(10),f(20),f(30),f(40)]):m.w(data+0x10+i*4,v)
 material=None;alpha=None;vc=None
 words=[f((i+1)*.125)for i in range(14)]
 if case['material']:material=add_property(m,geometry,'NiMaterialProperty',words)
 if case['alpha']is not None:alpha=add_property(m,geometry,'NiAlphaProperty',case['alpha'])
 if case['vc']is not None:vc=add_property(m,geometry,'NiVertexColorProperty',case['vc'])
 if case['tag']!='none':named(m,root if case['inherited']else geometry,case['tag'],'string')
 if case['tag']in ['Sg','Sg2']:named(m,geometry,'Sgv','integer',1)
 metadata=m.alloc(0x28);m.w(metadata+0xc,0xff7fffff);m.w(metadata+0x10,0x7f7fffff);context=m.alloc(36);m.w(context+0x20,f(1));filename=m.alloc(4);m.cs(filename,'static.nif')
 variables=m.alloc(0xd4);byte=m.alloc(4);m.w(byte,case['remove_normals']);m.w(variables+0xd0,byte);m.w(0xb39940,variables);m.w(0xb43e88,case['remove_base']);m.events=[];m.metadata_armed=True
 try:result=m.call(0x879a90,[root,context,metadata,0,m.alloc(16),filename])
 finally:m.metadata_armed=False
 return dict(result=result&255,metadata=[m.r(metadata+4*i)for i in range(10)],root_opi=binary(m,root,'opi'),geometry_opi=binary(m,geometry,'opi'),material_omi=None if material is None else binary(m,material,'omi'),vertex_color_present=bool(m.call(0x86ea70,[9],this=geometry,callee_pop=4)),alpha_flags=None if alpha is None else m.r(alpha+0x20)&0xffff,data_flags=m.r(data+0x30),normal_present=bool(m.r(data+0x24)),sphere=[m.r(data+0x10+4*i)for i in range(4)],geometry_rep=bool(m.call(0x86ab40,[m.narrow('RepColor')],this=geometry,callee_pop=4)),events=m.events.copy())
def cases():
 for tag,inherited,remove_normals,remove_base in itertools.product(['none','RepColor','Sg','Sg2'],[False,True],[0,1],[0,1]):
  yield dict(material=True,alpha=None,vc=None,tag=tag,inherited=inherited,remove_normals=remove_normals,remove_base=remove_base)
 for material,alpha,vc in [(False,None,None),(True,[0,17],None),(True,[0x201,231],None),(False,[0x200,9],None),(True,None,[0,1]),(True,None,[1,1]),(True,None,[0,0])]:yield dict(material=material,alpha=alpha,vc=vc,tag='none',inherited=False,remove_normals=0,remove_base=0)
def main():
 rows=[];pins={}
 for image in ['original','v14']:
  m=Machine(image)
  for descriptor in [0xb43f04,0xb45f74,0xb43f5c,0xb43f54]:m.init_type(descriptor)
  for precision in h.PC:
   m.fpu(precision)
   for case in cases():
    try:output=run(m,case)
    except Exception as e:
     from unicorn.x86_const import UC_X86_REG_EIP
     raise RuntimeError((case,hex(m.u.reg_read(UC_X86_REG_EIP)),[hex(v)for v in list(m.actual)[-20:]]))from e
    rows.append(dict(image=image,precision=precision,input=case,output=output))
  pins.update(m.actual)
 assert[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 instructions=[]
 for address,size in sorted(pins.items()):
  if h.PE0.base<=address<h.PE0.base+h.PE0.image_size:
   raw=h.PE0.range(address,size);assert raw==h.PE1.range(address,size);instructions.append(dict(va=hex(address),bytes=raw.hex()))
 doc=dict(scope=__doc__,rows=rows,instructions=instructions,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest());path=HERE/'native-model-static-metadata.json'
 if '--create'in sys.argv:path.open('x').write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_bytes())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
