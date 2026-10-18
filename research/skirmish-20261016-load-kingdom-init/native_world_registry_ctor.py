"""GWorld4F7D00 constructor's actual144/148/154/160 field initialization.

Execute4F7E63..4F7EDF with the earlier4F7D35 xorEBX constructor input. Heap
allocation is an explicit successful host seam. Allocated element bytes are
unwritten; this proof does not turn spare capacity into initialized null slots.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_ESP
ORIGINAL=nm.original

def main():
 rows=[];pins={}
 for image in ('original','v14'):
  nm.original=ORIGINAL if image=='original' else nm.modified
  for pc in nm.PRECISIONS:
   m=nm.Machine(pc);world=m.alloc(0x300);m.u.mem_write(world,b'\xcc'*0x300)
   m.seam(0x6f7810,lambda mm:mm.ret(mm.alloc(mm.arg(1))))
   m.u.reg_write(UC_X86_REG_ESI,world);m.u.reg_write(UC_X86_REG_EBX,0);m.u.reg_write(UC_X86_REG_ESP,nm.STACK)
   m.u.emu_start(0x4f7e63,0x4f7edf,count=1000)
   output=dict(count=m.r(world+0x144),all_count=m.r(world+0x14c),major_count=m.r(world+0x158),color_count=m.r(world+0x164),
      capacities=[m.r(world+off) for off in (0x150,0x15c,0x168)],allocations=[bool(m.r(world+off)) for off in (0x148,0x154,0x160)])
   assert output==dict(count=0,all_count=0,major_count=0,color_count=0,capacities=[32,32,1],allocations=[True]*3)
   rows.append(dict(image=image,precision=pc,output=output));pins.update(m.visited)
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 assert ORIGINAL.range(0x4f7d35,2)==nm.modified.range(0x4f7d35,2)==b'\x33\xdb'
 doc=dict(schema=1,scope=__doc__,cases=rows,instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(pins.items())],
  static=[dict(va=hex(0x4f7d35),bytes='33db')])
 path=HERE/'native-world-registry-constructor.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
