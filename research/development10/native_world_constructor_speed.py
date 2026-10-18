"""Exact World278 copy slices from currentSVar+114, before map Rule Load.
Constructor4F8125..813D and reset4F2BEB..2C05 execute without child seams.
Boundary stops do not certify the rest of either function.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'phase8-ai'))
import native_machine as nm
ORIGINAL=nm.original
class Boundary(Exception):pass

def run(image,pc,entry,end,bits):
 nm.original=ORIGINAL if image=='original' else nm.modified
 m=nm.Machine(pc);manager=m.alloc(0x200);cell=m.alloc(4);world=m.alloc(0x400)
 m.w(0xb39ab0,manager);m.w(manager+0x114,cell);m.w(cell,bits);m.w(world+0x278,0xdeadbeef)
 m.observers[entry]=lambda mm:mm.u.reg_write(nm.UC_X86_REG_ESI,world)
 def stop(mm):raise Boundary()
 m.seam(end,stop)
 try:m.call(entry)
 except Boundary:pass
 else:raise AssertionError('copy did not reach exact boundary')
 return m.r(world+0x278),m.visited

def main():
 rows=[];pins={}
 for entry,end in [(0x4f8125,0x4f813d),(0x4f2beb,0x4f2c05)]:
  for bits in [0,0x80000000,0x3f000000,0x3f800000,0x40000000,0x7fc01234,0x7f800000,0xffffffff,0x00800000]:
   for pc in nm.PRECISIONS:
    a,pa=run('original',pc,entry,end,bits);b,pb=run('v14',pc,entry,end,bits)
    assert a==b==bits
    for pp in (pa,pb):
     for va,raw in pp.items():
      assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw;pins[va]=raw
    rows.append(dict(entry=hex(entry),precision=pc,input=bits,output=a))
 doc=dict(schema=1,scope=__doc__,rows=rows,native_runs=len(rows)*2,
  instructions=[dict(va=hex(v),bytes=b.hex()) for v,b in sorted(pins.items())])
 path=HERE/'native-world-constructor-speed.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8')
 else:assert json.loads(path.read_text(encoding='utf-8'))==doc
 print(json.dumps(dict(rows=len(rows),runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
