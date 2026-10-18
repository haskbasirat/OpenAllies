"""Bounded original definition store, name lookup, set fallback and cue-link evidence."""
from pathlib import Path
import json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified,sha
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
F=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
def text(m,p):
 raw=bytearray()
 for i in range(4096):
  c=bytes(m.u.mem_read(p+i*2,2))
  if c==b'\0\0':return raw.decode('utf-16-le')
  raw+=c
 raise ValueError('unterminated source string')
def puttext(m,p,s):m.u.mem_write(p,s.encode('utf-16-le')+b'\0\0')

def stores():
 rows=[]
 for precision,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  for values in [[F(1),F(.1),F(2)],[F(.75),F(.6),F(1.33333)],[F(3),F(0),F(1)]]:
   m=Machine(cw);m.u.mem_map(0,4096);m.reset(0);m.w(m.STACK+4,m.STATE+0x800);m.w(m.STACK+8,m.STATE+0x900)
   stubs={};calls=[]
   for i,(name,value)in enumerate(zip(['length','key_ratio','walk_distance'],values)):
    address=m.STATE+0x100+i*0x20;m.w(address+0x10,value);m.u.mem_write(address,b'\xd9\x05'+struct.pack('<I',address+0x10)+b'\xc2\x04\x00');stubs[name]=address
   def hook(u,pc,size,_):
    if pc==0x5b5130:
     key=text(m,m.r(u.reg_read(UC_X86_REG_ESP)+4));calls.append(key);u.reg_write(UC_X86_REG_EIP,stubs[key])
   m.u.hook_add(UC_HOOK_CODE,hook);m.u.emu_start(0x461f70,0x461fbf,count=1000)
   rows.append(dict(precision=precision,input=values,fields=[m.r(m.STATE+o)for o in [0x10,0x14,0x2c]],calls=calls))
 return rows

def fallback():
 rows=[]
 for sets in [[],[11],[0,22,33],[11,0,33]]:
  for variant in [-1]+list(range(len(sets))):
   m=Machine();m.reset(0);m.w(m.STATE+0x1c0,len(sets));m.w(m.STATE+0x1bc,m.STATE+0x400)
   for i,v in enumerate(sets):m.w(m.STATE+0x400+i*4,v)
   m.w(m.STACK+4,variant);m.u.emu_start(0x45e690,m.STOP,count=1000)
   rows.append(dict(sets=sets,variant=variant,result=m.u.reg_read(UC_X86_REG_EAX)))
 return rows

def cues():
 rows=[];m=Machine()
 for times in [[],[F(.1)],[F(.7),F(.2),F(.7)],[F(-.1),0,F(1.5),F(.3)]]:
  m.reset(0);m.w(m.STATE+0x50,0);m.w(m.STATE+0x54,0)
  for i,time in enumerate(times):
   node=m.STATE+0x100+i*16;m.w(node,time);m.u.reg_write(UC_X86_REG_ESI,m.STATE);m.u.reg_write(UC_X86_REG_EDI,node);m.u.reg_write(UC_X86_REG_EBX,0)
   m.u.emu_start(0x4621ba,0x4621cb,count=1000)
  head=m.r(m.STATE+0x50);out=[]
  while head:out.append(m.r(head));head=m.r(head+0xc)
  rows.append(dict(input=times,linked=out))
 return rows

def lookups():
 rows=[];m=Machine();m.reset(0)
 source=[dict(name='idle',kind=0),dict(name='Attack1',kind=4),dict(name='attack1',kind=0),dict(name='ATTACK1',kind=4)]
 ptrs=[]
 for i,row in enumerate(source):
  a=m.STATE+0x1000+i*0x100;ptrs.append(a);m.w(a+0xc,a+0x20);puttext(m,a+0x20,row['name'])
 m.w(m.STATE,m.STATE+0x2000);m.w(m.STATE+4,len(ptrs))
 for i,a in enumerate(ptrs):m.w(m.STATE+0x2000+i*4,a)
 for kind in range(7):
  matches=[ptrs[i]for i,row in enumerate(source)if row['kind']==kind];array=m.STATE+0x2100+kind*0x100
  m.w(m.STATE+0xc+kind*12,array);m.w(m.STATE+0x10+kind*12,len(matches))
  for i,a in enumerate(matches):m.w(array+i*4,a)
 def hook(u,pc,size,_):
  if pc==0x96cad6:
   sp=u.reg_read(UC_X86_REG_ESP);a=text(m,m.r(sp+4)).lower();b=text(m,m.r(sp+8)).lower()
   u.reg_write(UC_X86_REG_EAX,0 if a==b else 1);u.reg_write(UC_X86_REG_EIP,m.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4)
 m.u.hook_add(UC_HOOK_CODE,hook)
 for name in ['idle','IDLE','attack1','Attack1','missing','']:
  for kind in [None,0,1,4,6]:
   #Do not call reset here: its STATE seed write would destroy the live view.
   m.u.context_restore(m.initial);m.w(m.STACK,m.STOP);m.u.reg_write(UC_X86_REG_ESP,m.STACK);m.u.reg_write(UC_X86_REG_ECX,m.STATE)
   puttext(m,m.STATE+0x3000,name)
   if kind is None:m.w(m.STACK+4,m.STATE+0x3000);entry=0x45ebb0
   else:m.w(m.STACK+4,kind);m.w(m.STACK+8,m.STATE+0x3000);entry=0x45ec00
   m.u.emu_start(entry,m.STOP,count=1000);answer=m.u.reg_read(UC_X86_REG_EAX)
   rows.append(dict(source=source,name=name,kind=kind,index=ptrs.index(answer)if answer else None))
 return rows

def type_rows():
 rows=[]
 for index,(source,looping)in enumerate([(0xa12c90,1),(0xa12c80,0),(0xa12c74,1),(0xa12c6c,0),(0xa12c5c,0),(0xa12c4c,0),(0xa12c44,0)]):
  m=Machine();m.u.mem_map(0,4096);m.reset(0);name=text(m,source)
  header=m.STATE+0x100;factory=m.STATE+0x300;vt=m.STATE+0x400;thunk=m.STATE+0x500
  m.w(header,factory);m.w(header+4,len(name));m.w(header+0xc,2);puttext(m,header+0x10,name)
  m.w(factory,vt);m.w(vt+0x10,thunk);m.u.mem_write(thunk,b'\xb8'+struct.pack('<I',factory)+b'\xc3')
  m.w(m.STACK+4,header+0x10);m.w(m.STACK+8,looping);m.u.emu_start(0x4970d0,m.STOP,count=1000)
  assert m.u.reg_read(UC_X86_REG_ESP)==m.STACK+12
  rows.append(dict(index=index,name=text(m,m.r(m.STATE)),looping=bool(m.u.mem_read(m.STATE+4,1)[0]),source_string=hex(source),source_hex=original.range(source,(len(name)+1)*2).hex()))
 return rows

def main():
 ranges={}
 for name,start,end in [('timing_getters',0x461f70,0x461fbf),('set_fallback',0x45e690,0x45e6c5),('cue_prepend',0x4621ba,0x4621cb),('name_lookup',0x45ebb0,0x45ebf3),('typed_lookup',0x45ec00,0x45ec53),('kind_constructor_rows',0x49a0b3,0x49a2a4),('type_entry',0x4970d0,0x497141),('shared_string',0x4012c0,0x4012e7)]:
  raw=original.range(start,end-start);assert raw==modified.range(start,end-start);ranges[name]=dict(start=hex(start),end=hex(end),hex=raw.hex(),sha256=sha(raw))
 report=dict(schema=1,original_sha256=sha(original.b),ranges=ranges,stores=stores(),fallback=fallback(),cues=cues(),lookups=lookups(),kinds=type_rows(),
  scope=['Timing getter fixture executes full function entry through the first3GetFixed stores; typed getter returns are explicit finite f32 leaves.',
  'Cue insertion fragment receives ESI=animation,EDI=new node,EBX=0; it proves head insertion, not allocation/FX compilation or iterator initialization.',
  'Set fallback executes45E690 unchanged. Name/type lookup executes45EBB0/45EC00 with an explicit ASCII-case-insensitive CRT comparison leaf. Canonical kind arguments are decoded from499FF0; each4970D0 entry and shared string reference executes with an explicit allocator-identity virtual leaf. Global constructor lifetime is not replayed.',
  'Full resource path expansion, KF resource acquisition and native FX compilation are not claimed.'])
 p=HERE/'native-bindings.json';p.write_text(json.dumps(report,indent=2)+'\n');print(sha(p.read_bytes()))
if __name__=='__main__':main()
