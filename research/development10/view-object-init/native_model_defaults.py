"""Actual NiMain8C3ED0 first twelve default-property producers.

Executes real CRT9E7850 registration ->8C46C0/8601A0, then the actual
8C3ED0 prefix through all twelve constructors, stopping BEFORE8C3F20's next
child. The following initialized-guard call executes and allocates nothing.
App4196A0->860120 and its registered indirect callback are static both-PE
callsite pins, not whole App/NiInit execution. Heap storage is poisoned and
native write masks distinguish padding. Allocation failure/destruction and
arbitrary later alias writes are outside this bounded fresh-start interval.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parents[1]/'skirmish-20261017-model-resource-load'))
from native_model_base_texture import Machine as Base,PE0,PE1
from unicorn import UC_HOOK_MEM_WRITE
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
GLOBALS=[0xb43f3c,0xb442f8,0xb44304,0xb43f48,0xb44334,0xb44310,0xb4431c,0xb442ec,0xb43dfc,0xb43f1c,0xb43e08,0xb44328]
SIZES=[0x24,0x24,0x34,0x64,0x24,0x24,0x24,0x40,0x48,0x2c,0x24,0x28]
class Stop(Exception):pass
class Machine(Base):
 def hook(self,u,pc,size,user):
  if getattr(self,'armed',False):
   if pc==0x8c3f20:raise Stop()
   if pc==0x6f7810:
    count=self.arg();assert count<0x1000;pointer=self.alloc(count);u.mem_write(pointer,b'\xa5'*count);self.allocations[pointer]=count;self.mask[pointer]=set();self.ret(value=pointer);return
  super().hook(u,pc,size,user)
def zero_tail(pe,address,size):
 offset=struct.unpack_from('<I',pe.b,0x3c)[0];sections=struct.unpack_from('<H',pe.b,offset+6)[0];optional=struct.unpack_from('<H',pe.b,offset+20)[0]
 for i in range(sections):
  p=offset+24+optional+40*i;name=pe.b[p:p+8].rstrip(b'\0').decode();virtual,relative,raw,_=struct.unpack_from('<IIII',pe.b,p+8)
  if pe.base+relative+raw<=address and address+size<=pe.base+relative+virtual:return dict(section=name,virtual_address=hex(pe.base+relative),virtual_size=virtual,raw_size=raw,offset=address-(pe.base+relative),size=size)
 raise AssertionError(hex(address))
def main():
 rows=[];pins={};dispatch={};cs=Cs(CS_ARCH_X86,CS_MODE_32)
 for image in ['original','v14']:
  for precision in ['Single','Double','Extended']:
   m=Machine(image);m.fpu(precision);m.allocations={};m.mask={};m.actual={};m.record_actual=True
   m.nonorder.discard(0x9e7850);m.call(0x9e7850);assert m.r(0xb43c64)==0x8c3ed0 and m.r(0xb43c68)==0x8c4380
   assert all(m.r(g)==0 for g in GLOBALS)and m.r(0xb45f54)==0
   assert [m.r(0xb43e68+i*4)for i in range(3)]==[0,0,0]
   before_counter=m.r(0xb323c0);assert before_counter==1
   def write(u,access,address,size,value,user):
    for allocation,count in m.allocations.items():
     if allocation<=address and address+size<=allocation+count:m.mask[allocation].update(range(address-allocation,address-allocation+size));break
   hook=m.u.hook_add(UC_HOOK_MEM_WRITE,write);m.armed=True
   try:m.call(0x8c3ed0)
   except Stop:pass
   else:raise AssertionError('default prefix did not reach next required child')
   finally:m.armed=False;m.u.hook_del(hook)
   properties=[];pointers=[m.r(g)for g in GLOBALS];assert len(set(pointers))==12 and all(pointers)
   for index,(global_address,pointer,size)in enumerate(zip(GLOBALS,pointers,SIZES)):
    assert m.allocations[pointer]==size;vt=m.r(pointer);kind=m.call(m.r(vt+0x34),this=pointer);assert kind==index
    descriptor=m.call(m.r(vt+4),this=pointer);m.init_type(descriptor);name=m.cascii(m.r(descriptor))
    for address in [vt+4,vt+0x34]:dispatch[address]=PE0.range(address,4)
    scalar=[]
    for offset in range(0x20,size):
     if offset in m.mask[pointer]:scalar.append(dict(offset=offset,value=int.from_bytes(bytes(m.u.mem_read(pointer+offset,1)),'little')))
    if index==8:
     #The descriptor array pointer is an owned allocation identity, never a
     #portable integer. Seven NULL entries/count7 are captured separately.
     array=m.r(pointer+0x2c);assert m.allocations[array]==28
     scalar=[x for x in scalar if not 0x2c<=x['offset']<0x30]
     descriptors=[m.r(array+4*i)for i in range(7)];assert descriptors==[0]*7
    else:descriptors=None
    properties.append(dict(index=index,global_address=hex(global_address),class_name=name,allocation_size=size,reference_count=m.r(pointer+4),written_scalar_bytes=scalar,descriptor_slots=descriptors))
   pins.update(m.actual);before_allocations=len(m.allocations);m.armed=True;m.call(0x8c3ed0);m.armed=False
   assert[m.r(g)for g in GLOBALS]==pointers and len(m.allocations)==before_allocations
   rows.append(dict(image=image,precision=precision,registered_initializer=hex(m.r(0xb43c64)),prefix_flag=m.r(0xb45f54),material_counter_before=before_counter,material_counter_after=m.r(0xb323c0),properties=properties,guard_allocations=0))
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows[:3]];b=[{k:v for k,v in r.items()if k!='image'}for r in rows[3:]];assert a==b
 instructions=[]
 for address,size in sorted(pins.items()):
  if PE0.base<=address<PE0.base+PE0.image_size:
   raw=PE0.range(address,size);assert raw==PE1.range(address,size);instructions.append(dict(va=hex(address),bytes=raw.hex()))
 static=[]
 for start,count in [(0x860120,0x40),(0x4196a0,0x350)]:
  for i in cs.disasm(PE0.range(start,count),start):
   if(start==0x860120)or(i.mnemonic=='call'and i.op_str=='0x860120'):
    assert PE0.range(i.address,i.size)==PE1.range(i.address,i.size);static.append(dict(va=hex(i.address),bytes=i.bytes.hex(),instruction=i.mnemonic+' '+i.op_str))
 assert any(x['instruction']=='call 0x860120'for x in static)
 tails={hex(a):zero_tail(PE0,a,n)for a,n in [(0xb43e68,12),(0xb45f54,1),(0xb45f58,4)]+[(g,4)for g in GLOBALS]}
 assert tails=={hex(a):zero_tail(PE1,a,n)for a,n in [(0xb43e68,12),(0xb45f54,1),(0xb45f58,4)]+[(g,4)for g in GLOBALS]}
 counter=PE0.range(0xb323c0,4);assert counter==PE1.range(0xb323c0,4)==struct.pack('<I',1)
 doc=dict(scope=__doc__,rows=rows,instructions=instructions,static_parent_pins=static,dispatch=[dict(va=hex(a),bytes=b.hex())for a,b in sorted(dispatch.items())],zero_tail=tails,material_counter_image_bytes=counter.hex(),producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-model-defaults.json'
 if '--create'in sys.argv:path.open('x').write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_bytes())==doc
 print(json.dumps(dict(rows=len(rows),properties=12,pins=len(instructions),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
