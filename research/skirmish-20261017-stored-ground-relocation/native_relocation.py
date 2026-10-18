"""4EEB40 -> real4EE5C0/+C8=4D0EE0 ->4EB820 ->50C250 composition.
Height query, tile-list update, Object ground effects, changed-cell body, log
sinks, Element and positive renderer leaves are explicit services. Snap,
position/cache stores, Org5782D0, attachment false, follower-null and native
call/read order execute. Separate existing576-step proof covers tile bodies.
"""
from pathlib import Path
import sys,json,struct,hashlib
P=Path(__file__).resolve().parent;R=P.parents[1]
sys.path.insert(0,str(R/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_EIP
class Cut(Exception):pass
def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for _,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 a=m.alloc(0x180);d=m.alloc(0x700);w=m.alloc(0x300);g=m.alloc(0x100);s=m.alloc(0x140);org=m.alloc(0xd0);element=m.alloc(0x40);ids=m.alloc(0x60010);view=m.alloc(0x500);fx=m.alloc(0x30)
 vt=m.alloc(0x180);m.u.mem_write(vt,nm.original.range(0xa273f0,0x180));m.w(a,vt);m.w(a+4,d);m.w(a+8,c['flags']);m.w(a+0x14,0x20007)
 for i,v in enumerate(c['old']):m.w(a+0x20+4*i,v)
 for i,v in enumerate([0,nm.f32(-1),0]):m.w(a+0x2c+4*i,v)
 m.w(d+0x38,1);m.w(d+0x3c,nm.f32(.25));m.w(d+0x50,1);m.w(d+0x54,nm.f32(.25))
 m.w(a+0x44,c['cache'][0]);m.w(a+0x48,c['cache'][1]);m.w(a+0x7c,org if c['org']else 0);m.w(org+4,a);m.w(org+0xbc,nm.f32(-7));m.w(org+0xc0,nm.f32(-8));m.w(a+0x80,element if c['element']else 0);m.w(element+4,a)
 m.w(0xb36534,w);m.w(w+0x30,g);m.w(g+4,1);m.w(g+8,nm.f32(2));m.w(w+0xd8,c['phase']);m.w(w+0x130,fx);m.w(fx+4,view if c['view']else 0)
 m.w(0xb36550,s);m.w(s+0xe4,c['session']);m.w(0xb364c0,ids)
 events=[];outcome='returned'
 def vector(p,n):return [m.r(p+4*i)for i in range(n)]
 def event(v,mm,pop=0,ret=0):
  events.append(v)
  if c['stop']==v['op']:raise Cut(v['op'])
  mm.ret(ret,pop)
 def ground(mm):
  events.append(dict(op='ground_height',position=vector(mm.arg(1),2),flags=mm.arg(2)))
  if c['stop']=='ground_height':raise Cut('ground_height')
  mm.ret_st0_f32(c['height'],8)
 def tiles(mm):event(dict(op='update_tiles'),mm,20)
 def obj(mm):assert mm.arg(1)==0;event(dict(op='object_effects'),mm,4)
 def cell(mm):event(dict(op='cell_changed',old=[struct.unpack('<i',struct.pack('<I',mm.arg(i)))[0]for i in [1,2]]),mm,8)
 def log(mm):
  k='Identity'if mm.u.reg_read(UC_X86_REG_EIP)==0x77da70 else 'X'if mm.arg(1)==0xa273b0 else 'Y'
  event(dict(op='diagnostic',value={k:mm.arg(2)}),mm,12)
 def el(mm):event(dict(op='element'),mm)
 def renderer(mm):event(dict(op='presentation'),mm,4)
 for va,fn in [(0x4f0e50,ground),(0x5539b0,tiles),(0x4eb500,obj),(0x4d6350,cell),(0x77da70,log),(0x77d930,log),(0x5705e0,el),(0x43b760,renderer)]:m.seam(va,fn)
 if c['org']and c['session']==3:m.observe(0x5782d0,lambda mm:events.append(dict(op='organization_xy',xy=vector(a+0x20,2))))
 #Bit12 without21 reads an uninitialized local in the native function. It is
 #outside this finite initialized-height corpus; Rust retains explicit refusal.
 try:m.call(0x4eeb40,a,c['point'])
 except Cut as e:outcome='required_'+str(e)
 m.verify_v14()
 return dict(outcome=outcome,position=vector(a+0x20,3),flags=m.r(a+8),cache=[struct.unpack('<i',struct.pack('<I',m.r(a+o)))[0]for o in [0x44,0x48]],org_xy=vector(org+0xbc,2),events=events),m.visited

def cases():
 b=nm.f32
 rows=[]
 for i in range(64):
  old=[b(4.25),b(5.25),b(7)];point=[b(4.75 if i&1 else 7.25),b(5.5 if i&2 else 8.25)]
  c=dict(flags=0x200|(1<<16 if i&4 else 0)|(1<<21 if i&8 else 0)|(1<<26 if i&16 else 0)|(1<<30 if i&32 else 0),old=old,point=point,cache=[2,2],height=b(1.25),org=bool(i&8),element=bool(i&16),session=3 if i&2 else 2,phase=6 if i%3==0 else 0,view=bool(i&32),stop=None)
  rows.append(c)
 for stop in ['ground_height','update_tiles','object_effects','cell_changed','diagnostic','element','presentation']:
  c=dict(rows[-1]);c['stop']=stop;c['phase']=0;c['point']=[b(7.25),b(8.25)];rows.append(c)
 return rows

def main():
 rows=[];pins={}
 for c in cases():
  for pc in nm.PRECISIONS:
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b,(c,pc,a,b)
   rows.append(dict(pc=pc,input=c,native=a));pins.update(pa);pins.update(pb)
 out={**nm.header(__file__,__doc__),'rows':rows,'native_executions':len(rows)*2,'executed_pins':nm.instructions(pins)}
 p=P/'native-relocation.json';text=json.dumps(out,indent=2)+'\n'
 if '--write'in sys.argv:
  with p.open('x')as f:f.write(text)
 else:assert json.loads(p.read_text())==out
 print(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest()))
if __name__=='__main__':main()
