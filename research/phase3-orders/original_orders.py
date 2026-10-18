"""Original packet read/decree dispatch/history fixtures; no native process.

Unchanged x86 handles readers, registry lookup, dispatch loop, client one-entry
boundary and history window helpers. Explicit synthetic handlers consume only
chosen bit widths; they are not invented game decree payloads.
"""
from pathlib import Path
import csv,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/engine-baseline'))
sys.path.insert(0,str(GAME/'reconstruction/tools/python-packages'))
from audit_baseline import PE
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
PE0=PE(GAME/'AA.exe');PE1=PE(GAME/'reconstruction/native-game/runtime-expanded-v14/AA.exe')
sha=lambda b:hashlib.sha256(b).hexdigest()
assert sha(PE0.b)=='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
assert sha(PE1.b)=='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
FUNCTIONS=[0x40c7c0,0x40d5d0,0x411fa0,0x4b2830,0x4efb10,0x74af40,0x74b820,0x74cdb0,0x74d710,0x74ec30,0x74fd10,0x750880,0x766ce0,0x766f20,0x766f60,0x766f80,0x766f90,0x766fc0,0x96cb83,0x96d18d,0x96be33]

class Machine:
 def __init__(self):
  self.u=Uc(UC_ARCH_X86,UC_MODE_32);self.u.mem_map(0,4096)
  self.u.mem_map(PE0.base,(PE0.image_size+4095)&~4095)
  for vs,r,size,off in PE0.sections:self.u.mem_write(PE0.base+r,PE0.b[off:off+size])
  self.u.mem_map(0x2000000,0x1800000);self.u.mem_map(0x4000000,0x20000)
  self.heap=0x2200000;self.stack=0x4008000;self.stop=0x401f000
  self.admin=0x2000000;self.session=0x2001000;self.reader=0x2002000;self.out=0x2003000
  self.events=[];self.handlers={};self.handler_after={};self.gate_after=None;self.client_delta=0
  self.w(0xb36550,self.session);self.u.hook_add(UC_HOOK_CODE,self.hook)
 def alloc(self,n):
  p=self.heap;self.heap+=(max(n,1)+15)&~15;assert self.heap<0x3700000
  self.u.mem_write(p,b'\0'*max(n,1));return p
 def w(self,p,v):self.u.mem_write(p,struct.pack('<I',v&0xffffffff))
 def r(self,p):return struct.unpack('<I',self.u.mem_read(p,4))[0]
 def signed(self,p):return struct.unpack('<i',self.u.mem_read(p,4))[0]
 def arg(self,n=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+4*n)
 def ret(self,n=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+n)
 def text(self,p):
  out=b''
  while True:
   v=bytes(self.u.mem_read(p,2));p+=2
   if v==b'\0\0':return out.decode('utf-16-le')
   out+=v;assert len(out)<1000
 def packet(self,data,refs=1):
  if data is None:return 0
  p=self.alloc(len(data)+8);self.w(p,refs);self.w(p+4,len(data));self.u.mem_write(p+8,bytes(data));return p+8
 def position(self,p):return {'byte':self.r(p+8),'bit':self.r(p+12)}
 def fields(self):return {hex(o):self.r(self.admin+o) for o in [0x20c,0x248,0x24c,0x250,0x284]}
 def hook(self,u,pc,size,user):
  if pc==0x77da70:
   self.events.append({'kind':'diagnostic','name':self.text(self.arg()),'value':self.arg(1),'fields':self.fields()});self.ret(12)
  elif pc==0x855d30:raise AssertionError('native assertion: '+self.text(self.arg()))
  elif pc==0x6f7810:self.ret(value=self.alloc(self.arg()))
  elif pc==0x6f7430:self.ret()
  elif pc==0x404f60:self.ret()
  elif pc in self.handlers:
   index,width,out=self.handlers[pc];reader=self.arg();self.w(out,0)
   self.events.append({'kind':'execute','index':index,'start':self.position(reader),'fields':self.fields()})
  elif pc in self.handler_after:
   index,out=self.handler_after[pc];reader=self.arg()
   self.events.append({'kind':'payload_read','index':index,'value':self.r(out),'end':self.position(reader)})
   if self.gate_after is not None and sum(e['kind']=='execute' for e in self.events)>=self.gate_after:
    self.u.mem_write(self.session+0x24,b'\1');self.u.mem_write(self.admin+0x2e5,b'\1')
   if self.client_delta:
    self.w(self.admin+0x20c,self.r(self.admin+0x20c)+self.client_delta);self.client_delta=0
  elif pc==0x74d710:self.events.append({'kind':'decoder_entry','fields':self.fields()})
  elif pc==0x766f20:self.last_reader=self.arg()
  elif pc==0x74d7cb:
   ptr=self.r(self.r(self.u.reg_read(UC_X86_REG_ESP)+4))
   self.events.append({'kind':'decoded','index':self.entry_indices[ptr],'position':self.position(self.last_reader)})
 def call(self,pc,args=(),this=None,callee_pop=0):
  self.w(self.stack,self.stop)
  for i,x in enumerate(args):self.w(self.stack+4+4*i,x)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.admin if this is None else this)
  regs={UC_X86_REG_EBX:0xabcdef01,UC_X86_REG_ESI:0xabcdef02,UC_X86_REG_EDI:0xabcdef03,UC_X86_REG_EBP:0xabcdef04}
  for r,v in regs.items():self.u.reg_write(r,v)
  try:self.u.emu_start(pc,self.stop,count=500000)
  except Exception as e:raise RuntimeError(f'native call{pc:08x} at{self.u.reg_read(UC_X86_REG_EIP):08x}') from e
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop,hex(self.u.reg_read(UC_X86_REG_EIP))
  assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+4+callee_pop,(hex(pc),self.u.reg_read(UC_X86_REG_ESP))
  for r,v in regs.items():assert self.u.reg_read(r)==v,(hex(pc),r)
  return self.u.reg_read(UC_X86_REG_EAX)
 def registry(self,count,width,join_index=None):
  manager=self.alloc(8);table=self.alloc(count*4);self.w(manager,table);self.w(manager+4,count);self.w(0xb27a84,manager);self.entry_indices={}
  for index in range(count):
   entry=self.alloc(12);out=self.alloc(4);pc=0x2100000+index*0x100
   code=b'\x8b\x44\x24\x04'+b'\x68'+struct.pack('<I',width)+b'\x68'+struct.pack('<I',out)+b'\x50'
   code+=b'\xe8'+struct.pack('<i',0x40d5d0-(pc+len(code)+5))+b'\x83\xc4\x0c'+b'\xc3'
   self.u.mem_write(pc,code);self.handlers[pc]=(index,width,out);self.handler_after[pc+len(code)-1]=(index,out)
   name='TGC_PlayerJoinOrderDecree' if index==join_index else f'FixtureOpaque{index}'
   text=name.encode('utf-16-le')+b'\0\0';p=self.alloc(len(text));self.u.mem_write(p,text)
   self.w(entry,pc);self.w(entry+8,p);self.w(table+index*4,entry);self.entry_indices[entry]=index

def cursor_cases():
 m=Machine();data=bytes([0xa5,0x3c,0xf0,0x96,0x55,0x81,0x7e,0x19]);p=m.packet(data);rows=[]
 for byte in [0,1,2]:
  for bit in range(9):
   for width in range(33):
    m.w(m.reader+4,p);m.w(m.reader+8,byte);m.w(m.reader+12,bit);m.w(m.out,0)
    m.call(0x40d5d0,[m.reader,m.out,width]);rows.append({'start':{'byte':byte,'bit':bit},'width':width,'value':m.r(m.out),'end':m.position(m.reader)})
 counts=[]
 for count in [1,2,3,4,5,8,9,16,17,255,256,257,32768,32769,65536]:counts.append({'count':count,'bits':m.call(0x40c7c0,[count])})
 return {'data':list(data),'cases':rows,'registry_widths':counts}

def dispatch_case(name,data,count,width,position=None,diagnostics=True,session=False,admin=False,gate_after=None,join=None):
 m=Machine();m.registry(count,width,join);m.gate_after=gate_after
 m.u.mem_write(m.session+0x24,bytes([session]));m.u.mem_write(m.admin+0x2e5,bytes([admin]))
 p=m.packet(data,2);start=[-1,-1] if position is None else [position['byte'],position['bit']]
 m.call(0x74d710,[p,*start,int(diagnostics)],callee_pop=16)
 return {'name':name,'packet':data,'registry_count':count,'payload_width':width,'join_index':join,'start':position,'diagnostics':diagnostics,'initial_gate':{'session':session,'admin':admin},'gate_after':gate_after,'events':m.events,'final_refcount':m.r(p-8) if p else None}

def dispatch_cases():
 return [dispatch_case('null_packet',None,4,3),dispatch_case('empty_packet',[5],4,3),
 dispatch_case('single_join',[8],4,3,join=1),dispatch_case('four_ordered_opaque_handlers',[0xc9,0x36,0xb2],4,3),
 dispatch_case('cross_byte_indices',[0x65,0x12,0x34],8,5),
 dispatch_case('explicit_resume',[0x08,0x1b,0xee],4,3,{'byte':1,'bit':1}),
 dispatch_case('byte_boundary_resume',[3,0x17],4,3,{'byte':0,'bit':8}),
 dispatch_case('both_flags_abort_after_decode',[0xc9,0x36,0xb2],4,3,session=True,admin=True),
 dispatch_case('session_flag_alone',[0xc9,0x36,0xb2],4,3,session=True),
 dispatch_case('admin_flag_alone',[0xc9,0x36,0xb2],4,3,admin=True),
 dispatch_case('handler_sets_abort_gate',[0xc9,0x36,0xb2],4,3,gate_after=1),
 dispatch_case('diagnostics_disabled',[0xc9,0x36,0xb2],4,3,diagnostics=False),
 dispatch_case('nonpower_registry_valid',[0x10],3,3),
 dispatch_case('one_descriptor_payload_only',[0x18],1,5)]

def history_init(m,begin,end,capacity,packets):
 window=m.admin+0x214;allocation=m.alloc(4+capacity*4);m.w(allocation,capacity);array=allocation+4
 for o,v in [(0,begin),(4,end),(8,capacity),(12,array)]:m.w(window+o,v)
 refs=[]
 for index,data in zip(range(begin,end),packets):
  packet=m.packet(data);m.w(array+((index+0x80000000)%capacity)*4,packet)
  if packet:refs.append(packet)
 return refs

def history_snapshot(m):
 window=m.admin+0x214;begin=m.signed(window);end=m.signed(window+4);capacity=m.r(window+8);array=m.r(window+12)
 def packet(p):return list(m.u.mem_read(p,m.r(p-4))) if p else None
 return {'begin':begin,'end':end,'capacity':capacity,'slots':[packet(m.r(array+4*i)) for i in range(capacity)],
 'entries':[packet(m.r(array+((i+0x80000000)%capacity)*4)) for i in range(begin,end)]}

def history_cases():
 rows=[]
 for name,begin,end,capacity,operations in [
  ('extend_without_growth',0,1,4,[['end',3]]),('extend_with_growth',0,2,2,[['end',5]]),
  ('wrapped_growth',-3,0,3,[['end',2]]),('retire_then_wrap',0,3,4,[['begin',2],['end',5]]),
  ('retire_beyond_end',-2,1,4,[['begin',3],['end',4]]),('shrink_then_extend',0,3,4,[['end',1],['end',3]]),
  ('same_bounds_noop',1,3,3,[['end',3],['begin',1]])]:
  m=Machine();packets=[[8+i*8] for i in range(end-begin)];refs=history_init(m,begin,end,capacity,packets);steps=[]
  for kind,index in operations:
   m.call(0x74ec30 if kind=='end' else 0x750880,[index],this=m.admin+0x214,callee_pop=4)
   steps.append({'action':kind,'index':index,'history':history_snapshot(m),'refcounts':[m.r(p-8) for p in refs]})
  rows.append({'name':name,'begin':begin,'end':end,'capacity':capacity,'packets':packets,'steps':steps})
 return rows

def client_cases():
 rows=[]
 for name,begin,end,cap,index,packets,gate,resume,delta in [
  ('client_existing',0,2,4,0,[[8],[16]],False,None,0),
  ('client_null_entry',0,1,4,0,[None],False,None,0),
  ('client_missing_extends',0,0,4,0,[],False,None,0),
  ('client_missing_grows',0,2,2,2,[[8],[16]],False,None,0),
  ('client_negative_wrapped',-3,0,3,-2,[[8],[16],[24]],False,None,0),
  ('client_abort_still_advances',0,1,2,0,[[8]],True,None,0),
  ('client_explicit_resume',0,1,2,0,[[3,0x17]],False,{'byte':0,'bit':8},0),
  ('client_handler_changes_index',0,2,2,0,[[8],[16]],False,None,1)]:
  m=Machine();m.registry(4,3,1);refs=history_init(m,begin,end,cap,packets)
  m.w(m.admin+0x20c,index);m.w(m.admin+0x248,-1 if resume is None else resume['byte']);m.w(m.admin+0x24c,-1 if resume is None else resume['bit']);m.w(m.admin+0x250,77);m.w(m.admin+0x284,99)
  m.u.mem_write(m.session+0x24,bytes([gate]));m.u.mem_write(m.admin+0x2e5,bytes([gate]));m.client_delta=delta
  m.call(0x74fd10)
  rows.append({'name':name,'begin':begin,'end':end,'capacity':cap,'index':index,'packets':packets,'abort_gate':gate,'resume':resume,'handler_index_delta':delta,'events':m.events,'fields':m.fields(),'history':history_snapshot(m),'final_refcounts':[m.r(p-8) for p in refs]})
 return rows

def main():
 table={int(row['address'],16):row for row in csv.DictReader((GAME/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')}
 ranges={};cs=Cs(CS_ARCH_X86,CS_MODE_32)
 for address in FUNCTIONS:
  n=int(table[address]['bodyBytes']);b=PE0.range(address,n);assert b==PE1.range(address,n)
  ranges[hex(address)]={'bytes':n,'sha256':sha(b),'unchanged_in_v14':True}
  (HERE/f'{address:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}' for i in cs.disasm(b,address))+'\n')
 out={'schema':1,'passed':True,'original_sha256':sha(PE0.b),'accepted_sha256':sha(PE1.b),'producer_sha256':sha(Path(__file__).read_bytes()),'ranges':ranges,'cursor':cursor_cases(),'dispatch':dispatch_cases(),'history':history_cases(),'client':client_cases(),
 'seams':['Synthetic registered handler machine code calls actual40D5D0 for selected opaque payload width. Names/registry fixtures are not native gameplay decree definitions.','Diagnostic callback77DA70 is traced. Allocator/free/log sinks are fixture boundaries. Actual packet refcount helpers, wcscmp and CRT vector constructor/destructor iterators run.'],
 'limits':['No native network framing, authoritative payloads, game events or scheduling cadence.','Valid finite packet/registry fixtures only; invalid inputs are Rust safety errors rather than emulated native assertions/access violations.']}
 p=HERE/'orders-native-oracle.json';p.write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'passed':True,'cursor_cases':len(out['cursor']['cases']),'registry_width_cases':len(out['cursor']['registry_widths']),'dispatch_cases':len(out['dispatch']),'history_cases':len(out['history']),'client_cases':len(out['client']),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
