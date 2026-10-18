"""Original Properties5812F0→5812A0→5816E0 cleanup on empty effects.

Real constructor, empty scheduler cancellation, Modifier502850 cleanup and
Updatable50B7A0 execute. GameData's definition-domain count and owner pointer
are explicit laboratory inputs. Allocation/free are named successful transport
seams; no scheduled positive node/effect/ability instance is claimed. Saved
table50B470/50B500 execute; returned table poison forbids stale-pointer reads.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_updatable_lifetime import Lifetime,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_FPSW
def run(m,pc,c):
 m.lifetime_ready=False;m.retired=False;m.trace=[];m.fpu(pc);m.actual={};m.record_actual=True
 world=m.alloc(0x300);m.w(0xb36534,world);m.u.reg_write(UC_X86_REG_ESI,world);m.u.reg_write(UC_X86_REG_EBX,0)
 m.u.emu_start(0x4f804e,0x4f805a)
 game=m.alloc(0x800);domain=m.alloc(0x40);m.w(0xb36530,game);m.w(game+0x3a8,domain);m.w(domain+0x28,c['definition_count'])
 actor=m.alloc(0x200);component=m.alloc(0xdc);m.u.mem_write(component,b'\xa5'*0xdc)
 m.call(0x5812f0,[actor],this=component,callee_pop=4)
 assert m.r(component)==0xa3e3dc and m.r(component+4)==actor
 assert m.r(component+0xc)==0 and m.r(component+0x14)==0 and m.r(world+0x280)==0
 m.table=m.alloc(12);m.call(0x50b470,[3,world+0x284],this=m.table,callee_pop=8);array=m.r(m.table+4)
 for i,v in enumerate([component+8,component+8,0]):m.w(array+4*i,v)
 m.call(0x5812a0,this=component)
 after_uninit=dict(head=m.r(component+0xc),effects_count=m.r(component+0x14))
 if c['returned']:
  m.call(0x50b500,this=m.table);assert m.r(world+0x284)==0;m.u.mem_write(m.table,b'\xa5'*12);m.retired=True
 m.lifetime_ready=True
 m.call(0x5816e0,[1],this=component,callee_pop=4)
 assert ((m.u.reg_read(UC_X86_REG_FPSW)>>11)&7)==0
 return dict(after_uninit=after_uninit,world284_present=m.r(world+0x284)!=0,world280_present=m.r(world+0x280)!=0,saved_slots=None if m.retired else ['self'if m.r(array+4*i)==component+8 else m.r(array+4*i)for i in range(3)],trace=m.trace),m.actual.copy()
def main():
 rows=[];pins={}
 for image in ['original','v14']:
  m=Lifetime(image)
  for pc in PC:
   for count in [0,1,17]:
    for returned in [False,True]:
     c=dict(definition_count=count,returned=returned);output,visited=run(m,pc,c);pins.update(visited);rows.append(dict(image=image,precision=pc,input=c,output=output))
 a=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='original'];b=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 data=[]
 for va in [0xa3e3dc,0xa3e3dc+0x10]:
  raw=PE0.range(va,4);assert raw==PE1.range(va,4);data.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,data_pins=data)
 p=HERE/'native-properties-cleanup.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executed_pins=len(instructions),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
