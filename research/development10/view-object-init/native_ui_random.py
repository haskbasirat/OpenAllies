"""Reached UI5CAE30 D4 constructor slice and exact85D840 stream.

5CAFA7..5CAFD5 executes from the supplied actual this/zero-EBX constructor
frame, stopping before following CString fields. This does not certify the
whole UI constructor, later indirect writers or unported UI random consumers.
Draw rows retain their one mutable word across the full sequence. No native
simulation RNG or clock seed is substituted for the UI allocation.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_ESP
def main():
 rows=[];pins={}
 for image in ['original','v14']:
  m=Machine(image)
  for pc in PC:
   m.fpu(pc);m.actual={};m.record_actual=True
   ui=m.alloc(0x200);m.u.mem_write(ui,b'\xa5'*0x200)
   m.u.reg_write(UC_X86_REG_ESI,ui);m.u.reg_write(UC_X86_REG_EBX,0);m.u.reg_write(UC_X86_REG_ESP,m.stack)
   m.u.emu_start(0x5cafa7,0x5cafd5,count=1000);seed=m.r(ui+0xd4);assert seed==0x8e4b12a5
   values=[]
   for _ in range(12):
    value=m.call(0x85d840,[ui+0xd4]);values.append(dict(value=value,state=m.r(ui+0xd4)))
   rows.append(dict(image=image,precision=pc,seed=seed,values=values));pins.update(m.actual)
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 out=HERE/'native-ui-random.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),draws=sum(len(r['values'])for r in rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
