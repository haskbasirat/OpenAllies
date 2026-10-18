"""Read-only original/v14 4F9510+4F4C40 oracle; explicitly owned seams only."""
from pathlib import Path
import hashlib, json, struct, sys
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP
TABLES={1:0xa2a680,2:0xa2a2a0,3:0xa273f0,4:0xa299d8,5:0xa29780,6:0xa28a00,7:0xa28b88,9:0xa28e90}
class Stop(Exception): pass
class DuplicateTeam(Exception): pass
class Machine(h.Machine):
 def __init__(self,case,pc):
  super().__init__(pc);self.case=case;self.trace=[];self.step=0
  self.world=self.alloc(0x400);self.registry=self.alloc(0x60020);self.ptrs={}
  self.w32(0xb364c0,self.registry);self.w32(0xb36534,self.world)
  self.w8(0xb3cc40,int(case['latch']))
  self.w32(self.world+0xf0,self.alloc(59*4),59,0);self.w8(self.world+0xfc,1)
  self.teams=self.alloc(4*len(case['teams']));self.w32(self.world+0x16c,self.teams)
  self.locale=self.alloc(0x200);self.tls=self.alloc(0x200);self.w32(self.tls+0x64,self.locale);self.w32(0xb35444,self.locale)
  self.seams.update({0x6f7810:lambda m:m.ret(value=m.alloc(m.arg())),0x6f7430:lambda m:m.ret(),
   0x4012c0:lambda m:m.ret(value=m.arg()),0x401b00:self.string_copy,
   0x9738a9:lambda m:m.ret(value=m.tls),0x4bedf0:self.capture,
   0x4d62f0:self.actor,0x855d30:self.assertion,0x4f0110:self.conflict})
  for row in case['records']:
   ident=row['id'];ptr=self.alloc(0x400);self.ptrs[ident]=ptr
   self.w32(ptr,TABLES[row['class']]);self.w32(ptr+8,row.get('flags',0x81000000));self.w32(ptr+0x14,ident)
   data=self.alloc(0x60);vt=self.alloc(0x40);self.w32(data,vt);self.w32(vt+0x24,0x462e50)
   self.w8(data+0x34,int(row.get('space',False)));self.w32(ptr+4,data)
   self.w32(ptr+0x18,self.string(row.get('name','')));self.w32(ptr+0x38,row.get('team',0))
   self.w32(ptr+0xd4,0x12345678 if row.get('city',False) else 0);self.w32(ptr+0x158,row.get('required',0))
   slot=ident&65535;self.u.mem_write(self.registry+4+slot*2,struct.pack('<H',row.get('generation',ident>>16)))
   self.w32(self.registry+0x20004+slot*4,ptr)
  for i,ident in enumerate(case['teams']):self.w32(self.teams+4*i,self.ptrs.get(ident,0))
  self.u.hook_add(UC_HOOK_CODE,self.observe)
 def string(self,s):
  p=self.alloc(18+len(s)*2)+16;self.w32(p-12,len(s),len(s),100000)
  self.u.mem_write(p,s.encode('utf-16le')+b'\0\0');return p
 def string_copy(self,m):self.w32(self.ecx(),self.arg());self.ret(4,value=self.ecx())
 def event(self,label,*args):
  self.trace.append([label,*args]);self.step+=1
  if self.case.get('stop')==self.step:raise Stop()
 def capture(self,m):
  self.event('name');self.w32(self.arg(),self.r32(self.ecx()+0x18));self.ret(4)
 def actor(self,m):
  self.event('actor')
  row=self.current
  for key,offset in [('space',0x34)]:
   if key in row.get('after_actor',{}):self.w8(self.r32(self.ecx()+4)+offset,int(row['after_actor'][key]))
  if 'city' in row.get('after_actor',{}):self.w32(self.ecx()+0xd4,0x12345678 if row['after_actor']['city'] else 0)
  if 'required' in row.get('after_actor',{}):self.w32(self.ecx()+0x158,row['after_actor']['required'])
  self.ret()
 def assertion(self,m):self.event('assert');self.ret(value=int(self.case.get('assert_return',False)))
 def conflict(self,m):raise DuplicateTeam()
 def observe(self,u,pc,n,_):
  if pc==0x4f9549:self.event('clear')
  elif pc in {0x4f95aa,0x4f95ec,0x4f9627,0x4f9670,0x4f4c62,0x4f4c7c}:
   self.event('predicate',{0x4f95aa:0xbc,0x4f95ec:0xc0,0x4f9627:0xac,0x4f9670:0xb4,0x4f4c62:0xb0,0x4f4c7c:0xb4}[pc])
  elif pc==0x4f9631:self.event('team')
  elif pc==0x4f4c71:self.event('source')
  elif pc==0x4f4c8a:self.event('city')
  elif pc==0x4f4c9a:self.event('required_live')
  elif pc==0x4f4caf:self.event('required_source')
 def text(self,p):
  a=[]
  while True:
   x=struct.unpack('<H',self.u.mem_read(p,2))[0]
   if x==0:return ''.join(chr(v) for v in a)
   a.append(x);p+=2
 def list(self,offset,pointers):
  result=[];node=self.r32(self.world+offset)
  while node:
   item=self.r32(node);result.append(self.r32(item+0x14) if pointers else item);node=self.r32(node+4)
  tail=self.r32(self.world+offset+4)
  assert bool(tail)==bool(result)
  if tail:assert self.r32(tail+4)==0
  return result
 def output(self,outcome):
  names={}
  for i in range(self.r32(self.world+0xf4)):
   node=self.r32(self.r32(self.world+0xf0)+i*4)
   while node:
    s=self.text(self.r32(node));names.setdefault(s.upper(),[]).append({'name':s,'id':self.r32(node+4)});node=self.r32(node+8)
  return {'outcome':outcome,'trace':self.trace,'flags':[[r['id'],self.r32(self.ptrs[r['id']]+8)]for r in self.case['records']],
   'names':names,'areas':self.list(0x188,True),'cameras':self.list(0x178,True),'teams':[self.r32(self.r32(self.teams+i*4)+0x14)if self.r32(self.teams+i*4)else None for i in range(len(self.case['teams']))],
   'available':self.list(0x260,False),'latch':bool(self.r8(0xb3cc40))}
 def run(self):
  outcome='ok'
  try:
   for ident in self.case['operations']:
    self.current=next(r for r in self.case['records']if r['id']==ident)
    self.call(0x4f9510,self.world,[self.ptrs[ident]])
    assert self.final_esp==h.STACK+8
  except Stop:outcome='stopped'
  except DuplicateTeam:outcome='duplicate_team'
  return self.output(outcome)
def cases():
 out=[]
 def add(name,records,ops=None,teams=None,**kw):
  c=dict(name=name,records=records,operations=ops or [r['id']for r in records],teams=teams if teams is not None else [None]*4,latch=False);c.update(kw);out.append(c);return c
 for cls in TABLES:
  for space in [False,True]:add(f'class{cls}-space{int(space)}',[dict(id=0x10001,**{'class':cls},space=space,name='Army')])
 add('head-order-and-name-replacement',[dict(id=0x10003,**{'class':6},space=True,name='army'),dict(id=0x10001,**{'class':6},space=True,name='ARMY'),dict(id=0x10002,**{'class':7},space=True,name='camera')],[0x10003,0x10001,0x10002,0x10003])
 add('same-team-pointer',[dict(id=0x10001,**{'class':5},team=2,name='team')],[0x10001,0x10001])
 add('duplicate-team-prefix',[dict(id=0x10001,**{'class':5},team=1,name='old'),dict(id=0x10002,**{'class':5},team=1,name='new')])
 for latch in [False,True]:
  for answer in [False,True]:add(f'duplicate-city-latch{latch}-answer{answer}',[dict(id=0x10001,**{'class':2},space=True,name='city')],[0x10001]*3,latch=latch,assert_return=answer)
 for required in [0,0x10002,0x20002,0x10001]:
  for reqspace in [False,True]:
   for city in [False,True]:add(f'city-{required}-{reqspace}-{city}',[dict(id=0x10001,**{'class':3},city=city,required=required,name='actor'),dict(id=0x10002,**{'class':2},space=reqspace)],[0x10001])
 add('post-actor-fields-live',[dict(id=0x10001,**{'class':3},space=False,city=False,after_actor={'space':True},name='live')])
 add('post-actor-city-live',[dict(id=0x10001,**{'class':3},space=False,city=False,after_actor={'city':True},name='live')])
 # One complete mandatory-boundary traversal, then stop before each boundary.
 base=add('all-callbacks',[dict(id=0x10001,**{'class':3},city=True,required=0x10002,name='same'),dict(id=0x10002,**{'class':2},space=False)],[0x10001,0x10001],assert_return=True)
 m=Machine(base,'Double53');count=len(m.run()['trace'])
 for stop in range(1,count+1):out.append({**base,'name':f'stop-{stop}','stop':stop})
 return out
def main():
 allcases=cases();rows=[];initialization=[]
 for label,image in [('original',h.P),('v14',h.V)]:
  old=h.P;h.P=image
  try:
   for pc in h.PRECISION:
    for case in allcases:rows.append({'image':label,'precision':pc,'input':case,'output':Machine(case,pc).run()})
    for count in [0,1,4,8,9,17]:
     m=h.Machine(pc);owner=m.alloc(12);buffer=m.alloc(8*4);m.w32(owner,buffer,0,8)
     m.w32(buffer,*([0xdeadbeef]*8))
     m.seams.update({0x6f7810:lambda m:m.ret(value=m.alloc(m.arg())),0x6f7430:lambda m:m.ret()})
     m.call(0x430240,owner,[count]);assert m.final_esp==h.STACK+8
     values=m.r32(m.r32(owner),count);assert values==[0]*count
     initialization.append({'image':label,'precision':pc,'count':count,'initialized':values,'capacity':m.r32(owner+8)})
  finally:h.P=old
 for i in range(len(allcases)*3):assert rows[i]['output']==rows[i+len(allcases)*3]['output']
 pins=[]
 for cls,t in TABLES.items():
  for off in [0xac,0xb0,0xb4,0xbc,0xc0]:
   address=h.P.u32(t+off);raw=h.P.range(address,3);assert raw==h.V.range(address,3)
   assert h.P.range(t+off,4)==h.V.range(t+off,4)
   pins.append({'class':cls,'vtable':hex(t),'offset':off,'target':hex(address),'bytes':raw.hex()})
 constructor=[]
 for a,n in [(0x4f7d00,1102),(0x4fae80,1424),(0x4fca6d,30)]:
  raw=h.P.range(a,n);assert raw==h.V.range(a,n);constructor.append({'address':hex(a),'bytes':raw.hex(),'sha256':hashlib.sha256(raw).hexdigest()})
 bss=[]
 for label,image in [('original',h.P),('v14',h.V)]:
  for vs,r,raw,off in image.sections:
   if image.base+r<=0xb3cc40<image.base+r+max(vs,raw):
    assert 0xb3cc40-image.base-r>=raw
    bss.append({'image':label,'section_va':hex(image.base+r),'virtual_size':vs,'raw_size':raw,'section_offset':0xb3cc40-image.base-r,'initial_process_zero_fill':True})
 doc={'schema':1,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'cases':rows,'class_pins':pins,
  'team_initialization':initialization,'latch_pe_section':bss,
  'constructor_and_dispatch_pins':constructor,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())],
  'seams':['UTF16 capture/copy ownership; stable fixture strings','allocator/free','CRT TLS accessor supplies C locale; comparison executes','Actor4D62F0 mandatory callback (no state/grid implementation claim)','assert dialog855D30 returns explicit byte','4F0110 LoadException stops at call'],
  'scope':'4F9510 registration/name/classification and4F4C40 insert branch; actual class/source leaves and GTID reference checks; no full Init/Create/startup or live game readiness'}
 target=HERE/'native-registration.json';target.write_text(json.dumps(doc,indent=1)+'\n')
 print(json.dumps({'cases':len(rows),'inputs':len(allcases),'instructions':len(h.VISITED),'sha256':hashlib.sha256(target.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
