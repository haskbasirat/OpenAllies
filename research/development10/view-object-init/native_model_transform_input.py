"""NiAVObject86F1E6..209 source-field load ordering, not wholeNi loading.

Actual selected native instructions call869420 (three translationwords),
870C10 (nine consecutive matrixwords) and streamread scale. A supplied
streamread transport copies exact4bytes, preserving rawnegativezero and
arbitrary finite scalar words. SourceESI/EBX frame is explicit. This proves
no byte transpose occurs in this load slice; NIF block parsing is separate.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_ESP
class Reader(Machine):
 def hook(self,u,pc,size,user):
  if pc==getattr(self,'read_stub',None):
   assert self.arg(1)==4
   u.mem_write(self.arg(),struct.pack('<I',self.words[self.position]));self.position+=1
   self.ret(8,4);return
  super().hook(u,pc,size,user)
def main():
 rows=[];pins={}
 cases=[[0,0x80000000,0x3f000001]+[0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000]+[0x3f800000],
        [0x3f800001,0xbf000001,0x7e800001]+[0x3dcccccd+i for i in range(9)]+[0x3eaaaaab]]
 for image in ['original','v14']:
  m=Reader(image);m.read_stub=m.alloc(0x10)
  for pc in PC:
   for words in cases:
    m.fpu(pc);m.actual={};m.record_actual=True;m.words=words;m.position=0
    node=m.alloc(0x100);stream=m.alloc(0x170);reader=m.alloc(4);vt=m.alloc(0x10)
    m.w(stream+0x15c,reader);m.w(reader,vt);m.w(vt+8,m.read_stub)
    m.u.reg_write(UC_X86_REG_ESI,node);m.u.reg_write(UC_X86_REG_EBX,stream);m.u.reg_write(UC_X86_REG_ESP,m.stack)
    m.u.emu_start(0x86f1e6,0x86f209,count=1000)
    assert m.position==13 and m.u.reg_read(UC_X86_REG_ESP)==m.stack
    actual=[m.r(node+0x5c+i*4)for i in range(3)]+[m.r(node+0x38+i*4)for i in range(9)]+[m.r(node+0x68)]
    assert actual==words;rows.append(dict(image=image,precision=pc,input=words,output=actual));pins.update(m.actual)
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 out=HERE/'native-model-transform-input.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
