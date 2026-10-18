"""Original42A590/429080/428F40/428480 Existing FX events.
Current Thing virtual predicates and OS thread/clock values are fixture
services. No runtime thread identity or source graph completion is implied.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent/"skirmish-20261016-fx-graph"));import native_fx_graph as graph
nm=graph.nm;ORIGINAL=nm.original
def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(ORIGINAL.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(ORIGINAL.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 world=m.alloc(0x150);manager=m.alloc(32);app=m.alloc(0x60);view=m.alloc(0xa0)
 m.w(0xb36534,world);m.w(world+0x130,manager);m.wb(manager,c['muted']);m.w(world+0xe0,c['world_time'])
 m.w(0xb364d4,app);m.w(app+0x48,c['app_thread']);m.w(0xb36538,view);m.w(view+0x8c,c['view_time'])
 list_pair=m.array([0x76543210 if c['nonempty'] else 0,0x76543210 if c['nonempty'] else 0])
 thing=m.alloc(0x200);vt=m.alloc(0x150);kingdom=m.alloc(0x30);m.w(thing,vt);m.w(thing+0x14,c['thing_id']);m.w(kingdom+0x14,c['kingdom_id'])
 for i,v in enumerate(c['position']):m.w(thing+0x20+4*i,v);m.w(0xb28114+4*i,c['global_position'][i])
 events=[]
 data=m.alloc(0x60);dvt=m.alloc(0xc0);m.w(thing+0xc,data);m.w(data,dvt);m.w(data+0x54,c['source_flags'])
 m.w(dvt+0xbc,m.stub(lambda mm:(events.append('source_BC'),mm.ret(0))))
 for offset,label,value in [(0xa8,'A8',c['kind']=='Kingdom'),(0xb4,'B4',c['kind']=='Actor')]:
  m.w(vt+offset,m.stub(lambda mm,l=label,v=value:(events.append(l),mm.ret(int(v)))))
 m.w(vt+0x108,m.stub(lambda mm:(events.append('108'),mm.ret(kingdom if c['kingdom_id'] else 0))))
 m.w(0x9f01e4,m.stub(lambda mm:(events.append('current_os_thread_id'),mm.ret(c['thread_id']))))
 command=None;output=None
 def allocate(mm):
  nonlocal command
  assert mm.arg(1)==0x6c;command=m.alloc(0x6c);m.u.mem_write(command,b'\xcd'*0x6c);mm.ret(command)
 m.seam(0x6f7810,allocate)
 def submit(mm):
  nonlocal output
  p=mm.arg(1);assert p==command
  assert m.r(p)==0x9fbdb4 and m.r(p+4)==list_pair
  assert m.r(p+0x5c)==0x9fbd78 and m.r(p+0x60)==0 and m.r(p+0x64)==p
  output=dict(words={str(off):m.r(p+off) for off in range(8,0x58,4)},byte58=m.rb(p+0x58),
      flag=mm.arg(2)&255,fxl_definition=None,fxl_parent_is_self=True,word68=m.r(p+0x68))
  mm.ret(pop=8)
 m.seam(0x50c800,submit)
 m.call(0x42a590,list_pair,[0 if c['kind']=='Null' else thing,c['flag']])
 return dict(command=output,events=events),m.visited
def main():
 rows=[];pins={}
 for pc,kind,nonempty,muted,thread,source_flags,flag in itertools.product(nm.PRECISIONS,['Object','Actor','Kingdom'],[False,True],[False,True],[41,42],[0,0x40000,0x80000,0xc0000],[0,1,255]):
  c=dict(source_flags=source_flags,kind=kind,nonempty=nonempty,muted=muted,thread_id=thread,app_thread=42,world_time=0x3f123456,view_time=0x41234567,
      thing_id=0x720001c,kingdom_id=0x100030d,position=[0x3f800001,0x40000002,0x40400003],global_position=[0xbf800004,0xc0000005,0xc0400006],flag=flag)
  a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(pins.items())])
 path=HERE/'native-existing-fx-command.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
