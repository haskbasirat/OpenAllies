"""Original/v14 Area/Camera/Doodad constructors, vtables and memory predicates.

Derived constructors execute unchanged with named base Object-constructor and
empty-string allocator seams. This does not re-prove the base constructor.
Memory predicates execute real class/Data vtables; only actual Session phase is
a fixture seam. Actor mask+174 is unread and forbidden for all admitted classes.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_ESP
ORIGINAL=nm.original
CLASSES={6:dict(name='Area',ctor=0x4db6f0,vtable=0xa28a00,data=0xa13318,kind=1,category=27,init=0x4db200),
 7:dict(name='Camera',ctor=0x4dbf40,vtable=0xa28b88,data=0xa13448,kind=20,category=28,init=0x4dbcf0),
 9:dict(name='Doodad',ctor=0x4dc6f0,vtable=0xa28e90,data=0xa0f6e0,kind=21,category=38,init=0x4dc730)}
def constructor(info,image,pc):
 nm.original=ORIGINAL if image=='original' else nm.modified;m=nm.Machine(pc)
 obj=m.alloc(0x100);m.u.mem_write(obj,b'\xcd'*0x100);base_calls=[]
 def base(mm):base_calls.append([mm.arg(i) for i in (1,2,3)]);mm.ret(mm.ecx(),pop=12)
 m.seam(0x4ea690,base)
 empty=m.alloc(18);m.u.mem_write(empty,b'\0'*18);allocator=m.alloc(16);vt=m.alloc(16)
 m.w(allocator,vt);m.w(vt+12,m.stub(lambda mm:mm.ret(empty)))
 m.seam(0x856370,lambda mm:mm.ret(allocator))
 definition=m.alloc(0x510);assert m.call(info['ctor'],obj,[0,definition,0x12340007])==obj
 assert m.r(obj)==info['vtable'] and base_calls==[[0,definition,0x12340007]]
 out=dict(vtable=hex(m.r(obj)),base_ctor_arguments_verified=True)
 if info['name']=='Area':
  out.update(updatable_vtable=hex(m.r(obj+0x60)),head=m.r(obj+0x64),shape=[m.r(obj+o) for o in range(0x68,0x80,4)],
   empty_label=m.r(obj+0x80)==empty+16,entry_head=m.r(obj+0x84),entry_tail=m.r(obj+0x88))
 elif info['name']=='Camera':out.update(word60=m.r(obj+0x60),empty_label=m.r(obj+0x64)==empty+16)
 else:out.update(components=[m.r(obj+0x60),m.r(obj+0x64)])
 return out,m.visited
def memory(info,image,pc,phase,flags,announce):
 nm.original=ORIGINAL if image=='original' else nm.modified;m=nm.Machine(pc)
 obj=m.alloc(0x100);data=m.alloc(0x510);m.w(obj,info['vtable']);m.w(obj+4,data);m.w(obj+8,flags)
 m.w(data,info['data']);m.wb(data+0x16d,announce);m.w(data+0x174,0xdeadbeef)
 def forbidden(u,access,address,size,value,_):
  assert not (address<data+0x178 and data+0x174<address+size),'special Object read Actor mask'
 m.u.hook_add(UC_HOOK_MEM_READ,forbidden)
 m.seam(0x76ac90,lambda mm:mm.ret(phase))
 out=dict(potential=bool(m.call(0x4ea350,obj)&255),memory=bool(m.call(0x4eb490,obj)&255))
 return out,m.visited
def main():
 rows=[];pins={};static=[]
 for cls,info in CLASSES.items():
  for offset in (0xa8,0xb0,0xb4,0xe8):
   va=info['vtable']+offset;raw=ORIGINAL.range(va,4);assert raw==nm.modified.range(va,4)
   static.append(dict(va=hex(va),bytes=raw.hex()))
  for offset in (0,0x24,0x28,0x2c,0x30,0x38,0x3c,0x40):
   va=info['data']+offset;raw=ORIGINAL.range(va,4);assert raw==nm.modified.range(va,4)
   static.append(dict(va=hex(va),bytes=raw.hex()))
  assert int.from_bytes(ORIGINAL.range(info['vtable']+0xe8,4),'little')==info['init']
  for pc in nm.PRECISIONS:
   a,pa=constructor(info,'original',pc);b,pb=constructor(info,'v14',pc);assert a==b;pins.update(pa);pins.update(pb)
   rows.append(dict(class_id=cls,precision=pc,operation='constructor',output=a))
   for phase in (0,5):
    for flags in (0,1<<29,(1<<29)|(1<<11),(1<<29)|(1<<23),(1<<29)|(1<<22),0xffffffff):
     for announce in (0,1):
      a,pa=memory(info,'original',pc,phase,flags,announce);b,pb=memory(info,'v14',pc,phase,flags,announce);assert a==b
      pins.update(pa);pins.update(pb);rows.append(dict(class_id=cls,precision=pc,operation='memory',phase=phase,flags=flags,announce=announce,output=a))
 for image in ('original','v14'):
  nm.original=ORIGINAL if image=='original' else nm.modified
  for pc in nm.PRECISIONS:
   m=nm.Machine(pc);world=m.alloc(0x300);m.u.mem_write(world,b'\xcc'*0x300)
   m.u.reg_write(UC_X86_REG_ESI,world);m.u.reg_write(UC_X86_REG_EBX,0);m.u.reg_write(UC_X86_REG_ESP,nm.STACK)
   m.u.emu_start(0x4f7f09,0x4f7f57,count=1000)
   assert m.r(world+0x180)==m.r(world+0x190)==0
   rows.append(dict(image=image,precision=pc,operation='world_counter_constructor',output=[0,0]));pins.update(m.visited)
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,classes={str(k):v for k,v in CLASSES.items()},paired_constructor_memory_rows=sum(v['operation']!='world_counter_constructor' for v in rows),
  native_executions=sum(1 if v['operation']=='world_counter_constructor' else 2 for v in rows),cases=rows,
  instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(pins.items())],static=static)
 path=HERE/'native-special-classes.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
