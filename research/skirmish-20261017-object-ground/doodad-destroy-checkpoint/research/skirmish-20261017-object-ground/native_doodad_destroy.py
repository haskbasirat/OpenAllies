"""Actual4F5490 Object/Doodad dispatch, F0 and E0/slot teardown.

Executes real constructor vtables,943800 RET4, false B4/BC/C0,4EA6F0,
50C1B0,4EC2D0/4DC9F0,4DC770 and both dynamic-component destructors,
including empty native50B250 and real50B7A0/table removal. Availability
4F4C40, world object-list4F3F10, spatial/identity4EE560 and Thing CString
4EEFD0 are explicit host transports; their portable owners have independent
tests. Positive view/grid/memory entries stop before their required child.
This is not native allocator/CString, full spatial or positive-memory proof.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import Machine as Base,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ECX,UC_X86_REG_EIP

class Stop(Exception):pass
class Machine(Base):
 def __init__(self,image):
  self.ready=False;super().__init__(image);self.ready=True
 def hook(self,u,pc,size,user):
  if self.ready:
   name={0x4f4c40:'availability',0x4f3f10:'object_list',0x4ee560:'detach_identity',0x4eefd0:'thing_string'}.get(pc)
   if name:
    if name in ['availability','object_list']:assert self.arg()==self.object
    else:assert u.reg_read(UC_X86_REG_ECX)==self.object
    self.trace.append(name)
    self.ret(8 if name=='availability' else 4 if name in ['object_list','detach_identity'] else 0);return
   if pc in [0x4f0390,0x43db80,0x4ebb80]:
    self.trace.append({0x4f0390:'positive_grid',0x43db80:'positive_view',0x4ebb80:'positive_memory'}[pc]);raise Stop()
   if pc==0x6f7430:
    p=self.arg();self.trace.append('free_object'if p==self.object else 'free_slot'+str(self.parts.index(p))if p in self.parts else 'free_other')
    self.ret();return
   if pc in [0x943800,0x4ea6f0,0x50c1b0,0x4ec2d0,0x4dc9f0,0x5903d0,0x5901a0,0x58fe70,0x50b7a0]:
    self.trace.append(hex(pc))
  super().hook(u,pc,size,user)

def run(m,pc,c):
 m.ready=False;m.fpu(pc);m.trace=[];m.actual={};m.record_actual=True
 world=m.alloc(0x300);fx=m.alloc(0x10);obj=m.alloc(0x180);registry=m.alloc(0x60010)
 m.object=obj;m.parts=[];m.w(0xb36534,world);m.w(0xb364c0,registry);m.w(world+0x130,fx)
 m.w(fx+4,1 if c.get('view')else 0);m.w(world+0x280,0);m.w(world+0x284,0)
 m.w(obj,0xa28e90 if c['class']==9 else 0xa2a2a0);m.w(obj+8,c['flags']);m.w(obj+0x50,0x10007)
 m.w(obj+0x4c,c.get('grid',0));m.w(obj+0x58,1 if c.get('memory')else 0)
 for slot in range(2):
  if c['slots']&(1<<slot):
   p=m.alloc(0x30);m.parts.append(p);m.w(p,0xa40e04 if slot==0 else 0xa40dbc);m.w(p+4,obj);m.w(p+0xc,0xa30bb8);m.w(p+0x10,0);m.w(obj+0x60+slot*4,p)
 if c['table']!='absent':
  table=m.alloc(12);m.call(0x50b470,[len(m.parts)*2,world+0x284],this=table,callee_pop=8);array=m.r(table+4)
  for i,p in enumerate(m.parts):m.w(array+i*8,p+0xc);m.w(array+i*8+4,p+0xc)
  if c['table']=='returned':m.call(0x50b500,this=table);m.u.mem_write(table,b'\xa5'*12)
 else:array=None
 m.ready=True;error=None
 try:m.call(0x4f5490,[obj],this=world,callee_pop=4)
 except Stop:error=m.trace[-1]
 return dict(flags=m.r(obj+8),slots=[m.r(obj+0x60+4*i)!=0 for i in range(2)],trace=list(m.trace),error=error,
  saved_slots=None if c['table']!='live'else ['slot'+str(m.parts.index(m.r(array+4*i)-0xc))if m.r(array+4*i)else None for i in range(len(m.parts)*2)]),dict(m.actual)

def main():
 rows=[];pins={}
 cases=[dict(**{'class':cl},slots=slots,flags=flags,table=table)for cl in [2,9]for slots in ([0]if cl==2 else range(4))for flags,table in itertools.product([0,0x4000000],['absent','live','returned'])]
 cases +=[dict(**{'class':cl},slots=3 if cl==9 else 0,flags=0,table='absent',**extra)for cl in [2,9]for extra in [dict(view=True),dict(grid=1),dict(memory=True)]]
 for image in ['original','v14']:
  m=Machine(image)
  for pc in PC:
   for case in cases:
    out,actual=run(m,pc,case);pins.update(actual);rows.append(dict(image=image,precision=pc,input=case,output=out))
    assert out['flags']==case['flags']|2
    if not out['error']:assert not any(out['slots'])and out['trace'][-1]=='free_object'
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 tables=[]
 for vt,offsets in [(0xa2a2a0,[0x2c,0xb0,0xb4,0xbc,0xc0,0xe0,0xf0]),(0xa28e90,[0x2c,0xb0,0xb4,0xbc,0xc0,0xe0,0xf0]),(0xa40e04,[0,8]),(0xa40dbc,[0,8])]:
  for off in offsets:
   raw=PE0.range(vt+off,4);assert raw==PE1.range(vt+off,4);tables.append(dict(va=hex(vt+off),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,dispatch=tables)
 path=HERE/'native-doodad-destroy.json'
 if '--create'in sys.argv:
  with path.open('x')as out:json.dump(doc,out,indent=2);out.write('\n')
 else:assert json.loads(path.read_bytes())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),dispatch=len(tables),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
