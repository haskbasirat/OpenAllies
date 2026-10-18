"""Actual87A820 base-texture choice and878170 same-property bt mutation.
Real RTTI initializers/vtable getters,86EA70 own-property traversal,711540,
5A4D00 Boolean(+10), extra lookup/remove/name/insert and recursive walks run.
Tree/property/descriptor/string allocations are explicit field fixtures.
Wide-to-narrow ASCII454810 and bounded ASCII CRT comparisons/wide searches
are transports; no NIF loader/geometry claim.
"""
from pathlib import Path
import sys,json,hashlib,struct,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import Machine as Base,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ECX

def va(pe,off):
 return next(pe.base+r+off-start for _,r,size,start in pe.sections if start<=off<start+size)
def initializer(pe,descriptor):
 raw=pe.b;needle=b'\x68';positions=[];start=0
 while True:
  pos=raw.find(b'\xb9'+struct.pack('<I',descriptor)+b'\xe8',start)
  if pos<0:break
  start=pos+1
  if raw[pos-5]==0x68 and va(pe,pos)>0x9c0000:
   if raw[pos-10]==0x68:positions.append((pos-10,struct.unpack_from('<I',raw,pos-9)[0],struct.unpack_from('<I',raw,pos-4)[0]))
   elif raw[pos-7:pos-5]==b'\x6a\0':positions.append((pos-7,0,struct.unpack_from('<I',raw,pos-4)[0]))
 assert len(positions)==1,(hex(descriptor),positions)
 offset,parent,name=positions[0];return va(pe,offset),parent,name
def type_vtable(pe,descriptor):
 pos=pe.b.find(b'\xb8'+struct.pack('<I',descriptor)+b'\xc3');assert pos>=0
 leaf=va(pe,pos);needle=struct.pack('<I',leaf);positions=[];start=0
 while True:
  pos=pe.b.find(needle,start)
  if pos<0:break
  start=pos+1
  if va(pe,pos)>0xa00000:positions.append(va(pe,pos)-4)
 assert len(positions)==1,(descriptor,positions)
 return positions[0]

class Machine(Base):
 def __init__(self,image):
  super().__init__(image);self.actual={};self.record_actual=True;self.ascii_transports=[];self.initialized=set();self.type_names={}
  self.cstring_allocate_stub=self.alloc(4);self.w(self.r(self.manager),self.cstring_allocate_stub)
  for descriptor in [0xb43cd8,0xb43ce0,0xb43cf0,0xb43df4,0xb43f24]:self.init_type(descriptor)
 def init_type(self,descriptor):
  if descriptor==0 or descriptor in self.initialized:return
  fn,parent,name=initializer(PE0,descriptor);assert PE0.range(fn,21)==PE1.range(fn,21)
  #The inherited order-registration laboratory skips unrelated CRT static
  #initializers. Explicitly remove these reached RTTI initializers from that
  #transport set so their real stores execute and enter the actual pins.
  self.nonorder.discard(fn);self.nonorder.discard(0x872e40)
  self.init_type(parent);self.call(fn);self.initialized.add(descriptor)
  assert self.r(descriptor)==name and self.r(descriptor+4)==parent,(hex(descriptor),hex(fn),hex(name),hex(parent),hex(self.r(descriptor)),hex(self.r(descriptor+4)))
  self.type_names[hex(descriptor)]=self.cascii(name)
 def cascii(self,p):
  out=bytearray()
  while p and self.u.mem_read(p,1)!=b'\0':out.extend(self.u.mem_read(p,1));p+=1
  return out.decode('ascii')
 def narrow(self,text):
  p=self.alloc(len(text)+1);self.u.mem_write(p,text.encode('ascii')+b'\0');return p
 def hook(self,u,pc,size,user):
  if pc==getattr(self,'cstring_allocate_stub',None):
   count=self.arg();width=self.arg(1);assert width in(1,2)and count<10000
   header=self.alloc(16+(count+1)*width);self.w(header,self.manager);self.w(header+4,0);self.w(header+8,count);self.w(header+12,1);self.ret(8,header);return
  if pc==0x96de44:
   pointer=self.arg();char=self.arg(1)&0xffff;index=0;last=0
   while True:
    unit=int.from_bytes(bytes(u.mem_read(pointer+2*index,2)),'little')
    if unit==char:last=pointer+2*index
    if unit==0:break
    index+=1
   self.ret(value=last);return
  if pc==0x96c26d:self.ret(value=len(self.text(self.arg())));return
  if pc in (0x97251b,0x970817):
   left=self.cascii(self.arg());right=self.cascii(self.arg(1));count=self.arg(2)if pc==0x97251b else None
   if count is not None:left=left[:count];right=right[:count]
   left=left.lower();right=right.lower();self.ret(value=(left>right)-(left<right));return
  if pc==0x454810:
   source=self.text(self.arg());assert source.isascii();out=u.reg_read(UC_X86_REG_ECX)
   self.cs(out,source);self.u.mem_write(self.r(out),source.encode('ascii')+b'\0')
   self.ascii_transports.append(source);self.ret(4,out);return
  super().hook(u,pc,size,user)

def extras(m,node):
 if not node:return None
 out=[]
 for i in range(m.r(node+0x18)):
  entry=m.r(m.r(node+0x14)+i*4);out.append(dict(name=m.cascii(m.r(entry+0xc)),value=m.r(entry+0x10),boolean_vtable=m.r(entry)==0xa44ec8))
 return out
def extra(m,node,name,value):
 p=m.alloc(0x14);m.call(0x5a4d00,[value],this=p,callee_pop=4)
 m.call(0x87b250,[m.narrow(name)],this=p,callee_pop=4)
 assert m.call(0x86ac80,[p],this=node,callee_pop=4)&255
def property_owner(m,node):return m.call(0x86ea70,[8],this=node,callee_pop=4)
def node(m,kind='geometry',texture='a.dds',property=True,children=(),initial=()):
 p=m.alloc(0x100);m.w(p,type_vtable(PE0,{'node':0xb43cd8,'geometry':0xb43cf0,'particles':0xb43ce0}[kind]))
 array=m.alloc(max(1,len(children))*4);m.w(p+0xc8,array);m.w(p+0xd0,len(children))
 for i,child in enumerate(children):m.w(array+4*i,child)
 if property:
  prop=m.alloc(0x50);m.w(prop,0xab72a4);link=m.alloc(12);m.w(link+8,prop);m.w(p+0xa4,link)
  descriptors=m.alloc(28);m.w(prop+0x2c,descriptors);m.w(prop+0x30,7)
  if texture is not False:
   descriptor=m.alloc(0x20);m.w(descriptors,descriptor)
   if texture!='wrong-class':
    source=m.alloc(0x4c);m.w(source,type_vtable(PE0,0xb43f24));m.w(source+0x38,0 if texture is None else m.narrow(texture))
   else:source=m.alloc(4);m.w(source,0xab6950)
   m.w(descriptor+4,source)
 for name,value in initial:extra(m,prop if property else p,name,value)
 return p

def choices():
 #Each usage entry is an actual {begin,end} pointer pair; equal meansunused.
 return [dict(file='Art/Depot.nif',names=[],used=[]),
  dict(file='Art/Depot.nif',names=['other.dds'],used=[True]),
  dict(file='Art/Depot.nif',names=['other.dds'],used=[False]),
  dict(file='Art/Depot.nif',names=['other.dds','Depot.dds'],used=[True,True]),
  dict(file='Art/Depot.nif',names=['Depotbase.dds','other.dds'],used=[True,True]),
  dict(file='Art/Depot.nif',names=['DEPOT.DDS','Depotbase.dds'],used=[True,True]),
  dict(file='Art/Depot.nif',names=['xbase.dds','yBASE.tga','plain.dds'],used=[True,True,True]),
  dict(file='Art/Depot.nif',names=['Depot.dds','other.dds'],used=[False,True]),
  dict(file='C:Depot.nif',names=['Depot.dds','other.dds'],used=[True,True]),
  dict(file='Art/Depot',names=['Depot.tga','other.dds'],used=[True,True]),
  dict(file='Art\\Depot.nif',names=['Depot.tga','other.dds'],used=[True,True]),
  dict(file='Art/Depot.nif',names=['a.dds','b.dds'],used=[True,True])]
def run_choice(m,c):
 names=m.alloc(8);array=m.alloc(max(1,len(c['names']))*4);m.w(names,array);m.w(names+4,len(c['names']))
 usage=m.alloc(4);pairs=m.alloc(max(1,len(c['used']))*8);m.w(usage,pairs)
 pointers=[]
 for i,(name,used)in enumerate(zip(c['names'],c['used'])):
  p=m.narrow(name);pointers.append(p);m.w(array+4*i,p);m.w(pairs+8*i,123);m.w(pairs+8*i+4,124 if used else 123)
 filename=m.alloc(4);m.cs(filename,c['file']);selected=m.call(0x87a820,[names,usage,filename]);assert selected==0 or selected in pointers
 return None if not selected else pointers.index(selected)
def mutations():
 return [dict(kind=k,texture=t,selected=s,property=prop,initial=initial)
  for k,t,s,prop,initial in [
   ('geometry','a.dds','A.DDS',True,[]),('geometry','a.dds','b.dds',True,[['bt',0]]),
   ('geometry','a.dds',None,True,[['aa',9],['bt',0],['zz',7]]),
   ('geometry',None,'b.dds',True,[]),('geometry',False,None,True,[]),('geometry',False,'b.dds',True,[]),
   ('geometry','wrong-class',None,True,[]),('geometry','wrong-class','b.dds',True,[]),
   ('geometry','a.dds','a.dds',False,[['bt',0]]),('particles','a.dds','a.dds',True,[['bt',0]]),
   ('node','a.dds','a.dds',True,[]),('geometry','a.dds','a.dds',True,[['BT',8],['bt',0]])]]
def main():
 rows=[];pins={};types=None
 for image in ['original','v14']:
  m=Machine(image)
  for pc in PC:
   m.fpu(pc)
   for c in choices():rows.append(dict(image=image,precision=pc,kind='choice',input=c,selected_index=run_choice(m,c)))
   for c in mutations():
    p=node(m,c['kind'],c['texture'],c['property'],initial=c['initial']);prop=property_owner(m,p);before=extras(m,prop);node_before=extras(m,p)
    m.call(0x878170,[p,0 if c['selected']is None else m.narrow(c['selected'])]);rows.append(dict(image=image,precision=pc,kind='mutation',input=c,before=before,after=extras(m,prop),node_before=node_before,node_after=extras(m,p)))
   child=node(m,texture='a.dds');particle=node(m,kind='particles',texture='a.dds',initial=[['bt',0]]);parent=node(m,kind='node',property=False,children=[child,0,particle]);m.call(0x878170,[parent,m.narrow('A.DDS')])
   rows.append(dict(image=image,precision=pc,kind='recursive',parent=extras(m,parent),child_property=extras(m,property_owner(m,child)),particle_property=extras(m,property_owner(m,particle))))
   left=node(m,texture='a.dds');right=node(m,texture='unused.dds');shared=property_owner(m,left)
   m.w(m.r(right+0xa4)+8,shared);root=node(m,kind='node',property=False,children=[left,right])
   m.call(0x878170,[root,m.narrow('A.DDS')]);assert property_owner(m,right)==shared
   assert m.r(shared+0x18)==1;tag=m.r(m.r(shared+0x14));assert m.r(tag+4)==1
   rows.append(dict(image=image,precision=pc,kind='shared_property',same_owner=True,left=extras(m,property_owner(m,left)),right=extras(m,property_owner(m,right)),boolean_reference_count=m.r(tag+4)))
  pins.update(m.actual)
  if types is None:types=m.type_names
  else:assert types==m.type_names
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for at,size in sorted(pins.items()):
  if PE0.base<=at<PE0.base+PE0.image_size:
   raw=PE0.range(at,size);assert raw==PE1.range(at,size);instructions.append(dict(va=hex(at),bytes=raw.hex()))
 assert PE0.range(0xab767c,3)==PE1.range(0xab767c,3)==b'bt\0'
 dispatch=[]
 for address in [type_vtable(PE0,d)+4 for d in [0xb43cd8,0xb43ce0,0xb43cf0,0xb43f24]]+[0xab72a8,0xab72d8,0xa44ec8]:
  raw=PE0.range(address,4);assert raw==PE1.range(address,4);dispatch.append(dict(va=hex(address),bytes=raw.hex()))
 out=dict(scope=__doc__,rows=rows,types=types,instructions=instructions,dispatch=dispatch,explicit_transports=['454810 ASCII conversion','96DE44 wide last-character search','96C26D wide length','97251B ASCII case-insensitive bounded comparison','970817 ASCII case-insensitive comparison','inherited allocator/CString storage/free transports'],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-model-base-texture.json'
 if '--create'in sys.argv:
  with path.open('x')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_bytes())==out
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
