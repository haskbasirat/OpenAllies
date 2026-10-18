"""Actual shared45BBA0 virtual8C and original view-vtable slot receipts.

No successful seams are reached. Laboratory Thing/view scalar owners, both
images and all3PC. This does not certify derived view constructors or Init.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
def main():
 rows=[];pins={}
 for image in ['original','v14']:
  m=Machine(image)
  for pc in PC:
   for flags in [0,0x4000,0x1000101,0xffffffff]:
    for thing_flags in [0,0x4000,0xffffffff]:
     view=m.alloc(0x80);thing=m.alloc(0x40);m.w(view+0x14,thing);m.w(view+0x54,flags);m.w(thing+8,thing_flags)
     m.actual={};m.record_actual=True;m.fpu(pc);m.call(0x45bba0,this=view);pins.update(m.actual)
     rows.append(dict(image=image,precision=pc,flags=flags,thing_flags=thing_flags,result=m.r(view+0x54)))
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 slots=[]
 for name,vt in [('VThing',0xa02d40),('VObject',0xa02798),('VActor',0xa02270),('Organization',0xa019f8),('Camera',0xa01ca0)]:
  raw=PE0.range(vt+0x8c,4);assert raw==PE1.range(vt+0x8c,4)and int.from_bytes(raw,'little')==0x45bba0
  slots.append(dict(name=name,va=hex(vt+0x8c),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,slots=slots)
 out=HERE/'native-view-flag14.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
