"""Original instruction writer/decoder and guarded command-context fixtures.

No native process. Allocation/diagnostics and final gameplay calls are declared
seams; the packet fields, registry lookups and guarded process instructions run.
"""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]; GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/phase4-orders'))
from recover_orders import OrdersMachine,PE0,PE1,sha,registry
from unicorn.x86_const import *

TABLES={'command':0xb3aa58,'object':0xb39f58,'parameter':0xb3ab08,'parameter_set':0xb3ab00,'extra':0xb3aa2c}

class Machine(OrdersMachine):
 def __init__(self,counts=None):
  super().__init__();self.mode='commands';self.trace=[];self.stop_film_mutation=False
  self.counts=counts or {'command':7,'object':5,'parameter':3,'parameter_set':2,'extra':8}
  self.tables={};self.objects={};self.object_valid=True;self.slot_valid=True
  for key,address in TABLES.items():
   manager=self.alloc(48);array=self.alloc(4*self.counts[key]);self.w(address,manager);self.w(manager+0x24,array);self.w(manager+0x28,self.counts[key]);self.tables[key]=manager
   refs=[]
   for i in range(self.counts[key]):
    p=self.alloc(64);v=self.alloc(64);self.w(p,v);self.w(v+0x38,0x2110100);self.w(p+0xc,i);self.w(p+0x20,2);self.w(p+8,self.string(key+str(i)));self.w(array+4*i,p);refs.append(p)
   self.objects[key]=refs
  self.actor=self.alloc(0x200);v=self.alloc(0x180);self.w(self.actor,v);self.w(v+0xb4,0x2110110);self.w(v+0x150,0x2110120);self.w(v+0x108,0x2110130)
  self.w(self.actor+0x14,0x1234ffff);d=self.alloc(32);self.w(d+8,self.string('actor'));self.w(self.actor+4,d)
  self.actor_registry=self.alloc(0x60004);self.w(0xb364c0,self.actor_registry);self.w(self.actor_registry+0x20004+65535*4,self.actor)
  self.w(0xb36558,self.admin);self.w(self.admin+0x2c4,self.admin)
  self.client=self.alloc(0x300);self.other_client=self.alloc(0x300)
  self.players=[]
 def hook(self,u,pc,size,user):
  self.visited[pc]=size;ecx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x2110100:self.ret(value=int(self.object_valid))
  elif pc==0x2110110:self.ret(value=int(self.slot_valid))
  elif pc==0x2110130:self.ret(value=0)
  elif pc==0x2110120:
   self.trace.append({'kind':'permit','player':self.arg(),'selection':self.arg(2)})
   if self.replace_command:self.w(self.arg(1),self.replacement)
   self.ret(12,int(self.permitted))
  elif pc==0x4d1900:self.trace.append({'kind':'linked_group'});self.ret(4,self.client)
  elif pc==0x4dff30:self.trace.append({'kind':'prepare_group'});self.ret()
  elif pc==0x4d9b60:self.trace.append({'kind':'deliver','command':self.command_fields(ecx)});self.ret(4,0)
  elif pc==0x751eb0:
   self.trace.append({'kind':'stop_film'})
   if self.stop_film_mutation:self.w(self.admin+0x284,self.players[0]);self.u.mem_write(self.admin+0x2e5,b'\0')
   self.ret()
  elif pc==0x404f60:self.ret()
  elif pc==0x404f90 and self.mode!='registry':self.ret()
  elif pc==0x765e00:self.ret(value=0)
  elif pc==0x766380:
   dst=self.arg();self.w(dst,self.string('fixture_player'));self.ret(8,dst)
  elif pc==0x59aba0:self.ret(value=160)
  elif pc==0x40b000:
   # Exact-value clock seam returns double0 on x87; arguments belong to caller.
   u.mem_write(0x2110200,b'\xd9\xee\xc3');u.reg_write(UC_X86_REG_EIP,0x2110200)
  elif pc==0x4da570:self.ret(8,self.objects['command'][0])
  elif pc==0x4ed360:self.trace.append({'kind':'selection_delivery','command':self.command_fields(self.r(self.arg()))});self.ret(4)
  else:super().hook(u,pc,size,user)
 def ref(self,key,index):return 0 if index is None else self.objects[key][index]
 def index(self,key,p):return None if p==0 else self.objects[key].index(p)
 def command(self,flags,nulls=False,list_count=2):
  p=self.alloc(64);self.call(0x4d9c80,this=p);self.w(p+0xc,flags)
  for off,key in [(8,'command'),(0x14,'object'),(0x2c,'parameter'),(0x38,'extra')]:self.w(p+off,self.ref(key,None if nulls else len(self.objects[key])-1))
  for off,val in [(0x10,0xfedcba98),(0x18,0x80000000),(0x1c,0x7fc12345),(0x20,0x3f800000),(0x24,0xc0200000),(0x28,0xffffffff)]:self.w(p+off,val)
  self.u.mem_write(p+0x30,b'\x01');self.u.mem_write(p+0x3c,b'\x01')
  if not nulls:
   q=self.alloc(24);self.w(q+8,self.objects['parameter_set'][0]);a=self.alloc(4*list_count);self.w(q+0xc,a);self.w(q+0x10,list_count);self.w(q+0x14,list_count)
   for i in range(list_count):self.w(a+4*i,0 if i%2 else self.objects['object'][0])
   self.w(p+0x34,q)
  return p
 def command_fields(self,p):
  flags=self.r(p+0xc);o={'flags':flags}
  for off,key,label,bit in [(8,'command','command',6),(0x14,'object','object',1),(0x2c,'parameter','parameter',7),(0x38,'extra','extra',8)]:
   o[label]=self.index(key,self.r(p+off)) if flags>>bit&1 else None
  for off,label,bit in [(0x10,'word',0),(0x28,'scalar',4)]:o[label]=self.r(p+off) if flags>>bit&1 else 0
  for off,label,bit in [(0x18,'position',2),(0x20,'auxiliary',3)]:o[label]=[self.r(p+off),self.r(p+off+4)] if flags>>bit&1 else [0,0]
  q=self.r(p+0x34);o['parameters']=None
  if flags>>5&1 and q:o['parameters']={'definition':self.index('parameter_set',self.r(q+8)),'objects':[self.index('object',self.r(self.r(q+0xc)+4*i)) for i in range(self.r(q+0x10))]}
  o['boolean']=bool(self.u.mem_read(p+0x30,1)[0]) if flags>>9&1 else False
  o['trailing']=bool(self.u.mem_read(p+0x3c,1)[0]);return o
 def stream(self,bit):
  p=self.packet([0]*4096);self.w(self.reader+4,p);self.w(self.reader+8,0);self.w(self.reader+12,bit);return p
 def player(self,id,film,client):
  p=self.alloc(0x80);self.w(p+4,client);self.w(p+0x20,id);self.u.mem_write(p+0xe,bytes([film]));self.players.append(p);return p
 def player_list(self):
  nxt=0
  for p in reversed(self.players):n=self.alloc(8);self.w(n,p);self.w(n+4,nxt);nxt=n
  self.w(self.session+0xbc,nxt)
 def player_label(self,p):return None if p==0 else self.players.index(p)

def payloads(m):
 rows=[]
 masks=[0,*[1<<i for i in range(11)],0x5c,0x3ff,0x7ff]
 for nulls in [False,True]:
  for flags in masks:
   for bit in range(9):
    src=m.command(flags,nulls);p=m.stream(bit);m.call(0x61e6b0,[m.reader,src]);end=m.position(m.reader);raw=list(m.u.mem_read(p,(end['byte']*8+end['bit']+7)//8));out=m.alloc(64);m.call(0x4d9c80,this=out)
    m.w(m.reader+8,0);m.w(m.reader+12,bit);m.call(0x61eba0,[m.reader,out]);assert m.position(m.reader)==end
    assert m.command_fields(out)==m.command_fields(src)
    rows.append({'counts':m.counts,'start':{'byte':0,'bit':bit},'raw':raw,'end':end,'command':m.command_fields(out)})
 return rows

def slots(m):
 rows=[]
 for slot in [0,1,65535]:
  for valid in [False,True]:
   for bit in [0,3,8]:
    m.w(m.actor+0x14,0xdead0000|slot);m.slot_valid=valid;src=m.alloc(4);m.w(src,m.actor if slot else 0)
    m.w(m.actor_registry+0x20004+slot*4,m.actor if slot else 0);p=m.stream(bit);m.call(0x76a500,[m.reader,src]);end=m.position(m.reader);raw=list(m.u.mem_read(p,(end['byte']*8+end['bit']+7)//8));dst=m.alloc(4);m.w(m.reader+8,0);m.w(m.reader+12,bit);m.call(0x76a5a0,[m.reader,dst])
    rows.append({'slot':slot,'valid':valid,'start':{'byte':0,'bit':bit},'raw':raw,'end':end,'resolved':m.r(dst)==m.actor})
 return rows

def boundary_payloads(parent):
 rows=[]
 def sample(m,bit,nulls,list_count):
  src=m.command(0x7ff,nulls,list_count);p=m.stream(bit);m.call(0x61e6b0,[m.reader,src]);end=m.position(m.reader);raw=list(m.u.mem_read(p,(end['byte']*8+end['bit']+7)//8));out=m.alloc(64);m.call(0x4d9c80,this=out);m.w(m.reader+8,0);m.w(m.reader+12,bit);m.call(0x61eba0,[m.reader,out]);assert m.position(m.reader)==end;assert m.command_fields(src)==m.command_fields(out)
  rows.append({'counts':m.counts,'start':{'byte':0,'bit':bit},'raw':raw,'end':end,'command':m.command_fields(out)})
 for count in [0,1,16]:
  m=Machine(dict.fromkeys(TABLES,count))
  for bit in range(9):sample(m,bit,count==0,2)
  parent.visited.update(m.visited)
 for count in [0,1,3,8]:
  for bit in [0,3,8]:sample(parent,bit,False,count)
 return rows

def actor_process(m):
 rows=[]
 for exists,allowed,linked,replace in [(False,True,True,False),(True,False,True,False),(True,True,False,False),(True,True,True,False),(True,True,True,True)]:
  src=m.command(0x5c);m.replacement=m.command(0x401,True);m.permitted=allowed;m.replace_command=replace
  m.w(m.actor+0x164,0x101);m.u.mem_write(m.actor_registry+4+0x101*2,b'\0\0');m.w(m.actor_registry+0x20004+0x101*4,m.actor if linked else 0)
  order=m.alloc(12);m.w(order+4,src);m.w(order+8,m.actor if exists else 0);m.trace=[];m.call(0x61e220,this=order)
  rows.append({'exists':exists,'allowed':allowed,'linked':linked,'replace':replace,'initial':m.command_fields(src),'replacement':m.command_fields(m.replacement),'events':m.trace})
 return rows

def selected_process(m):
 rows=[]
 player=m.player(11,False,m.client);selection=m.alloc(0x40);m.w(player+0x50,selection);m.w(m.admin+0x284,player)
 for name,head_actor in [('empty',None),('null_first',0),('actor',m.actor)]:
  head=0
  if head_actor is not None:head=m.alloc(8);m.w(head,head_actor)
  m.w(selection+4,head);m.slot_valid=True
  src=m.command(0x5c);order=m.alloc(8);m.w(order+4,src);m.trace=[];m.call(0x61f210,this=order)
  rows.append({'name':name,'command':m.command_fields(src),'events':m.trace})
 return rows

def player_process(m):
 rows=[]
 m.players=[];m.player(17,False,m.client);m.player(17,True,m.other_client);m.player(18,False,m.other_client);m.player_list()
 for name,id,film,auth,source,mutate in [('live_owner',17,False,True,m.client,False),('live_intruder',18,False,True,m.client,False),('live_nonauthority',18,False,False,m.client,False),('film',17,True,True,m.client,False),('missing_live',99,False,True,m.client,False),('missing_film',99,True,True,m.client,False),('missing_film_callback',99,True,True,m.client,True)]:
  m.u.mem_write(m.session+0x24,bytes([film]));m.w(m.admin+0x2c4,m.admin if auth else 0);m.w(m.admin+0x280,source);m.w(m.admin+0x284,m.players[2]);m.w(m.client+0x2c,0);m.w(m.other_client+0x2c,0);m.u.mem_write(m.admin+0x2e5,b'\0');m.stop_film_mutation=mutate
  order=m.alloc(8);m.w(order+4,id);m.trace=[];m.call(0x75d710,this=order)
  rows.append({'name':name,'id':id,'film':film,'authority':auth,'source_client':0,'stop_mutates':mutate,'events':m.trace,'current_player':m.player_label(m.r(m.admin+0x284)),'client_players':[m.player_label(m.r(c+0x2c)) for c in [m.client,m.other_client]],'abort':bool(m.u.mem_read(m.admin+0x2e5,1)[0])})
 return rows

def packet_cases(m):
 m.mode='registry';registry(m);m.mode='commands';rows=[]
 selection=m.alloc(0x40);head=m.alloc(8);m.w(head,m.actor);m.w(selection+4,head)
 for player in m.players:m.w(player+0x50,selection)
 m.slot_valid=True;m.permitted=True;m.replace_command=False
 m.w(m.actor+0x14,0x1234ffff);m.w(m.actor_registry+0x20004+65535*4,m.actor)
 m.w(m.actor+0x164,0x101);m.w(m.actor_registry+0x20004+0x101*4,0)
 for name,id,descriptor,head_present in [('player_actor',17,87,True),('player_selected',17,86,True),('player_empty_selected',17,86,False)]:
  m.w(selection+4,head if head_present else 0);m.u.mem_write(m.session+0x24,b'\0');m.u.mem_write(m.admin+0x2e5,b'\0');m.w(m.admin+0x2c4,m.admin);m.w(m.admin+0x280,m.client);m.w(m.admin+0x284,0)
  command=m.command(0x5c);actor_ref=m.alloc(4);m.w(actor_ref,m.actor);src=m.alloc(4);m.w(src,id);p=m.stream(3)
  # Actual registry identities/native payload serialization; only final padding
  # framing uses the already-proved three-bit packet contract.
  m.call(0x766d00,[m.reader,0xb40938]);m.call(0x40d420,[m.reader,src,32])
  m.call(0x766d00,[m.reader,0xb3e5b8 if descriptor==86 else 0xb3e5c4]);m.call(0x61e6b0,[m.reader,command])
  if descriptor==87:m.call(0x76a500,[m.reader,actor_ref])
  end=m.position(m.reader);bits=end['byte']*8+end['bit'];n=(bits+7)//8;raw=bytearray(m.u.mem_read(p,n));raw[0]|=n*8-bits
  packet=m.packet(raw,refs=2);m.trace=[];m.events=[];m.call(0x74d710,[packet,-1,-1,0],callee_pop=16)
  rows.append({'name':name,'packet':list(raw),'id':id,'descriptor':descriptor,'command':m.command_fields(command),'actor_slot':65535 if descriptor==87 else None,'events':m.trace,'decoded':[e['index'] for e in m.events if e['kind']=='decoded'],'current_player':m.player_label(m.r(m.admin+0x284)),'packet_refs':m.r(packet-8)})
 return rows

def proof(m):
 ranges=[]
 for va,size in sorted(m.visited.items()):
  if not PE0.base<=va<PE0.base+PE0.image_size:continue
  if ranges and ranges[-1][1]==va:ranges[-1][1]+=size
  else:ranges.append([va,va+size])
 out=[]
 for a,b in ranges:
  raw=PE0.range(a,b-a);assert raw==PE1.range(a,b-a),hex(a);out.append({'start':hex(a),'end':hex(b),'sha256':sha(raw)})
 return out

if __name__=='__main__':
 m=Machine();payload=payloads(m)+boundary_payloads(m);slot=slots(m);actor=actor_process(m);selected=selected_process(m);players=player_process(m);packets=packet_cases(m)
 report={'schema':1,'passed':True,'original_sha256':sha(PE0.b),'v14_sha256':sha(PE1.b),'producer_sha256':sha(Path(__file__).read_bytes()),'payloads':payload,'slots':slot,'actor_process':actor,'selected_process':selected,'player_process':players,'packets':packets,'executed_windows':proof(m),
 'byte_window_scope':'The compatibility field executed_windows records visited native instruction addresses, including intercepted seam-entry bytes. A pinned seam entry is not evidence that its original instructions executed.',
 'seams':['Allocator/free, CString allocation, clock and diagnostic output are fixture seams.','Actual conditional command writer61E6B0/decoder61EBA0 and each registry serializer/lookup execute. Registry contents are synthetic explicit inputs; their authoring/index population is not claimed.','Actual actor-slot lookup76A5A0 runs; virtual+B4 type-validity is supplied. Object-definition virtual+38 validity is supplied.','Actual actor guarded process61E220 and linked-GTID validation44DEA0 run. Virtual+150 may replace the command; linked-group lookup/preparation and final delivery are traced gameplay leaves.','Actual selected61F210 runs; selection/gameplay handler4ED360 is an explicit leaf, not implemented formation/path/ownership behavior.','Actual current-player75D710, linked-list lookup76B8F0 and setter helpers execute. Film-stop751EB0 is a synchronous traced callback, including a state-mutation case.'],
 'limits':['Requests are decoded but no request authorization, authoritative scheduling or outgoing recording/mirror is implemented.','Malformed negative/oversized parameter counts and invalid registry references are Rust safety errors, not emulated native corruption.','This is not complete world command execution.']}
 out=HERE/'commands-native-oracle.json';out.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'counts':{k:len(report[k]) for k in ['payloads','slots','actor_process','selected_process','player_process']},'sha256':sha(out.read_bytes())}))
