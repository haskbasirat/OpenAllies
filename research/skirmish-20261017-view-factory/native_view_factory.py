"""Native4F0750 -> real50CAB0(empty pending)/50C220/43D160/43AC20/43C420/4EFBE0.
Source45DAF0 executes over explicit fixture Data/category/mask fields. Actual
factory switch/allocation/virtual dispatch/list links/publication execute. Derived
constructors, Init/destructor, profiler/log and successful allocation are named
fixture services; no real model/derived Init/GPU or view readiness is claimed.
Required callbacks stop before their omitted body; prior writes remain visible.
"""
from pathlib import Path
import sys,json,itertools,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EDI,UC_X86_REG_EBX,UC_X86_REG_ESI
import capstone
class Required(Exception):pass
md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_32)
TABLE={}
for k in range(2,27):
 target=struct.unpack('<I',nm.original.range(0x43b154+4*(k-2),4))[0];ins=list(md.disasm(nm.original.range(target,80),target))
 size=int(ins[0].op_str,16);calls=[i for i in ins if i.mnemonic=='call'];TABLE[k]=(size,int(calls[1].op_str,16))

def cases():
 t=lambda slot,kind=2,**kw:dict(dict(slot=slot,generation=2,kind=kind,mask=0,link=0,init=True,flags=0),**kw)
 out=[dict(name='empty',things=[],kingdoms=[])]
 out.append(dict(name='ordered_repeated_and_old',things=[t(17,0),t(2,2,flags=1<<20),t(11,3,link=901)],kingdoms=[2,None,17]))
 for kind in [0,1,*range(2,28),0xffffffff]:out.append(dict(name='factory_'+str(kind),things=[t(3,kind)],kingdoms=[]))
 for phase,view in itertools.product([0,5,6],[False,True]):out.append(dict(name=f'gates_{phase}_{view}',phase=phase,view=view,things=[t(5)],kingdoms=[None,5]))
 for init,flags,old_update in itertools.product([False,True],[0,1<<20],[False,True]):out.append(dict(name=f'init_list_{init}_{flags}_{old_update}',things=[dict(t(6),init=init,flags=flags)],kingdoms=[],old_update=old_update))
 for fail in ['begin','attach','kingdom_count','kingdom','phase','attached','source','factory','construct','init','destroy','update','publish','next','has','final']:
  out.append(dict(name='required_'+fail,things=[dict(t(8),init=fail!='destroy',flags=1<<20)],kingdoms=[None]if fail=='kingdom' else[],fail=fail))
 out.append(dict(name='allocation_null',things=[t(9)],kingdoms=[],allocation_null=True))
 out.append(dict(name='mask_selects_character',things=[dict(t(4,0),mask=4)],kingdoms=[]))
 out.append(dict(name='negative_kingdom_count',things=[t(1,0)],kingdoms=[1],count=-1))
 out.append(dict(name='reread_shrinking_kingdoms',things=[t(3),t(4)],kingdoms=[3,4],after_init_count=0))
 return [dict(phase=3,view=True,old_update=False,**c)if not('phase'in c or'view'in c or'old_update'in c)else dict({'phase':3,'view':True,'old_update':False},**c)for c in out]

def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 world=m.alloc(0x150);manager=m.alloc(0x30);view=m.alloc(0x230);idm=m.alloc(0x60010);slots=m.alloc(max(16,4*len(c['kingdoms'])))
 m.w(0xb36534,world);m.w(0xb364c0,idm);m.w(world+0x130,manager);m.w(world+0xd8,c['phase']);m.w(world+0x148,slots);m.w(world+0x14c,c.get('count',len(c['kingdoms'])))
 things={};data={};spec={};labels={0:None};views={};events=[];allocations=[];reg_started=False;final=None;current_factory=None
 for row in c['things']:
  actor=(row['generation']<<16)|row['slot'];p=m.alloc(0x20);d=m.alloc(0x180);category=m.alloc(0x24)
  m.w(p+4,d);m.w(p+0xc,row['link']);m.w(p+0x14,actor);m.w(d,0xa05540);m.w(d+0x18,category);m.w(category+0x20,row['kind']);m.w(d+0x174,row['mask'])
  things[actor]=p;data[d]=actor;spec[actor]=row;labels[p]=actor
  m.w(idm+0x20004+row['slot']*4,p)
 m.w(idm+0x6000c,max([r['slot']for r in c['things']],default=0))
 for i,slot in enumerate(c['kingdoms']):m.w(slots+4*i,0 if slot is None else things[(2<<16)|slot])
 if c['old_update']:
  n=m.alloc(8);m.w(n,0xfeed0001);m.w(n+4,0);m.w(view+0x9c,n);m.w(view+0xa0,n);labels[0xfeed0001]='prior'
 def event(value):
  events.append(value)
  if c.get('fail')==value[0]:raise Required(value[0])
 def begin(mm):event(['begin']);mm.ret(123)
 m.seam(0x59aba0,begin)
 def log(mm):
  nonlocal final
  if mm.arg(5)==0xa2ad08:
   final=[mm.arg(6),mm.arg(7)];event(['final',*final])
  mm.ret()
 m.seam(0x404f60,log)
 def attach(mm):event(['attach'])
 m.observe(0x50cab0,attach)
 def getter(mm):
  actor=data[mm.ecx()];r=spec[actor];value=4 if r['mask']&4 else r['kind'];event(['source',actor,value])
 m.observe(0x45daf0,getter)
 def factory(mm):
  nonlocal current_factory
  current_factory=labels[mm.arg(2)];event(['factory',current_factory])
 m.observe(0x43ac20,factory)
 def initialize(mm):
  label=labels[mm.ecx()];actor=views[mm.ecx()]['actor'];event(['init',label]);
  if 'after_init_count'in c:m.w(world+0x14c,c['after_init_count'])
  mm.ret(int(spec[actor]['init']))
 def destroy(mm):event(['destroy',labels[mm.ecx()],mm.arg(1)]);mm.ret(0,4)
 vt=m.alloc(0xd0);m.w(vt+0xc8,m.stub(initialize));m.w(vt+0xc4,m.stub(destroy))
 def allocate(mm):
  size=mm.arg(1)
  if size==8:event(['update',labels[m.u.reg_read(UC_X86_REG_EDI)]]);p=m.alloc(size);mm.ret(p);return
  allocations.append(size)
  if c.get('allocation_null'):mm.ret(0);return
  p=m.alloc(size);m.u.mem_write(p,b'\xcd'*size);label='v'+str(len(views)+1);labels[p]=label;views[p]={'actor':current_factory,'size':size}
  mm.ret(p)
 m.seam(0x6f7810,allocate)
 for kind,(size,ctor)in TABLE.items():
  def construct(mm,k=kind,sz=size):
   p=mm.ecx();actor=labels[mm.arg(2)];assert mm.arg(1)in data and data[mm.arg(1)]==actor;assert views[p]['size']==TABLE[k][0]
   event(['construct',labels[p],k,sz]);m.w(p,vt);m.w(p+0x54,spec[actor]['flags']);mm.ret(p,8)
  m.seam(ctor,construct)
 def publishing(mm):
  actor=labels[m.u.reg_read(UC_X86_REG_ESI)];value=m.u.reg_read(nm.UC_X86_REG_EAX);event(['publish',actor,labels[value]])
 m.observe(0x43d188,publishing)
 def null_init(mm):
  if m.u.reg_read(UC_X86_REG_EDI)==0:raise Required('null43C426')
 m.observe(0x43c426,null_init)
 def next_thing(mm):event(['next',m.r(mm.arg(1))])
 m.observe(0x4efbe0,next_thing)
 def registry_begin(mm):
  nonlocal reg_started
  reg_started=True
 m.observe(0x4f07f3,registry_begin)
 def read(u,access,a,size,value,_):
  if a==world+0x14c:event(['kingdom_count',struct.unpack('<i',m.u.mem_read(a,4))[0]])
  elif a==world+0xd8:event(['phase',m.r(a)])
  elif a==manager+4:event(['attached',m.r(a)!=0])
  elif slots<=a<slots+4*len(c['kingdoms'])and size==4:event(['kingdom',(a-slots)//4,labels[m.r(a)]])
  else:
   for actor,p in things.items():
    if a==p+0xc:event(['has',actor,m.r(a)!=0]);break
 m.u.hook_add(UC_HOOK_MEM_READ,read)
 stopped=None
 try:m.call(0x4f0750,world,[view if c['view'] else 0])
 except Required as e:stopped=str(e)
 nodes=[];n=m.r(view+0x9c)
 while n:nodes.append(labels[m.r(n)]);n=m.r(n+4);assert len(nodes)<100
 out=dict(events=events,links={str(a):labels.get(m.r(p+0xc),m.r(p+0xc))for a,p in things.items()},attached=m.r(manager+4)!=0,
  update_order=nodes,tail=labels.get(m.r(m.r(view+0xa0)),None)if m.r(view+0xa0)else None,
  allocations=allocations,views=[dict(label=labels[p],actor=v['actor'],size=v['size'],flags=m.r(p+0x54))for p,v in views.items()],
  stopped=stopped,final=final,registry_started=reg_started)
 return out,m.visited

def main():
 rows=[];pins={}
 for pc,c in itertools.product(nm.PRECISIONS,cases()):
  a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b,(pc,c,a,b);pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,b in pins.items():assert nm.original.range(va,len(b))==nm.modified.range(va,len(b))==b
 doc=dict(schema=1,scope=__doc__,original_sha256=nm.sha(nm.original.b),v14_sha256=nm.sha(nm.modified.b),rows=rows,native_executions=len(rows)*2,
  factory_table={str(k):dict(size=s,ctor=hex(a))for k,(s,a)in TABLE.items()},instructions=nm.instructions(pins))
 path=HERE/'native-view-factory.json'
 if '--write'in sys.argv:
  with path.open('x')as f:json.dump(doc,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=len(rows)*2,pins=len(pins),sha256=nm.sha(path.read_bytes()))))
if __name__=='__main__':main()
