"""Actual8ABEC0 GeometryData replacement and89F5F0 sphere recomputation.

Fields/arrays are explicit native-layout inputs, including authored old sphere,
allocation/shared-block ownership and the empty-vertex global center words.
Only inherited allocation/free transports are used. This does not claim native
NIF loading, actual startup values for the empty-case globals, controller or
skin behavior. Source vertices are consumed unchanged by original/v14/allPC.
"""
from pathlib import Path
import sys,json,struct,hashlib,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
import native_model_prelude as h
from unicorn import UC_HOOK_MEM_WRITE

def run(m,case):
 data=m.alloc(0x40);vertices=m.alloc(max(1,len(case['vertices']))*12);old_normal=m.alloc(12);old_color=m.alloc(16);old_uv=m.alloc(8)
 for i,v in enumerate(case['vertices']):
  for j,w in enumerate(v):m.w(vertices+i*12+j*4,w)
 for i,w in enumerate(case['empty_center']):m.w(0xb43cf8+4*i,w)
 block=m.alloc(0x10)if case['shared']else 0
 if block:m.w(block+0xc,100)
 old_vertices=vertices if not case['replace_vertices']else m.alloc(12)
 m.w(data+8,block);m.w(data+0xc,0xa5a50005);m.w(data+0x20,old_vertices);m.w(data+0x24,old_normal);m.w(data+0x28,old_color);m.w(data+0x2c,old_uv);m.w(data+0x30,case['flags'])
 for i,w in enumerate(case['old_sphere']):m.w(data+0x10+4*i,w)
 selected_vertices=vertices if case['vertices_present']else 0
 fields={data+o:hex(o)for o in [0xc,0x10,0x14,0x18,0x1c,0x20,0x24,0x28,0x2c,0x30]};writes=[]
 def write(u,access,address,size,value,user):
  if address in fields:writes.append(dict(offset=fields[address],bytes=size,value=value&((1<<(8*size))-1)))
 hook=m.u.hook_add(UC_HOOK_MEM_WRITE,write)
 try:m.call(0x8abec0,[len(case['vertices']),selected_vertices,0,old_color,old_uv,case['uv_sets'],case['additional_flags']],this=data,callee_pop=28)
 finally:m.u.hook_del(hook)
 def pointer(p):
  if p==0:return None
  for name,known in [('vertices',vertices),('old_vertices',old_vertices),('old_normal',old_normal),('old_color',old_color),('old_uv',old_uv)]:
   if p==known:return name
  raise AssertionError(hex(p))
 #Normalize native allocation addresses in the write trace while preserving
 #exact order and scalar stores. Each field still belongs to this SAME Data.
 for event in writes:
  if event['offset']in ['0x20','0x24','0x28','0x2c']:event['value']=pointer(event['value'])
 return dict(count_word=m.r(data+0xc),sphere=[m.r(data+0x10+i*4)for i in range(4)],vertices=pointer(m.r(data+0x20)),normal=pointer(m.r(data+0x24)),color=pointer(m.r(data+0x28)),uv=pointer(m.r(data+0x2c)),flags_word=m.r(data+0x30),block_references=None if not block else m.r(block+0xc),writes=writes)

def cases():
 f=h.f
 sets=[[],[[0,0,0]],[[f(3),f(-4),f(1)]],[[f(3),f(-4),f(-1)],[f(-7),f(5),f(9)]],[[f(x),f(y),f(z)]for x,y,z in [(3,-4,-1),(-7,5,9),(2,2,2),(0,1,1),(-1,-2,3)]],[[0x3f800001,0x3f7fffff,0x80000000],[f(127.1),f(-129.3),f(1e-20)],[f(.1),f(-.3),f(1e-10)],[f(3),f(-3),f(-1)]]]
 for vertices,shared,replace in itertools.product(sets,[False,True],[False,True]):
  yield dict(vertices=vertices,shared=shared,replace_vertices=replace,vertices_present=True,old_sphere=[f(100),f(-200),f(300),f(400)],empty_center=[f(1.25),0x80000000,f(-2.5)],flags=0xa5a5ffff,uv_sets=0,additional_flags=0)
 for present,uv_sets,additional in itertools.product([False,True],[0,3,0xff],[0,0x8000,0xffff]):
  yield dict(vertices=[[f(1),f(2),f(3)]],shared=False,replace_vertices=False,vertices_present=present,old_sphere=[f(100),f(-200),f(300),f(400)],empty_center=[0,0,0],flags=0x5a5a1234,uv_sets=uv_sets,additional_flags=additional)

def main():
 rows=[];pins={};data={}
 for image in ['original','v14']:
  m=h.Machine(image)
  for pc in h.h.PC:
   m.fpu(pc)
   for case in cases():rows.append(dict(image=image,precision=pc,input=case,output=run(m,case)))
  pins.update(m.actual)
  for key,value in m.data_reads.items():
   if key in data:assert data[key]==value
   else:data[key]=value
 a=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='original'];b=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='v14'];assert a==b
 instructions=[]
 for at,size in sorted(pins.items()):
  if h.h.PE0.base<=at<h.h.PE0.base+h.h.PE0.image_size:
   raw=h.h.PE0.range(at,size);assert raw==h.h.PE1.range(at,size);instructions.append(dict(va=hex(at),bytes=raw.hex()))
 reads=[]
 for(address,size),value in sorted(data.items()):
  raw=h.h.PE0.range(address,size);assert raw==h.h.PE1.range(address,size)and raw.hex()==value;reads.append(dict(va=hex(address),bytes=value))
 out=dict(scope=__doc__,rows=rows,instructions=instructions,immutable_data_reads=reads,images=dict(original=hashlib.sha256(h.h.PE0.b).hexdigest(),v14=hashlib.sha256(h.h.PE1.b).hexdigest()),producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-geometry-replace.json'
 if '--create'in sys.argv:
  with path.open('x')as target:json.dump(out,target,indent=2);target.write('\n')
 else:assert json.loads(path.read_bytes())==out
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),data_pins=len(reads),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
