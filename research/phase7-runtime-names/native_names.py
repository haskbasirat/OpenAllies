"""Original World name registration, duplicate lookup/removal and real rehash.
String ownership, allocator, thread-locale accessor and later publication are
explicit seams. Native hash and C-locale wide comparison execute unchanged.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'));from original_body import IMAGE,ACCEPTED,pe
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
WORLD,THING,VT,REF,VALUE,STACK,STOP,TLS,LOCALE=range(0x3000000,0x3009000,0x1000)
REGISTRY=0x3100000;ACTORS=0x3200000;HEAP=0x3300000
class Machine:
 def __init__(self,c):
  self.c=c;self.visited={};self.heap=HEAP;self.u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(self.u);self.u.mem_map(0,4096);self.u.mem_map(0x3000000,0x500000);self.u.hook_add(UC_HOOK_CODE,self.hook)
  self.w(0xb36534,WORLD);self.w(0xb364c0,REGISTRY);self.w(0xb35444,LOCALE);self.w(TLS+0x64,LOCALE)
  self.w(WORLD+0xf0,self.alloc(59*4));self.w(WORLD+0xf4,59);self.u.mem_write(WORLD+0xfc,b'\x01')
  self.w(THING,VT);stub=VT+0x300;self.u.mem_write(stub,b'\x31\xc0\xc3')
  for offset in [0xbc,0xc0,0xac,0xb4]:self.w(VT+offset,stub)
  for i,a in enumerate(c['actors']):
   raw=a['id'];p=ACTORS+i*0x500;self.u.mem_write(REGISTRY+4+(raw&65535)*2,struct.pack('<H',a.get('generation',raw>>16)));self.w(REGISTRY+0x20004+(raw&65535)*4,p);self.w(p, p+0x100);self.w(p+0x14,raw)
   for offset,label,k in [(0xb4,'actor',0),(0xa8,'kingdom',1)]:
    stub=p+0x300+k*16;self.w(p+0x100+offset,stub);self.u.mem_write(stub,b'\xb8'+struct.pack('<I',int(a[label]))+b'\xc3')
 def alloc(self,n):p=self.heap;self.heap+=(max(n,16)+15)&~15;assert self.heap<0x3500000;return p
 def w(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def r(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+4*i)
 def text(self,p):
  words=[]
  while True:
   word=struct.unpack('<H',self.u.mem_read(p,2))[0];p+=2
   if not word:return words
   words.append(word)
 def string(self,s):
  words=[ord(x) for x in s];p=self.alloc(18+len(words)*2)+16;self.w(p-12,len(words));self.w(p-8,len(words));self.w(p-4,100000);self.u.mem_write(p,s.encode('utf-16le')+b'\0\0');return p
 def ret(self,pop=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def hook(self,u,a,n,_):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  if a==0x6f7810:self.ret(value=self.alloc(self.arg()))
  elif a==0x6f7430:self.ret()
  elif a==0x4012c0:self.ret(value=self.arg())
  elif a==0x401b00:self.w(u.reg_read(UC_X86_REG_ECX),self.arg());self.ret(4,value=u.reg_read(UC_X86_REG_ECX))
  elif a==0x9738a9:self.ret(value=TLS) # real compare executes the explicit C-locale branch
  elif a==0x4f4c40:self.ret(8) # later spatial/publication, outside table scope
 def call(self,a,owner,args):
  self.w(STACK,STOP)
  for i,arg in enumerate(args):self.w(STACK+4+i*4,arg)
  self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,owner)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  try:self.u.emu_start(a,STOP,count=1000000)
  except Exception as e:raise RuntimeError(self.c['name']+' at '+hex(self.u.reg_read(UC_X86_REG_EIP))) from e
  assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==STACK+4+4*len(args)
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  return self.u.reg_read(UC_X86_REG_EAX)
 def snapshot(self,full):
  if not full:return {'buckets':self.r(WORLD+0xf4),'count':self.r(WORLD+0xf8)}
  groups={}
  for i in range(self.r(WORLD+0xf4)):
   node=self.r(self.r(WORLD+0xf0)+i*4)
   while node:
    name=''.join(chr(x) for x in self.text(self.r(node)));groups.setdefault(name.upper(),[]).append({'name':name,'id':self.r(node+4)});node=self.r(node+8)
  return {'buckets':self.r(WORLD+0xf4),'count':self.r(WORLD+0xf8),'entries':groups}
 def run(self):
  rows=[]
  for op in self.c['operations']:
   self.w(REF,self.string(op['name']));kind=op['kind'];result=None;flags=op.get('flags',0x81000000);self.w(THING+8,flags)
   if kind in ['alias','publish']:
    self.w(THING+0x14,op['id']);self.w(THING+0x18,self.r(REF))
    self.call(0x4f94d0 if kind=='alias' else 0x4f9510,WORLD,[THING,0,REF] if kind=='alias' else [THING])
   elif kind=='remove':result=bool(self.call(0x5c9180,WORLD+0xf0,[REF]))
   elif kind=='lookup':
    node=self.call(0x4154d0,WORLD+0xf0,[REF]);result=self.r(node+4) if node else None
   elif kind in ['actor','kingdom']:
    actor=self.call(0x4f8e90 if kind=='actor' else 0x4f8f00,WORLD,[REF]);result=self.r(actor+0x14) if actor else None
   else:raise ValueError(kind)
   rows.append({'result':result,'flags':self.r(THING+8),'table':self.snapshot(kind!='alias')})
  return rows
def cases():
 actors=[{'id':0x10001,'actor':True,'kingdom':False},{'id':0x10002,'actor':False,'kingdom':True},{'id':0x20003,'actor':True,'kingdom':False}]
 def op(kind,name,id=0x10001):return {'kind':kind,'name':name,'id':id}
 rows=[{'name':'duplicates_and_classes','actors':actors,'operations':[op('alias','Army'),op('alias','ARMY',0x10002),op('lookup','army'),op('actor','aRmY'),op('kingdom','army'),op('publish','ArMy',0x10003),op('actor','army'),op('remove','ARMY'),op('actor','army'),op('remove','army'),op('remove','missing'),op('publish',''),op('alias',''),op('lookup','')]},
 {'name':'hash_collisions','actors':actors,'operations':[op('alias','a'),op('alias','A',0x10002),op('alias','ab'),op('alias','ba'),op('publish','a',0x10001),op('remove','ba'),op('lookup','ab'),op('actor','missing')]}]
 for count in [117,118,119,237,238,239,477,478,479]:
  ops=[op('alias','same' if i%7==0 else f'n{i}',0x10001+(i%3)) for i in range(count)]
  ops +=[op('lookup','SAME'),op('publish','same',0x10002),op('lookup','same'),op('remove','SAME'),op('lookup','same')]
  rows.append({'name':f'rehash_{count}','actors':actors,'operations':ops})
 return rows
def main():
 rows=[];visited={}
 for c in cases():
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=IMAGE.at(a,n);assert raw==ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'visited_windows':windows,'seams':['String copy/ref ownership (stable nonfreed UTF16 fixture buffers)','Heap allocator/free','CRT thread-locale accessor supplies C locale; native_wcsicmp comparison executes','4F4C40 later spatial publication; constructor actor class branches false during name publication'],'scope':'Actual registration4F94D0/4F9510,4154D0 lookup,5C9180 removal,593570 rehash,405030 hash, live-generation/class-filtered named lookups. Original world table constructor fields59/count0/grow1 supplied and statically pinned separately.'}
 p=HERE/'native-names.json';p.write_text(json.dumps(doc,indent=2)+'\n');print({'sequences':len(rows),'transitions':sum(len(r['output']) for r in rows),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
if __name__=='__main__':main()
