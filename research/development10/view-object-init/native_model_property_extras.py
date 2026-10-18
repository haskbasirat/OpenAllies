"""Execute879D77..879F1E material omi/property opi publication sub-block.

Material/alpha pointers are explicit already-RTTI-checked operands of879A90;
this does not execute the preceding lookup, tree traversal or whole87AA10.
Actual named-extra lookup, BinaryExtra constructor and insertion run. Heap
transport poisons new48/12-byte payloads so unwritten alpha padding remains
visible. Invalid allocation aliases/OOM/native reference-count parity excluded.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parents[1]/'skirmish-20261017-model-resource-load'))
from native_model_base_texture import Machine as Base,PE0,PE1,PC,extra
from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_EDI,UC_X86_REG_ESP

class Stop(Exception):pass
class Machine(Base):
 def hook(self,u,pc,size,user):
  if getattr(self,'armed',False):
   if pc==0x879d77:
    u.reg_write(UC_X86_REG_EBX,self.node);u.reg_write(UC_X86_REG_EDI,self.material)
    self.w(u.reg_read(UC_X86_REG_ESP)+0x18,self.alpha)
   if pc==0x879f1e:raise Stop()
   if pc==0x6f7810 and self.arg()in(12,48):
    size=self.arg();pointer=self.alloc(size);u.mem_write(pointer,b'\xa5'*size);self.ret(value=pointer);return
  super().hook(u,pc,size,user)

def binary(m,owner,name):
 pointer=m.call(0x86ab40,[m.narrow(name)],this=owner,callee_pop=4)
 if not pointer:return None
 if m.r(pointer)!=0xab7bfc:return dict(kind='existing-boolean',value=m.r(pointer+0x10))
 count=m.r(pointer+0x14);return dict(kind='binary',bytes=bytes(m.u.mem_read(m.r(pointer+0x10),count)).hex())

def main():
 rows=[];pins={}
 cases=[dict(material=material,alpha=alpha,old_omi=omi,old_opi=opi)
  for material,alpha,omi,opi in [(False,None,False,False),(True,None,False,False),(True,[0,17],False,False),(True,[0x201,231],False,False),(False,[0x200,9],False,False),(True,[1,35],True,False),(True,[0x201,90],False,True),(True,[0,0],True,True)]]
 for image in ['original','v14']:
  m=Machine(image)
  for precision in PC:
   m.fpu(precision)
   for case in cases:
    m.armed=False;m.node=m.alloc(0x100);m.material=m.alloc(0x60)if case['material']else 0;m.alpha=m.alloc(0x24)if case['alpha']is not None else 0
    words=[struct.unpack('<I',struct.pack('<f',(i+1)*0.125))[0]for i in range(14)]
    if m.material:
     for i,word in enumerate(words):m.w(m.material+0x24+i*4,word)
    if m.alpha:m.u.mem_write(m.alpha+0x20,struct.pack('<HB',*case['alpha']))
    if case['old_omi']:extra(m,m.material,'omi',77)
    if case['old_opi']:extra(m,m.node,'opi',88)
    m.actual={};m.record_actual=True;m.armed=True
    try:m.call(0x879d77)
    except Stop:pass
    else:raise AssertionError('property sub-block did not reach required following child')
    m.armed=False;pins.update(m.actual)
    rows.append(dict(image=image,precision=precision,input=case,source_words=words,material_omi=binary(m,m.material,'omi')if m.material else None,node_opi=binary(m,m.node,'opi')))
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows[:24]]==[{k:v for k,v in r.items()if k!='image'}for r in rows[24:]]
 instructions=[]
 for address,size in sorted(pins.items()):
  if PE0.base<=address<PE0.base+PE0.image_size:
   raw=PE0.range(address,size);assert raw==PE1.range(address,size);instructions.append(dict(va=hex(address),bytes=raw.hex()))
 strings=[]
 for address,text in [(0xab7674,b'omi\0'),(0xab7678,b'opi\0')]:
  assert PE0.range(address,len(text))==PE1.range(address,len(text))==text;strings.append(dict(va=hex(address),bytes=text.hex()))
 doc=dict(scope=__doc__,rows=rows,instructions=instructions,strings=strings,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-model-property-extras.json'
 if '--create'in sys.argv:path.open('x').write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_bytes())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
