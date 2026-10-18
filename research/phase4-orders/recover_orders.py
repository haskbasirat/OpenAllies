"""Original x86 registry initialization and real movement-order payload fixtures.

No native process. Non-order CRT initializers, allocation and logging are seams.
Original registry constructor/append and the actual CRT initializer loop run.
"""
from pathlib import Path
import csv, hashlib, json, struct, sys
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]; GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/phase3-orders'))
from original_orders import Machine, PE0, PE1, sha
import original_orders as original
from crt_order_initializers import audit, TABLE_BEGIN, TABLE_END
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32

FUNCTIONS={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((GAME/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')}
TABLE,CRT_AUDIT=audit(PE0,sha)
_v14_table,_v14_audit=audit(PE1,sha)
assert TABLE==_v14_table and CRT_AUDIT==_v14_audit
CALLERS={int(row['initializer'],16) for row in CRT_AUDIT if row['register_sites']}

class OrdersMachine(Machine):
 def __init__(self,image=None):
  self.image=PE0 if image is None else image
  self.ctor=None;self.registrations=[];self.retired_strings=[];self.mode='registry';self.visited={};self.process_events=[]
  previous=original.PE0
  try:
   original.PE0=self.image
   super().__init__()
  finally:original.PE0=previous
  self.manager=self.alloc(32);self.vtable=self.alloc(32)
  self.w(self.manager,self.vtable);self.w(self.vtable+4,0x2110010);self.w(self.vtable+12,0x2110000);self.w(self.vtable+16,0x2110020)
  self.order_manager=self.r(0xb27a84);self.w(self.order_manager,self.alloc(4096*4));self.w(self.order_manager+4,0);self.w(self.order_manager+8,4096)
  self.nonorder=set(TABLE)-CALLERS-{0}
  self.nonorder.update(struct.unpack('<7I',self.image.range(0xb27968,28)))
  self.w(0xb35098,0)
 def string(self,text):
  b=text.encode('utf-16-le',errors='surrogatepass');p=self.alloc(len(b)+18)
  self.w(p,self.manager);self.w(p+4,len(b)//2);self.w(p+8,len(b)//2);self.w(p+12,1)
  self.u.mem_write(p+16,b+b'\0\0');return p+16
 def units(self,p):return list(struct.unpack('<%dH'%self.r(p-12),self.u.mem_read(p,self.r(p-12)*2)))
 def hook(self,u,pc,size,user):
  self.visited[pc]=size
  ecx=u.reg_read(UC_X86_REG_ECX)
  if pc in CALLERS:self.ctor=pc
  if pc in self.nonorder:self.ret()
  elif pc==0x96cab0:self.ret()
  elif pc==0x401b00:
   self.w(ecx,self.string(self.text(self.arg())));self.ret(4,ecx)
  elif pc==0x4094f0:
   p=self.arg();b=bytearray()
   while self.u.mem_read(p,1)!=b'\0':b.extend(self.u.mem_read(p,1));p+=1
   self.w(ecx,self.string(b.decode('ascii')));self.ret(4,ecx)
  elif pc==0x410970:
   dst,a,b=self.arg(),self.arg(1),self.arg(2);self.w(dst,self.string(self.text(self.r(a))+self.text(b)));self.ret(value=dst)
  elif pc==0x4108c0:
   dst,a,b=self.arg(),self.arg(1),self.arg(2);self.w(dst,self.string(self.text(self.r(a))+self.text(self.r(b))));self.ret(value=dst)
  elif pc==0x856370:self.ret(value=self.manager)
  elif pc==0x2110000:self.ret(value=self.string('')-16)
  elif pc==0x2110010:self.ret(4)
  elif pc==0x2110020:self.ret(value=self.manager)
  elif pc==0x401330:
   self.w(ecx,self.string('\0'*self.arg()));self.ret(4)
  elif pc in (0x40f8d0,0x40f950):self.ret()
  elif pc==0x766c50:self.ret(value=0)
  elif self.mode=='process' and pc==0x40eaa0:
   self.process_events.append({'kind':'can_process'});self.ret(value=int(self.can_process))
  elif self.mode=='process' and pc==0x4f8e90:
   self.process_events.append({'kind':'resolve_actor','selector':self.units(self.r(self.arg()))});self.ret(4,self.actor if self.actor_exists else 0)
  elif self.mode=='process' and pc==0x2110030:
   self.process_events.append({'kind':'allows_command','type':self.arg()});self.ret(4,int(self.actor_allows))
  elif self.mode=='process' and pc==0x4d9b60:
   self.process_events.append({'kind':'deliver','actor':self.arg(),'fields':{hex(k):self.r(ecx+k)for k in [0x4,0xc,0x10,0x14,0x18,0x1c,0x20,0x24,0x28,0x2c,0x34,0x38]},'byte_fields':{hex(k):self.u.mem_read(ecx+k,1)[0]for k in [0x30,0x3c]}});self.ret(4,int(self.accepted))
  elif pc==0x404f90:
   if self.mode=='registry':
    self.registrations.append({'id':self.arg(6),'name':self.text(self.arg(5)),
      'initializer':hex(self.ctor),'descriptor':hex(u.reg_read(UC_X86_REG_ESI)),
      'handler':hex(self.r(u.reg_read(UC_X86_REG_ESI)))})
   self.ret()
  else:super().hook(u,pc,size,user)

def registry(m):
 m.call(0x96ec76,[0]);n=m.r(m.order_manager+4)
 assert n==len(m.registrations)
 assert set(CALLERS)=={int(r['initializer'],16)for r in m.registrations}
 assert [r['id']for r in m.registrations]==list(range(n))
 assert n==358
 for i,r in enumerate(m.registrations):
  p=m.r(m.r(m.order_manager)+4*i);assert m.r(p+4)==i and m.text(m.r(p+8))==r['name']
 m.entry_indices={int(r['descriptor'],16):r['id'] for r in m.registrations}
 return m.registrations

def payload_cases(m):
 m.mode='payload';rows=[]
 cases=[('',[0,0,0]),('usa_infantry',[0x3f800000,0xc0200000,0x42c60000]),('unit\0alias',[0x80000000,0x7f800000,0x7fc12345]),('A\ud800\udfffZ',[0xffffffff,0x12345678,0x1]),('actor_'+('x'*250),[0x43480000,0x43960000,0x40a00000])]
 for text,words in cases:
  for bit in range(9):
   source=m.alloc(20);m.w(source+4,m.string(text))
   for i,w in enumerate(words):m.w(source+8+i*4,w)
   needed=2+len(text.encode('utf-16-le',errors='surrogatepass'))+12
   p=m.packet([0]*(needed+16));m.w(m.reader+4,p);m.w(m.reader+8,0);m.w(m.reader+12,bit)
   m.call(0x60d240,[m.reader],this=source,callee_pop=4);end=m.position(m.reader)
   raw=list(m.u.mem_read(p,(end['byte']*8+end['bit']+7)//8))
   target=m.alloc(20);m.w(target+4,m.string(''))
   m.w(m.reader+8,0);m.w(m.reader+12,bit)
   m.call(0x60f760,[m.reader],this=target,callee_pop=4)
   units=m.units(m.r(source+4));assert m.units(m.r(target+4))==units
   assert [m.r(target+8+i*4)for i in range(3)]==words and m.position(m.reader)==end
   rows.append({'start':{'byte':0,'bit':bit},'raw':raw,'selector':units,'words':words,'end':end})
 return rows

def process_cases(m):
 m.mode='process';m.actor=m.alloc(0x100);v=m.alloc(0x100);m.w(m.actor,v);m.w(v+0x34,0x2110030)
 gd=m.alloc(0x300);defs=m.alloc(32);definition=m.alloc(32);m.w(0xb36530,gd);m.w(gd+0x298,defs);m.w(defs+5*4,definition)
 rows=[]
 for can,exists,allows,accepted in [(False,True,True,True),(True,False,True,True),(True,True,False,True),(True,True,True,False),(True,True,True,True)]:
  m.can_process=can;m.actor_exists=exists;m.actor_allows=allows;m.accepted=accepted;m.process_events=[]
  obj=m.alloc(20);m.w(obj+4,m.string('movement_fixture'));words=[0x43014000,0xc2980000,0x7fc12345]
  for i,w in enumerate(words):m.w(obj+8+i*4,w)
  m.call(0x61e830,this=obj)
  rows.append({'can_process':can,'actor_exists':exists,'actor_allows':allows,'dispatch_accepts':accepted,'words':words,'events':m.process_events})
 return rows

def packet_cases(m):
 rows=[]
 for name,can,exists,allows,accepted,number in [('delivered',True,True,True,True,1),('rejected_leaf_return',True,True,True,False,1),('disabled',False,True,True,True,1),('missing_actor',True,False,True,True,1),('not_allowed',True,True,False,True,1),('two_ordered_moves',True,True,True,True,2)]:
  m.can_process=can;m.actor_exists=exists;m.actor_allows=allows;m.accepted=accepted;m.process_events=[];m.events=[]
  p=m.packet([0]*1024);m.w(m.reader+4,p);m.w(m.reader+8,0);m.w(m.reader+12,3)
  orders=[]
  for i in range(number):
   source=m.alloc(20);m.w(source+4,m.string('actor_'+str(i)));words=[0x43014000+i,0xc2980000+i,0x7fc12345+i]
   for j,w in enumerate(words):m.w(source+8+j*4,w)
   m.call(0x766d00,[m.reader,0xb3e488]);m.call(0x60d240,[m.reader],this=source,callee_pop=4)
   orders.append({'selector':m.units(m.r(source+4)),'words':words})
  end=m.position(m.reader);bits=end['byte']*8+end['bit'];length=(bits+7)//8
  # Same three-bit end padding contract proven in Phase3. Packet finalizer itself
  # is not claimed here; serializer bytes above are original x86 output.
  raw=bytearray(m.u.mem_read(p,length));raw[0]|=length*8-bits
  actual=m.packet(raw,refs=2)
  m.call(0x74d710,[actual,-1,-1,0],callee_pop=16)
  rows.append({'name':name,'packet':list(raw),'orders':orders,'can_process':can,'actor_exists':exists,'actor_allows':allows,'dispatch_accepts':accepted,'events':m.process_events,'decode_events':[e for e in m.events if e['kind']=='decoded'],'packet_references':m.r(actual-8)})
 return rows

def proofs(m):
 # Bind every original instruction visited, including constructors/name getters,
 # rather than assuming decompiler body lengths are contiguous.
 intervals=[]
 for a,size in sorted(m.visited.items()):
  if not PE0.base<=a<PE0.base+PE0.image_size:continue
  if intervals and intervals[-1][1]>=a:intervals[-1][1]=max(intervals[-1][1],a+size)
  else:intervals.append([a,a+size])
 ranges=[]
 for a,end in intervals:
  b=PE0.range(a,end-a);assert b==PE1.range(a,end-a),hex(a)
  ranges.append({'start':hex(a),'end':hex(end),'sha256':sha(b)})
 table=PE0.range(TABLE_BEGIN,TABLE_END-TABLE_BEGIN);assert table==PE1.range(TABLE_BEGIN,TABLE_END-TABLE_BEGIN)
 return {'executed_windows':ranges,'crt_table':{'start':hex(TABLE_BEGIN),'end':hex(TABLE_END),'sha256':sha(table)},'constructor_count':len(CALLERS),'all_direct_constructor_callers_in_table':all(x in TABLE for x in CALLERS),'discovery':'Actual CRT entry-point direct branch/tail-jump audit, independent of incomplete Ghidra calls.tsv','crt_initializer_count':len(CRT_AUDIT),'discovery_sha256':sha((HERE/'crt_order_initializers.py').read_bytes())}

if __name__=='__main__':
 m=OrdersMachine();rows=registry(m)
 v14=OrdersMachine(PE1);assert registry(v14)==rows
 payloads=payload_cases(m);process=process_cases(m);packets=packet_cases(m)
 report={'schema':1,'passed':True,'source_sha256':sha(PE0.b),'v14_sha256':sha(PE1.b),'producer_sha256':sha(Path(__file__).read_bytes()),'phase3_machine_sha256':sha((ROOT/'research/phase3-orders/original_orders.py').read_bytes()),'registry':rows,'payloads':payloads,'process':process,'packets':packets,'proof':proofs(m),
  'seams':['Non-order CRT callbacks skipped; original CRT pointer loop and all358 discovered direct order initializers/constructor/append execute separately in both executable images. No arbitrary indirect registration or whole CRT claim.','CString construction/ASCII conversion/concatenation and allocation virtuals are exact-value seams; native retain/release and assignment execute.','Movement string buffer growth is an allocation seam; actual4049A0 zero/length setup,410B50 reader,60F760 decoder and60D240 writer execute.','Global process permission, actor alias/GTID lookup, actor command-eligibility and generic component delivery are explicit callback seams. Real61E830 guard sequence,4CA760/4D9E20 construction and4CA2D0 cleanup execute.','Packet cases execute full74D710 descriptor lookup,real60F830 movement decree handler/decoder/process and ownership. Logging/diagnostics and disabled outgoing mirror path are seams.','No request authorization, native mirror/recording, actor component pathfinding, cadence or packet finalizer implementation claimed.']}
 (HERE/'orders-native-oracle.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({'count':len(rows),'payloads':len(payloads),'process':len(process),'packets':len(packets),'native_windows':len(report['proof']['executed_windows']),'sha256':sha((HERE/'orders-native-oracle.json').read_bytes())}))
 (HERE/'registry-exploration.json').write_text(json.dumps(rows,indent=2)+'\n')
 (HERE/'payload-exploration.json').write_text(json.dumps({'payloads':payloads,'process':process},indent=2)+'\n')
