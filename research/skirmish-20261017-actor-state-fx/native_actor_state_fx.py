"""Native4D0A00 scan plus4293C0/4286E0 state-event construction.
Real CreateVFX428360 and Sound6A71F0 predicates execute. Thing virtual identity,
OS ID, allocator and final50C800 are explicit services; no active battle claim.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
ORIGINAL=nm.original

def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(ORIGINAL.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(ORIGINAL.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 world=m.alloc(0x150);manager=m.alloc(32);app=m.alloc(0x60);view=m.alloc(0xa0)
 m.w(0xb36534,world);m.w(world+0x130,manager);m.wb(manager,c['muted']);m.w(world+0xe0,c['world_time'])
 m.w(0xb364d4,app);m.w(app+0x48,c['app_thread']);m.w(0xb36538,view);m.w(view+0x8c,c['view_time'])
 thing=m.alloc(0x200);vt=m.alloc(0x150);kingdom=m.alloc(0x30);m.w(thing,vt);m.w(thing+0x14,c['thing_id']);m.w(kingdom+0x14,c['kingdom_id'])
 for i,v in enumerate(c['position']):m.w(thing+0x20+4*i,v);m.w(0xb28114+4*i,c['global_position'][i])
 events=[]
 for offset,label,value in [(0xa8,'A8',c['kind']=='Kingdom'),(0xb4,'B4',c['kind']=='Actor')]:
  m.w(vt+offset,m.stub(lambda mm,l=label,v=value:(events.append(l),mm.ret(int(v)))))
 m.w(vt+0x108,m.stub(lambda mm:(events.append('108'),mm.ret(kingdom if c['kingdom_id'] else 0))))
 m.w(0x9f01e4,m.stub(lambda mm:(events.append('current_os_thread_id'),mm.ret(c['thread_id']))))
 nodes=[];indices={};head=0;tail=0
 for i,n in reversed(list(enumerate(c['nodes']))):
  p=m.alloc(0x70);indices[p]=i;nodes.insert(0,p);nvt=m.alloc(0x30);m.w(p,nvt);m.w(p+0x34,n['state'])
  source=m.alloc(0x30);m.w(p+0x3c,source);m.w(source+0x24,n.get('class',0));m.wb(p+0x64,n.get('duration',False))
  m.w(nvt+0x20,0x428360 if n['kind']=='create' else 0x6a71f0)
  link=m.array([p,head]);head=link
  if tail==0:tail=link
 pair=m.array([head,tail])
 for va in [0x428360,0x6a71f0]:m.observe(va,lambda mm:events.append(['predicate',indices[mm.ecx()]]))
 from unicorn import UC_HOOK_MEM_READ
 from unicorn.x86_const import UC_X86_REG_EIP
 def read(u,access,address,size,value,_):
  if u.reg_read(UC_X86_REG_EIP)==0x4d0a31:events.append(['state',indices[address-0x34]])
 m.u.hook_add(UC_HOOK_MEM_READ,read)
 command=None;output=None
 def allocate(mm):
  nonlocal command
  assert mm.arg(1)==0x6c;command=m.alloc(0x6c);m.u.mem_write(command,b'\xcd'*0x6c);mm.ret(command)
 m.seam(0x6f7810,allocate)
 def submit(mm):
  nonlocal output
  p=mm.arg(1);assert p==command and mm.arg(2)==0
  assert m.r(p)==0x9fbdb4 and m.r(p+4)==pair
  assert m.r(p+0x5c)==0x9fbd78 and m.r(p+0x60)==0 and m.r(p+0x64)==p
  output=dict(words={str(off):m.r(p+off)for off in range(8,0x58,4)},byte58=m.rb(p+0x58),flag=0,fxl_definition=None,fxl_parent_is_self=True,word68=m.r(p+0x68));mm.ret(pop=8)
 m.seam(0x50c800,submit)
 result=m.call(0x4d0a00,thing,[pair,c['state'],c['active']])&255
 return dict(matched=bool(result),command=output,events=events),m.visited

def cases():
 # Explicit immutable root-list order, not declaration order.
 shapes=[[],[dict(kind='sound',state=1)]]
 for k,duration in itertools.product([0,1,2,3],[False,True]):shapes.append([dict(kind='create',state=1,**{'class':k,'duration':duration})])
 shapes.append([dict(kind='sound',state=1),dict(kind='create',state=1,**{'class':1,'duration':False})])
 shapes.append([dict(kind='create',state=0,**{'class':1,'duration':False}),dict(kind='create',state=1,**{'class':1,'duration':False})])
 for nodes,state,active,muted,kind,thread in itertools.product(shapes,[0,1,2],[0,1],[False,True],['Actor','Kingdom'],[41,42]):
  yield dict(nodes=nodes,state=state,active=active,muted=muted,kind=kind,thread_id=thread,app_thread=42,
   world_time=0x3f123456,view_time=0x41234567,thing_id=0x720001c,kingdom_id=0x100030d,
   position=[0x3f800001,0x40000002,0x40400003],global_position=[0xbf800004,0xc0000005,0xc0400006])

def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases():
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
   pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(v),bytes=b.hex())for v,b in sorted(pins.items())])
 path=HERE/'native-actor-state-fx.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
