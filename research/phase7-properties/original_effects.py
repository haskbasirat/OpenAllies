"""Numeric property insertion/removal runs through original581630/581810.

Actual list duplicate rules, descriptor matching, aggregation, clamps and masks
execute. Heap allocation and actor/property consequence callbacks are seams.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));from original_properties import IMAGE,ACCEPTED,pe,bits
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
PROP,ACTOR,VT,DOMAIN,DATA,CACHE,MINIMUM,REGISTRY,STACK,STOP,STUB=[0x3000000+i*0x1000 for i in range(11)]
instance=lambda id:0x3100000+id*0x100
definition=lambda id:0x3200000+id*0x1000
source=lambda id:0 if id is None else 0x3300000+id*0x100
num=lambda id,index:0x3400000+id*0x1000+index*0x40
ADD_VT=0x3500000;MUL_VT=0x3501000;PROPERTY_VT=0x3502000

class Machine:
 def __init__(self,c):
  self.c=copy.deepcopy(c);self.events=[];self.visited={};self.live={};self.next_heap=0x3600000
  self.u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(self.u);self.u.mem_map(0,4096);self.u.mem_map(0x3000000,0x800000);self.u.hook_add(UC_HOOK_CODE,self.hook)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']])
  self.w(0xb36530,DATA);self.w(DATA+0x3a8,DOMAIN);self.w(DOMAIN+0x28,19);self.w(0xb39ab0,CACHE);self.w(CACHE+0x1a8,MINIMUM);self.w(MINIMUM,c['minimum']);self.w(0xb364c0,REGISTRY)
  for id in [1,2]:self.u.mem_write(REGISTRY+4+id*2,struct.pack('<H',1));self.w(REGISTRY+0x20004+id*4,source(id));self.w(source(id)+0x14,0x10000+id)
  self.w(ACTOR,VT);self.w(VT+0x108,STUB);self.call(0x5812f0,PROP,[ACTOR])
  for target,original in [(ADD_VT,0xa2d79c),(MUL_VT,0xa2d690)]:
   assert IMAGE.at(original,0x34)==ACCEPTED.at(original,0x34)
   self.u.mem_write(target,IMAGE.at(original,0x34));self.w(target+0x30,STUB+0x10)
  self.w(PROPERTY_VT+8,STUB+0x20);self.w(PROPERTY_VT+12,STUB+0x30)
  for id,d in c['definitions'].items():
   id=int(id);p=definition(id);self.w(p+0x24,d['flags']);self.w(p+0x1c,p+0x800 if d['modifiers'] else 0)
   for i,m in enumerate(d['modifiers']):
    node=p+0x800+i*8;self.w(node,num(id,i));self.w(node+4,node+8 if i+1<len(d['modifiers']) else 0);q=num(id,i)
    self.w(q,ADD_VT if m['kind']=='Add' else MUL_VT);self.w(q+8,1 if m['kind']=='Add' else 2);self.w(q+12,m['amount']);self.w(q+16,m['index']);self.w(q+20,int(m['repair']))
 def w(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def r(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+i*4)
 def ret(self,pop=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def event(self,event):
  self.events.append(event);name=event if isinstance(event,str) else next(iter(event));mutations=self.c['mutations']
  if mutations and mutations[0]['on']==name:
   m=mutations.pop(0)
   for index,value in m.get('scalars',{}).items():self.w(PROP+0x20+int(index)*4,value)
 def hook(self,u,a,n,_):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  if a==0x6f7810:
   size=self.arg();assert size<10000;p=self.next_heap;self.next_heap+=(size+255)&~255;self.ret(value=p);return
  if a==0x6f7430:
   p=self.arg()
   if p in self.live:self.event({'Released':{'handle':self.live[p]}});del self.live[p]
   self.ret();return
  if a==STUB:self.event('Allegiance');self.ret(value=source(1));return
  if a==STUB+0x10:
   q=u.reg_read(UC_X86_REG_ECX);i=self.r(q+16)
   self.event({'NumericChanged':{'kind':'Add' if self.r(q+8)==1 else 'Multiply','index':i,'amount':self.r(q+12),'repair':bool(self.r(q+20)&1) if self.r(q+8)==1 else False,'value':self.r(PROP+0x20+i*4)}});self.ret(8);return
  if a in [STUB+0x20,STUB+0x30]:
   id=self.live[u.reg_read(UC_X86_REG_ECX)];self.event({('PropertyAdded' if a==STUB+0x20 else 'PropertyRemoved'):{'handle':id}});self.ret(8 if a==STUB+0x20 else 4);return
  if a in [0x4ceac0,0x4ceb00]:self.event({('ActorAdded' if a==0x4ceac0 else 'ActorRemoved'):{'handle':self.live[self.arg()]}});self.ret(4);return
  if a==0x404f90:self.ret();return
 def call(self,entry,this,args):
  self.w(STACK,STOP);self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,this)
  for i,value in enumerate(args):self.w(STACK+4+i*4,value)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  try:self.u.emu_start(entry,STOP,count=200000)
  except Exception as e:raise RuntimeError(self.c['name']+' at '+hex(self.u.reg_read(UC_X86_REG_EIP))) from e
  assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==STACK+4+len(args)*4
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
 def snapshot(self):
  ids=[self.live[self.r(self.r(PROP+0x10)+i*4)] for i in range(self.r(PROP+0x14))];records={}
  for p,id in self.live.items():records[str(id)]={'handle':id,'definition':(self.r(p+12)-0x3200000)//0x1000,'kind':self.r(p+8),'created':self.r(p+4),'source_id':self.r(p+20),'disabled':bool(self.u.mem_read(p+16,1)[0])}
  return {'effects':ids,'scalars':[self.r(PROP+0x20+i*4) for i in range(25)],'modifier_flags':self.r(PROP+0xd8),'instances':records,'events':copy.deepcopy(self.events)}
 def run(self):
  rows=[]
  for op in self.c['operations']:
   if 'Insert' in op:
    v=op['Insert'];p=instance(v['handle']);self.live[p]=v['handle'];self.w(p,PROPERTY_VT);self.w(p+4,v['created']);self.w(p+8,v['kind']);self.w(p+12,definition(v['definition']));self.u.mem_write(p+16,bytes([v['disabled']]));self.w(p+20,v['source_id']);self.call(0x581630,PROP,[p])
   else:v=op['Remove'];self.call(0x581810,PROP,[definition(v['definition']),source(v['source'])])
   rows.append(self.snapshot())
  assert not self.c['mutations'];return rows
def cases():
 rows=[]
 def m(kind,index,amount,repair=False):return {'kind':kind,'index':index,'amount':bits(amount),'repair':repair}
 def ins(handle,prop,source_id=0x10001):return {'Insert':{'handle':handle,'definition':prop,'kind':7,'created':bits(10),'source_id':source_id,'disabled':False}}
 def rem(prop,src=1):return {'Remove':{'definition':prop,'source':src}}
 def add(name,defs,ops,minimum=.001,mutations=None):rows.append({'name':name,'definitions':{str(i+1):d for i,d in enumerate(defs)},'operations':ops,'minimum':bits(minimum),'mutations':mutations or []})
 one=lambda mods,flags=1:{'flags':flags,'modifiers':mods}
 add('rank_replace',[one([m('Add',3,1),m('Multiply',19,.05)]),one([m('Add',3,2),m('Multiply',19,.1)],2)],[ins(1,1),rem(1),ins(2,2)])
 add('duplicate_sources',[one([m('Add',3,1)])],[ins(1,1),ins(2,1,0x10002),rem(1),rem(1,2)])
 add('remove_disabled',[one([m('Add',3,1)])],[ins(1,1),ins(2,1,0x10002),rem(1,2),rem(1)])
 add('stale_source_matches_null',[one([m('Add',3,1)])],[ins(1,1,0x20001),rem(1,None)])
 add('missing_property',[one([m('Add',3,1)])],[rem(1)])
 for index in range(25):
  kind='Add' if index<13 else'Multiply';mods=[one([m(kind,index,.1)]),one([m(kind,index,-.25)],2)]
  add('index_'+str(index),mods,[ins(1,1),ins(2,2),rem(1),rem(2)])
 add('duplicate_modifier_entries',[one([m('Add',3,.1),m('Add',3,.2),m('Multiply',19,.1),m('Multiply',19,.2)])],[ins(1,1),rem(1)])
 add('negative_special',[one([m('Add',11,-10),m('Multiply',21,-10),m('Multiply',22,-3)])],[ins(1,1),rem(1)])
 add('minimum_clamp',[one([m('Multiply',19,-2),m('Multiply',21,-2)])],[ins(1,1),rem(1)],.125)
 add('flags_zero',[one([m('Add',3,1)],0)],[ins(1,1),rem(1)])
 add('zero_numeric',[one([m('Add',3,0),m('Multiply',19,0)])],[ins(1,1),rem(1)])
 add('callback_changes_other_slot',[one([m('Add',3,1),m('Multiply',19,.1)])],[ins(1,1),rem(1)],mutations=[{'on':'NumericChanged','scalars':{'19':bits(.75)}}])
 add('repair_flag_preserved',[one([m('Add',4,1,True)]),one([m('Add',4,2,False)],2)],[ins(1,1),ins(2,2),rem(1),rem(2)])
 for p in ['Single24','Double53','Extended64']:
  for row in rows:yield {**copy.deepcopy(row),'name':row['name']+'_'+p,'precision':p}
def main():
 rows=[];visited={}
 for c in cases():
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=IMAGE.at(a,n);assert raw==ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'visited_windows':windows,'seams':['heap allocation/free','allegiance virtual','numeric actor consequence virtual','property add/remove virtuals','actor property consequence callbacks','diagnostic logging']}
 path=HERE/'native-effects-fixtures.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'sequences':len(rows),'operations':sum(len(r['output']) for r in rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
