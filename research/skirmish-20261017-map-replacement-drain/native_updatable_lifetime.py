"""Fresh World280/284 and real saved-table50B470/50B500 lifetime.

Executes constructor's zero EBX instruction then exact World280/284 store
window, transporting this/EBX across the unexecuted remainder of construction.
Executes full50B470 and50B500 and full50B7A0 Updatable destructor. Allocation
and free are existing named successful transport seams. Positive scheduled
head50B250 or save-hash7ADDA0 stops before that child. After table return, the
retired allocation is poisoned: destructor must not reread it. This is not a
full World constructor, map-load return, save operation or destructor proof.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_FPSW
class Stop(Exception):pass
class Lifetime(Machine):
 def __init__(self,image):
  self.lifetime_ready=False;super().__init__(image);self.u.hook_add(UC_HOOK_MEM_READ,self.read)
 def read(self,u,access,address,size,value,user):
  if self.lifetime_ready and self.retired and self.table<=address<self.table+12:raise AssertionError('retired table read')
 def hook(self,u,pc,size,user):
  if self.lifetime_ready and pc in [0x50b250,0x7adda0]:
   self.trace.append(hex(pc));raise Stop()
  super().hook(u,pc,size,user)
def run(m,pc,c):
 m.lifetime_ready=False;m.fpu(pc);m.actual={};m.record_actual=True;m.trace=[];m.retired=False
 world=m.alloc(0x300);m.u.mem_write(world+0x280,b'\xa5'*8);m.w(0xb36534,world)
 m.u.reg_write(UC_X86_REG_ESI,world);m.u.reg_write(UC_X86_REG_EBX,0x12345678)
 m.u.emu_start(0x4f7d35,0x4f7d37);m.u.emu_start(0x4f804e,0x4f805a)
 constructor=[m.r(world+0x280),m.r(world+0x284)];assert constructor==[0,0]
 m.table=m.alloc(12);m.u.mem_write(m.table,b'\xa5'*12)
 count=c['count'];m.call(0x50b470,[count,world+0x284],this=m.table,callee_pop=8)
 assert m.r(world+0x284)==m.table and m.r(m.table)==count
 array=m.r(m.table+4);updatable=m.alloc(8);m.w(updatable,0xa30bb8);m.w(updatable+4,1 if c.get('scheduled',False)else 0)
 values=([updatable,updatable]+[123]*count)[:count]
 for i,v in enumerate(values):m.w(array+4*i,v)
 if c.get('returned',False):
  m.call(0x50b500,this=m.table);assert m.r(world+0x284)==0
  m.u.mem_write(m.table,b'\xa5'*12);m.retired=True
 if c.get('save_hash',False):
  save_hash=m.alloc(0x20);m.call(0x50b3e0,[world+0x280],this=save_hash,callee_pop=4);assert m.r(world+0x280)==save_hash
 m.lifetime_ready=True;error=None
 try:m.call(0x50b7a0,this=updatable)
 except Stop:error=m.trace[-1]
 assert ((m.u.reg_read(UC_X86_REG_FPSW)>>11)&7)==0
 return dict(constructor=constructor,saved_table_present=m.r(world+0x284)!=0,save_hash_present=m.r(world+0x280)!=0,error=error,trace=m.trace,values=None if m.retired else ['self'if m.r(array+4*i)==updatable else m.r(array+4*i)for i in range(count)]),m.actual.copy()
def main():
 cases=[dict(count=count,returned=returned)for count in [0,1,2,9]for returned in [False,True]]
 cases +=[dict(count=2,returned=returned,scheduled=True)for returned in [False,True]]
 cases +=[dict(count=2,returned=returned,save_hash=True)for returned in [False,True]]
 rows=[];pins={}
 for image in ['original','v14']:
  m=Lifetime(image)
  for pc in PC:
   for case in cases:
    output,visited=run(m,pc,case);pins.update(visited);rows.append(dict(image=image,precision=pc,input=case,output=output))
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 static=[]
 for va,size,label in [(0x4fd7b1,0x20,'4FC600 post-conversion epilogue calls50B500 then frees table'),(0x4f2e01,0x12,'WorldSave passes actualWorld280 backpointer to50B3E0')]:
  raw=PE0.range(va,size);assert raw==PE1.range(va,size);static.append(dict(va=hex(va),bytes=raw.hex(),observation=label))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,inspected_source_windows=static)
 p=HERE/'native-updatable-lifetime.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executed_pins=len(instructions),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
