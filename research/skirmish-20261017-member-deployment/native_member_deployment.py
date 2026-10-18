"""Native4FACA0 transforms, same parameter writes and deployment order.
Actor virtualC/46EC70 shape outcome,4F9C10 creation, current Mind198,
warning/log/destruction are explicit required child services.409230 executes.
45D460 source-point stores execute with explicit typed node getter services;
462A81..99 constructor fragment executes with enclosing constructor EBX=0.
No successful fresh Actor/Mind/destructor or fully initialized World claim.
"""
from pathlib import Path
import sys,json,hashlib,copy,itertools,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_FPCW,UC_X86_REG_EAX
class Required(Exception):pass
def bits(v):return struct.unpack('<I',struct.pack('<f',v))[0]
def machine(image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 return m
def cases():
 base=dict(enabled=True,inside=False,created=True,mind=True,position=list(map(bits,[12.25,-2.5])),facing=list(map(bits,[1,0])),creation=list(map(bits,[1,2])),deployment=list(map(bits,[2,7])),action='',fail='')
 out=[]
 for enabled,inside,created,mind in itertools.product([False,True],repeat=4):
  c=copy.deepcopy(base);c.update(enabled=enabled,inside=inside,created=created,mind=mind);out.append(c)
 for facing,creation,deployment in itertools.product([[1,0],[0,-1],[.6,.8],[-1.3,.7]],[[0,0],[1.1,-3.7]],[[0,0],[1.1,-3.7],[1.10001,-3.7],[8.5,9.3]]):
  c=copy.deepcopy(base);c.update(facing=list(map(bits,facing)),creation=list(map(bits,creation)),deployment=list(map(bits,deployment)));out.append(c)
 for fail in ['contains','create','mind','log','destroy']:
  c=copy.deepcopy(base);c.update(fail=fail,mind=False);out.append(c)
 c=copy.deepcopy(base);c.update(inside=True,fail='warn');out.append(c)
 for action in ['shape_changes_source_pose','create_changes_source_pose_params']:
  c=copy.deepcopy(base);c['action']=action;out.append(c)
 return out
def run(c,image,pc):
 m=machine(image,pc);structure=m.alloc(0x80);member=m.alloc(0x80);defs=[m.alloc(0x400)for _ in range(2)];params=m.alloc(0x54);mind=m.alloc(4);world=m.alloc(4);shape=m.alloc(0x20)
 definitionids={d:i+1 for i,d in enumerate(defs)};events=[];error=None;result=None
 def event(label,*args):
  events.append([label,*args])
  if c['fail']==label:raise Required(label)
 for d in defs:
  for off,v in zip([0x3dc,0x3e0,0x3e4,0x3e8],c['creation']+c['deployment']):m.w(d+off,v)
  m.u.mem_write(d+0x3ec,bytes([int(c['enabled'])]))
 for off,v in zip([0x20,0x24,0x2c,0x30],c['position']+c['facing']):m.w(structure+off,v)
 m.w(structure+4,defs[0]);m.w(member+4,defs[0]);m.w(member+0x70,mind)
 for off,v in zip([8,12,16,20,24],[bits(33),bits(44),bits(55),0,0xbf800000]):m.w(params+off,v)
 m.u.mem_write(params+0x52,b'\x5a')
 def snapshot():return dict(position=[m.r(params+i)for i in [8,12,16]],direction=[m.r(params+i)for i in [20,24]],byte52=m.u.mem_read(params+0x52,1)[0])
 def change_source_pose():
  m.w(structure+4,defs[1]);m.w(structure+0x20,bits(-77));m.w(structure+0x24,bits(66));m.w(defs[1]+0x3dc,bits(-5));m.w(defs[1]+0x3e8,bits(19))
 vt=m.alloc(0x10);m.w(structure,vt);m.w(vt+0xc,m.stub(lambda mm:mm.ret(shape)))
 def contains(mm):
  event('contains',[mm.arg(1),mm.arg(2)])
  if c['action']=='shape_changes_source_pose':change_source_pose()
  assert mm.ecx()==shape;mm.ret(int(c['inside']),8)
 m.seam(0x46ec70,contains)
 def warning(mm):event('warn',1);mm.ret(0)
 m.seam(0x404ed0,warning)
 def create(mm):
  assert mm.ecx()==world and mm.arg(1)==params;event('create',snapshot())
  if c['action']=='create_changes_source_pose_params':
   change_source_pose();m.w(params+8,bits(99));m.w(params+0x14,bits(.25));m.u.mem_write(params+0x52,b'\x07')
  mm.ret(member if c['created']else 0,4)
 m.seam(0x4f9c10,create)
 def deploy(mm):
  assert mm.ecx()==mind and mm.arg(3)==structure;event('mind',2,[mm.arg(1),mm.arg(2)],1);mm.ret(int(c['mind']),12)
 vt2=m.alloc(0x19c);m.w(mind,vt2);m.w(vt2+0x198,m.stub(deploy))
 m.seam(0x404f60,lambda mm:(event('log',2),mm.ret(0)))
 def destroy(mm):assert mm.ecx()==world and mm.arg(1)==member;event('destroy',2,mm.arg(2));mm.ret(0,8)
 m.seam(0x4f6760,destroy)
 def definition(mm):event('definition',1,definitionids[m.r(structure+4)])
 m.observe(0x4faca9,definition);m.observe(0x4fad25,definition)
 m.observe(0x4facae,lambda mm:event('enabled',definitionids[mm.u.reg_read(UC_X86_REG_EAX)]))
 for va,offset in [(0x4facbd,0x3e4),(0x4facca,0x3e8),(0x4fad28,0x3dc),(0x4fad31,0x3e0),(0x4fad59,0x3e4),(0x4fad5f,0x3e8)]:
  m.observe(va,lambda mm,o=offset:event('source',definitionids[mm.u.reg_read(UC_X86_REG_EAX)],o))
 for va,offset in [(0x4fad18,0x24),(0x4fad1b,0x2c),(0x4fad1e,0x30),(0x4fad2e,0x20)]:m.observe(va,lambda mm,o=offset:event('actor',1,o))
 try:result=2 if m.call(0x4faca0,world,[params,structure])else None
 except Required as e:error=str(e)
 return dict(result=result,error=error,events=events,params=snapshot()),m.visited
def source(c,image,pc):
 m=machine(image,pc);data=m.alloc(0x20);node=m.alloc(4);out=m.alloc(8);events=[];error=None
 for i in range(5):m.w(data+4*i,0)
 def getter(mm):
  name='creation_offset'if mm.arg(2)==0xa03308 else'deployment_offset';assert mm.ecx()==node;events.append(name)
  if c['fail']==name:raise Required(name)
  pair=c['creation']if name=='creation_offset'else c['deployment']
  for i,v in enumerate(pair):m.w(mm.arg(1)+4*i,v)
  mm.ret(mm.arg(1),8)
 m.seam(0x5b4d80,getter)
 try:m.call(0x45d460,data,[node if c['present']else 0])
 except Required as e:error=str(e)
 return dict(error=error,events=events,words=[m.r(data+i)for i in [0,4,8,12]],enabled=m.u.mem_read(data+16,1)[0]),m.visited
def ctor(image,pc):
 m=machine(image,pc);d=m.alloc(0x400);m.u.mem_write(d+0x3dc,b'\xa5'*17);m.u.reg_write(UC_X86_REG_ESI,d);m.u.reg_write(UC_X86_REG_EBX,0);m.u.reg_write(UC_X86_REG_FPCW,nm.PRECISIONS[pc]);m.u.emu_start(0x462a81,0x462a9f)
 return dict(words=[m.r(d+i)for i in [0x3dc,0x3e0,0x3e4,0x3e8]],enabled=m.u.mem_read(d+0x3ec,1)[0]),m.visited
def shape_run(c,image,pc):
 m=machine(image,pc);shape=m.alloc(0x18)
 for i,v in enumerate(c['shape']):m.w(shape+4*i,v)
 value=m.call(0x46ec70,shape,c['point'])&255
 return bool(value),m.visited
def main():
 rows=[];sources=[];constructors=[];shapes=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases():
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;rows.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
  for present,fail in [(False,''),(True,''),(True,'creation_offset'),(True,'deployment_offset')]:
   c=dict(present=present,fail=fail,creation=list(map(bits,[1.1,-2.3])),deployment=list(map(bits,[3.7,4.9])))
   a,pa=source(c,'original',pc);b,pb=source(c,'v14',pc);assert a==b;sources.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
  a,pa=ctor('original',pc);b,pb=ctor('v14',pc);assert a==b==dict(words=[0]*4,enabled=0);constructors.append(dict(precision=pc,output=a));pins.update(pa);pins.update(pb)
  for kind,radius,point in itertools.product([0,1,2,3,4,0xffffffff],[0,.01,1,5],[[0,0],[1,0],[.99995,0],[.9999,0],[-2,-3],[4,5],[4.00001,5],[0,5.1],[-2.00001,-3]]):
   c=dict(shape=[kind,bits(radius),*map(bits,[-2,-3,4,5])],point=list(map(bits,point)))
   a,pa=shape_run(c,'original',pc);b,pb=shape_run(c,'v14',pc);assert a==b;shapes.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
 for va,raw in pins.items():assert raw==nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))
 doc={**nm.header(__file__,__doc__),'rows':rows,'source_rows':sources,'constructor_rows':constructors,'shape_rows':shapes,'instructions':nm.instructions(pins)}
 path=HERE/'native-member-deployment.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),source_rows=len(sources),constructors=len(constructors),shape_rows=len(shapes),runs=2*(len(rows)+len(sources)+len(constructors)+len(shapes)),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
