"""Actual5A9840 first-Bone/own NiTexturingProperty slot3 query.

One supplied named-node lookup is explicit transport, validating the exact
first Bone+1C input pointer. Native86EA70 property-list traversal,711540 RTTI,
real vtable type getters and slot3 presence execute unchanged. Descriptor
filter/value is deliberately poisoned/unread. No controller-time inference.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ECX
class Query(Machine):
 def hook(self,u,pc,size,user):
  if pc==getattr(self,'find_stub',None):
   assert self.arg()==self.expected_name;self.ret(4,self.found);return
  super().hook(u,pc,size,user)

def make_node(m,slots=None,children=()):
 node=m.alloc(0x100);m.w(node,0xab6950);m.w(node+0xd0,len(children));array=m.alloc(max(1,len(children))*4)
 m.w(node+0xc8,array)
 for i,child in enumerate(children):m.w(array+i*4,child)
 if slots is not None:
  prop=m.alloc(0x50);m.w(prop,0xab72a4);links=m.alloc(12);m.w(links+8,prop);m.w(node+0xa4,links)
  descriptors=m.alloc(28);m.w(prop+0x2c,descriptors);m.w(prop+0x30,7)
  for i,present in enumerate(slots):
   if present:
    descriptor=m.alloc(0x20);m.w(descriptor+0xc,0xdeadbeef);m.w(descriptors+4*i,descriptor)
 return node

def main():
 cases=[dict(found=False,own=None,children=[]),dict(found=True,own=None,children=[]),dict(found=True,own=[1,0,0,0,0,0,0],children=[]),dict(found=True,own=[0,0,0,1,0,0,0],children=[]),dict(found=True,own=[0]*7,children=[None,[0,0,0,1,0,0,0]]),dict(found=True,own=[0]*7,children=[None,[1,0,0,0,0,0,0]])]
 rows=[];pins={}
 for image in ['original','v14']:
  m=Query(image);m.find_stub=m.alloc(4)
  for pc in PC:
   for case in cases:
    m.fpu(pc);m.actual={};m.record_actual=True
    children=[0 if slots is None else make_node(m,slots)for slots in case['children']]
    selected=make_node(m,case['own'],children);root=make_node(m)
    #Only vf40 is a transport. All RTTI/property type functions come from
    #the exact original vtables, with executable pointer words pinned below.
    vt=m.alloc(0x90);m.u.mem_write(vt,PE0.range(0xab6950,0x90));m.w(vt+0x40,m.find_stub);m.w(root,vt)
    m.found=selected if case['found']else 0
    manager=m.alloc(0x600);bones=m.alloc(0x4c);vector=m.alloc(4);bone=m.alloc(0x24);name=m.alloc(16)
    m.u.mem_write(name,b'Scene Root\0');m.w(bone+0x1c,name);m.w(vector,bone);m.w(bones+0x24,vector);m.w(manager+0x580,bones);m.w(0xb36530,manager);m.expected_name=name
    result=m.call(0x5a9840,[root],callee_pop=4)&255
    expected=case['found']and(bool(case['own']and case['own'][3])or any(slots and slots[3]for slots in case['children']))
    assert result==int(expected),(case,result,expected)
    rows.append(dict(image=image,precision=pc,input=case,output=bool(result)));pins.update(m.actual)
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 dispatch=[]
 for a in [0xab6954,0xab72a8,0xab72d8]:
  raw=PE0.range(a,4);assert raw==PE1.range(a,4);dispatch.append(dict(va=hex(a),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,dispatch=dispatch)
 out=HERE/'native-model-texturing-query.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()

