"""Actual47A650 KingdomColor Data+C producer and native93D4E0 append fragment.

The unchanged create-tail executes after an explicit successful constructed
Data allocation. Source/TGI constructor and hash insertion are outside this
fragment; the source manager's current registration count is an explicit input.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
from unicorn.x86_const import UC_X86_REG_EAX,UC_X86_REG_EDI,UC_X86_REG_ESP
ORIGINAL=nm.original

def main():
 rows=[];pins={}
 for image in ('original','v14'):
  nm.original=ORIGINAL if image=='original' else nm.modified
  for pc in nm.PRECISIONS:
   for count in (0,1,2,7,18,19,31,33):
    m=nm.Machine(pc);manager=m.alloc(0x80);obj=m.alloc(0x24)
    old=[m.alloc(0x24) for _ in range(count)];array=m.array(old+[0,0]);m.w(manager+0x24,array)
    m.w(manager+0x28,count);m.w(manager+0x2c,count+2);m.w(obj+12,0xcccccccc)
    m.u.reg_write(UC_X86_REG_EAX,obj);m.u.reg_write(UC_X86_REG_EDI,manager);m.u.reg_write(UC_X86_REG_ESP,nm.STACK)
    m.u.emu_start(0x47a6fd,0x47a71c,count=1000)
    assert m.r(obj+12)==count and m.r(manager+0x28)==count+1
    assert [m.r(array+4*i) for i in range(count+1)]==old+[obj]
    rows.append(dict(image=image,precision=pc,count_before=count,color_index=m.r(obj+12),
        count_after=m.r(manager+0x28),prefix_retained=True,new_color_appended=True))
    pins.update(m.visited)
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,cases=rows,
  instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(pins.items())],
  static=[dict(va=hex(0xa0c57c+0x1c),bytes=ORIGINAL.range(0xa0c57c+0x1c,4).hex())])
 assert ORIGINAL.range(0xa0c57c+0x1c,4)==nm.modified.range(0xa0c57c+0x1c,4)==bytes.fromhex('90a54700')
 path=HERE/'native-color-ordinal.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
