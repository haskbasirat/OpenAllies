"""Whole native564300/5640A0, real4549C0,4D01E0/578C00,517F30,4D0900.

Only6F7810(8) is a successful allocator transport. Unsheduled positive calls
stop at required50AC60 entry; no successful queue callback is substituted.
Laboratory actor/source/component/list owners are explicit inputs. No map
startup, source loader, scheduling backend or City tick proof is claimed.
"""
from pathlib import Path
import argparse,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EAX
sha=lambda b:hashlib.sha256(b).hexdigest()

def cases():
 base=dict(target='other',flags=0,owner=True,nonmajor=0,element='absent',values=[None,None,0x3f800000,None],body=False,captureable=0,destructible=0,accepted=[],candidates=[],pending=True,stop_allocation=False)
 out=[]
 def add(name,**kw):out.append(dict(base,name=name,**kw))
 add('positive')
 for name,changes in [('null',dict(target='null')),('self',dict(target='self')),('dead',dict(flags=1)),('no-owner',dict(owner=False)),('nonmajor',dict(nonmajor=1)),('nonmajor-signed',dict(nonmajor=0x80000000)),('element-null-parent',dict(element='null')),('element-parent',dict(element='parent')),('flying',dict(flags=0x1000))]:add(name,**changes)
 for name,values in [('zero',[None]*4),('negative',[None,None,0xbf800000,None]),('signed-zero',[0x80000000,None,None,None]),('denormal',[None,None,1,None]),('negative-denormal',[None,None,0x80000001,None]),('qnan',[None,None,0x7fc01234,None]),('snan',[None,None,0x7f801234,None]),('pos-inf',[None,None,0x7f800000,None]),('neg-inf',[None,None,0xff800000,None]),('opposite-inf',[0x7f800000,0xff800000,None,None]),('pc-cancellation',[0x4b800000,0x3f800000,0xcb800000,None]),('all-components',[0x3f000000,0x3f800000,0x40000000,0x40800000]),('all-negative',[0xbf000000,0xbf800000,0xc0000000,0xc0800000])]:add(name,values=values)
 for cap,des in [(0,0),(1,0),(0,1),(1,1),(0x80,0),(0,0xff)]:add('body-%d-%d'%(cap,des),body=True,captureable=cap,destructible=des)
 for field in ['accepted','candidates']:
  for index,items in enumerate([[0x12345678],[1,0x12345678,2],[1,2,0x12345678],[1,1,2],[0x12345678,0x12345678]]):add(field+str(index),**{field:items})
 add('prepend-existing',candidates=[1,2,3])
 add('schedule-required',pending=False)
 add('schedule-required-existing',pending=False,candidates=[1,2])
 add('allocation-required',stop_allocation=True)
 add('allocation-required-existing',stop_allocation=True,candidates=[1,2])
 return out

def run(c,image,pc):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 city,city_actor,target,kingdom,source,element=[m.alloc(n)for n in[0x24,0x200,0x200,0x300,0x600,0x20]]
 target=0 if c['target']=='null' else city_actor if c['target']=='self' else target
 m.w(city+4,city_actor);m.w(city+0xc,int(c['pending']))
 def nodes(values):
  head=tail=0
  for v in reversed(values):
   p=m.alloc(8);m.w(p,v);m.w(p+4,head);head=p
   if not tail:tail=p
  return head,tail
 for offset,name in[(0x10,'accepted'),(0x18,'candidates')]:
  head,tail=nodes(c[name]);m.w(city+offset,head);m.w(city+offset+4,tail)
 if target:
  m.w(target,0xa273f0);m.w(target+4,source);m.w(target+8,c['flags']);m.w(target+0x14,0x12345678)
  m.w(target+0xe8,kingdom if c['owner'] else 0)
  m.w(target+0x80,0 if c['element']=='absent' else element);m.w(element+0x10,0x1234 if c['element']=='parent' else 0)
  m.w(target+0x60,m.alloc(0x20) if c['body'] else 0)
  for (offset,field),value in zip([(0x7c,0x6c),(0x98,0x34),(0x74,0x60),(0x88,0x14)],c['values']):
   p=m.alloc(0xe0) if value is not None else 0;m.w(target+offset,p)
   if p:m.w(p+field,value)
 m.w(kingdom+0x190,c['nonmajor']);m.wb(source+0x290,c['captureable']);m.wb(source+0x291,c['destructible'])
 events=[];terminal=['returned'];comparison=[None];allocated=[]
 reads={city+4:'city_owner',kingdom+0x190:'nonmajor',city+0x10:'accepted',city+0x18:'candidates',city+0xc:'pending'}
 if target:reads.update({target+8:'flags',target+0xe8:'owner',target+0x60:'body',target+4:'definition',target+0x14:'reference',source+0x290:'byte290',source+0x291:'byte291'})
 def read(u,access,address,size,value,data):
  if address in reads:events.append(reads[address])
 def write(u,access,address,size,value,data):
  if address==city+0x18:events.append('store_head')
  if address==city+0x1c:events.append('store_tail')
 m.u.hook_add(UC_HOOK_MEM_READ,read);m.u.hook_add(UC_HOOK_MEM_WRITE,write)
 m.observe(0x4549c0,lambda _:events.append('element'))
 m.observe(0x4d01e0,lambda _:events.append('value'))
 def classify(_):
  flags=m.u.reg_read(UC_X86_REG_EAX)&0x4500
  comparison[0]='Unordered' if flags==0x4500 else 'Equal' if flags&0x4000 else 'Less' if flags&0x100 else 'Greater'
 m.observe(0x56410b,classify)
 def alloc(_):
  assert m.arg(1)==8;events.append('allocate')
  if c['stop_allocation']:terminal[0]='allocation_required';m.u.emu_stop();return
  p=m.alloc(8);allocated.append(p);m.ret(p,0)
 m.seam(0x6f7810,alloc)
 def schedule(_):
  assert m.ecx()==city+8 and m.arg(1)==0
  events.append('schedule');terminal[0]='schedule_required';m.u.emu_stop()
 m.seam(0x50ac60,schedule)
 try:m.call(0x564300,city,[target])
 except AssertionError:
  if terminal[0]=='returned':raise
 def values(offset):
  out=[];node=m.r(city+offset);last=0
  while node:
   assert node!=last;out.append(m.r(node));last=node;node=m.r(node+4)
  assert m.r(city+offset+4)==last
  return out
 return dict(terminal=terminal[0],events=events,value_comparison=comparison[0],candidates=values(0x18),allocated=len(allocated)),m.visited

def main():
 parser=argparse.ArgumentParser();parser.add_argument('--create',action='store_true');args=parser.parse_args()
 rows=[];pins={}
 for pc in ['Single24','Double53','Extended64']:
  for case in cases():
   pair=[]
   for image in [original,modified]:
    output,visited=run(case,image,pc);pair.append(output)
    for address,raw in visited.items():
     assert raw==original.range(address,len(raw))==modified.range(address,len(raw)),hex(address)
     pins[address]=raw
   assert pair[0]==pair[1],(case,pc,pair)
   rows.append(dict(precision=pc,input=case,native=pair[0]))
 fixture=dict(scope=__doc__,original_sha256=sha(original.b),v14_sha256=sha(modified.b),rows=rows,pins=[dict(address=hex(a),bytes=b.hex())for a,b in sorted(pins.items())])
 path=HERE/'native-city-admission.json'
 if args.create:
  with path.open('x',encoding='utf-8')as f:json.dump(fixture,f,indent=2);f.write('\n')
 else:assert fixture==json.loads(path.read_text(encoding='utf-8'))
 print(json.dumps(dict(paired_rows=len(rows),separate_executions=len(rows)*2,pins=len(pins),fixture_sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
