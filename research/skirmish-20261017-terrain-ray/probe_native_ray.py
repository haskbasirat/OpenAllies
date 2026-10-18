"""Development probe for whole71D840 over explicit laboratory TerrainMap cells.
Not an accepted oracle until producer/projection fields are source-proved.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_WRITE
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
def run(eye,direction,heights=None,mesh=True,loaded=True,pc='Double53',secondary=True,border=0):
 m=h.Machine(pc);terrain=m.alloc(0x158);heightfield=m.alloc(0x50);sample=m.alloc(9*9*4)
 m.w32(terrain+4,0,0,h.f2b(1));m.w32(terrain+16,h.f2b(1),0,0);m.w32(terrain+28,0,h.f2b(1),0)
 m.w32(terrain+0x6c,heightfield);m.w32(terrain+0x74,16,16,8,8,2,2)
 vals=heights if heights is not None else[float((x+y)%5)for y in range(9)for x in range(9)]
 m.w32(terrain+0x8c,h.f2b(min(vals)),h.f2b(max(vals)));m.w32(terrain+0xa0,8)
 cells=m.alloc(16);m.w32(terrain+0xb8,cells);m.w8(terrain+0xbc,int(loaded))
 m.w32(heightfield+4,sample);m.w32(heightfield+16,h.f2b(2),h.f2b(.5),9,9)
 m.w32(sample,*map(h.f2b,vals))
 for y in range(2):
  for x in range(2):
   cell=m.alloc(0x150);m.w32(cells+(y*2+x)*4,cell);m.w32(cell+0x40,terrain)
   vertices=[]
   for yy in range(5):
    for xx in range(5):vertices.extend([float(x*8+xx*2),float(y*8+yy*2),vals[(y*4+yy)*9+x*4+xx]])
   blockheights=vertices[2::3];m.w32(cell+0x80,h.f2b(min(blockheights)),h.f2b(max(blockheights)))
   m.w32(cell+0xac,x,y);m.w32(cell+0xc8,5,4,1,2)
   if mesh:
    ni=m.alloc(0x100);data=m.alloc(0x40);v=m.alloc(len(vertices)*4)
    m.w32(cell+0x44,ni);m.w32(ni+0xcc,data);m.w32(data+0x20,v);m.w32(v,*map(h.f2b,vertices))
 eye_ptr=m.alloc(12);m.w32(eye_ptr,*map(h.f2b,eye));out=m.alloc(12);m.w32(out,0xcdcdcdcd,0xcdcdcdcd,0xcdcdcdcd)
 secondary_ptr=m.alloc(12)if secondary else 0
 if secondary:m.w32(secondary_ptr,0xabababab,0xabababab,0xabababab)
 writes=[]
 def write(u,access,address,size,value,data):
  if out<=address<out+12:writes.append(['point',address-out,value&0xffffffff])
  if secondary and secondary_ptr<=address<secondary_ptr+12:writes.append(['secondary',address-secondary_ptr,value&0xffffffff])
 m.u.hook_add(UC_HOOK_MEM_WRITE,write)
 try:
  result=m.call(0x71d840,terrain,[eye_ptr,*map(h.f2b,direction),out,secondary_ptr,border],count=1_000_000)
  return dict(result=result&255,out=m.r32(out,3),secondary=m.r32(secondary_ptr,3)if secondary else None,writes=writes,esp=m.final_esp-h.STACK)
 except Exception:
  pcnow=m.u.reg_read(h.UC_X86_REG_EIP);print('native stop',hex(pcnow))
  print([(hex(i.address),i.mnemonic,i.op_str)for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(h.P.range(pcnow,32),pcnow)])
  raise
if __name__=='__main__':
 for direction in[[0,0,-1],[.2,.1,-1],[-.5,.2,-1],[1,1,0]]:
  print(direction,json.dumps(run([8,8,50],direction)))
