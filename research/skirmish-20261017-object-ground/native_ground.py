"""Actual4EB500 nonnull GACD orchestration, including real401060/432E40.
Actual4EE700/4CF210/5528F0 execute. Heightfield brush, doodad-clear,
variation source lookup and painting are explicit child services. Descriptor
1C is controlled caller input here, independently proved by scratch producer.
No full brush, source-loader, mission or gameplay claim.
"""
from pathlib import Path
import sys,json,hashlib,copy
P=Path(__file__).resolve().parent;R=P.parents[1]
sys.path.insert(0,str(R/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ESP
def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for _,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 obj=m.alloc(0x180);data=m.alloc(0x700);world=m.alloc(0x200);gmap=m.alloc(0x100);session=m.alloc(0x140)
 m.w(0xb36534,world);m.w(world+0x30,gmap);m.w(world+0x34,m.alloc(0x100));m.w(world+0x48,c['world48'])
 m.w(gmap+8,c['cell']);m.w(0xb36550,session);m.w(session+0xe4,c['session'])
 m.w(obj,0xa273f0);m.w(obj+4,data);m.w(obj+8,c['flags']);m.w(obj+0x14,9)
 for i,v in enumerate(c['position']):m.w(obj+0x20+4*i,v)
 for i,v in enumerate(c['facing']):m.w(obj+0x2c+4*i,v)
 for i,v in enumerate(c['shape']):m.w(data+0x38+4*i,v)
 m.w(data+0x8c,c['padding']);m.w(obj+0x90,1 if c['wall']else 0)
 vtable=m.alloc(0x180);m.u.mem_write(vtable,nm.original.range(0xa273f0,0x180));m.w(obj,vtable);m.w(vtable+0xb4,0x6a71d0 if c['actor']else 0x6a71f0)
 params=m.alloc(0x54);m.w(params,0xa25b28);m.w(params+0x38,c['mode']);m.w(params+0x4c,c['reference'])
 ids=m.alloc(0x60010);m.w(0xb364c0,ids);other=m.alloc(0x180);m.w(other+0x28,c['reference_height'])
 m.w(ids+4+7*2,0);m.w(ids+0x20004+7*4,other if c['reference_valid']else 0)
 svar=m.alloc(0x500);slope=m.array([c['slope']]);m.w(0xb39ab0,svar);m.w(svar+0x18,slope)
 events=[];radial=[]
 def vector(p,n):return [m.r(p+4*i)for i in range(n)]
 def shape(p):return dict(kind=m.r(p),radius_bits=m.r(p+4),bounds=vector(p+8,4))
 def ground(mm):events.append(dict(op='ground_height',position=vector(mm.arg(1),2),flags=mm.arg(2)));mm.ret_st0_f64(c['height64'],8)
 def edit(mm):
  settings=mm.arg(5);radial.append(mm.r(settings+0x1c))
  events.append(dict(op='edit_height',edit=dict(placement=dict(shape=shape(mm.arg(1)),facing=vector(mm.arg(2),2),position=vector(mm.arg(3),2)),
   settings_words=vector(settings,5),flags=[bool(mm.rb(settings+20)),bool(mm.rb(settings+21))],strength_bits=mm.r(settings+24),trailing_flag=bool(mm.rb(settings+32)))))
  assert mm.arg(3)==mm.arg(4);mm.ret(0,20)
 def clear(mm):
  p=mm.arg(1);events.append(dict(op='clear_doodads',placement=dict(shape=shape(mm.r(p)),facing=vector(p+4,2),position=vector(p+12,2))));mm.ret(0,4)
 def variation(mm):events.append(dict(op='ground_variation'));mm.ret(c['variation'])
 def paint(mm):
  events.append(dict(op='paint_terrain',paint=dict(placement=dict(shape=shape(mm.arg(1)),facing=vector(mm.arg(2),2),position=vector(mm.arg(3),2)),variation=mm.arg(5),deferred=bool(mm.arg(11)&255))))
  assert mm.arg(3)==mm.arg(4) and [mm.arg(i)for i in range(6,11)]==[0,0x3f800000,0,0,0];mm.ret(0,44)
 for address,fn in [(0x4f0e50,ground),(0x551140,edit),(0x4f5560,clear),(0x4ea260,variation),(0x555c10,paint)]:m.seam(address,fn)
 #One stack argument: entryESP=STACK-8; uninitialized descriptor1C=entry-8.
 m.w(nm.STACK-16,c['radial'])
 m.call(0x4eb500,obj,[params]);m.verify_v14()
 assert all(v==c['radial']for v in radial)
 return dict(events=events,radial=radial),m.visited
def cases():
 import struct
 b=nm.f32
 out=[]
 for i in range(64):
  c=dict(flags=(0x20000400 if i&1 else 0x400)|(0x2000 if i&2 else 0),world48=1 if i&4 else 0,
   cell=b(2),session=3 if i&8 else 2,position=[b(7.5),b(8.25),b(6.14)],facing=[b(.6),b(.8)],
   shape=[i%4,b(1 if i&16 else .25),b(-1),b(-.75),b(1),b(.75)],padding=b(.3 if i%5 else -3),
   wall=bool(i&32),actor=bool(i%7),mode=[0,1,5,6][i%4],reference=7 if i&8 else 0,
   reference_valid=bool(i&16),reference_height=b(4.125),slope=b(3),height64=struct.unpack('<Q',struct.pack('<d',1.234567890123))[0],
   radial=b(6.14),variation=123 if i%3 else 0)
  out.append(c)
 return out
def main():
 rows=[];pins={}
 for c in cases():
  for pc in nm.PRECISIONS:
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
   rows.append(dict(pc=pc,input=c,native=a));pins.update(pa);pins.update(pb)
 out={**nm.header(__file__,__doc__),'rows':rows,'native_executions':len(rows)*2,'executed_pins':nm.instructions(pins)}
 path=P/'native-ground-nonnull.json';encoded=json.dumps(out,indent=2)+'\n'
 if '--write'in sys.argv:
  with path.open('x')as f:f.write(encoded)
 else:assert json.loads(path.read_text())==out
 print({'rows':len(rows),'runs':len(rows)*2,'pins':len(pins),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
if __name__=='__main__':main()
