"""Original reader control flow against all original MAP_HEADER payloads.

Object allocation, string storage, registry lookup and I/O are explicit seams.
The actual WorldRules/WorldParameters loops and header/creator field order run.
Unresolved definition lookup returns null; no applied runtime-rule claim.
"""
from pathlib import Path
import sys,struct,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/engine-baseline'))
from audit_baseline import PE
from inspect_header import Reader,decode,read_chunks
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
P=PE(ROOT.parent/'AA.exe');V=PE(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe')
assert hashlib.sha256(P.b).hexdigest()=='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
assert hashlib.sha256(V.b).hexdigest()=='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
VISITED={};SEAMS={}
class Machine:
 def __init__(self,bytes):
  self.r=Reader(bytes);self.u=Uc(UC_ARCH_X86,UC_MODE_32);self.u.mem_map(0,4096)
  self.u.mem_map(P.base,(P.image_size+4095)&~4095)
  for vs,r,size,off in P.sections:self.u.mem_write(P.base+r,P.b[off:off+size])
  self.u.mem_map(0x2000000,0x1000000);self.next=0x2001000;self.stack=0x2f08000;self.stop=0x2f10000
  self.object=self.alloc(0x200);self.stream=self.alloc(0x100);table=self.alloc(0x100)
  self.write(self.stream,table);self.write(table,0x2e00000);self.write(table+0x48,0x2e00010)
  self.string_table=self.alloc(0x40);self.write(self.string_table+4,0x2e00020);self.write(self.string_table+12,0x2e00030)
  self.string_allocator=self.alloc(4);self.write(self.string_allocator,self.string_table)
  self.empty=self.string(b'');self.write(0xb43b40,self.empty);self.write(0xb388a0,self.empty)
  registry=self.alloc(0x100);self.write(0xb36530,registry)
  self.rules=self.alloc(0x40);self.write(self.rules,0xa31990)
  self.params=self.alloc(0x60);self.write(self.params,0xa317a8)
  self.write(self.object+4,self.rules);self.write(self.object+8,self.params)
  for off in (0x20,0x24,0x28,0x2c,0x38,0x3c,0x58,0x5c,0x60):self.write(self.object+off,self.empty)
  self.reads=[];self.seams=[]
  self.u.reg_write(UC_X86_REG_FPCW,0x27f);self.u.hook_add(UC_HOOK_CODE,self.hook)
 def alloc(self,n):
  p=self.next;self.next+=(n+15)&~15;assert self.next<0x2d00000;self.u.mem_write(p,b'\0'*n);return p
 def write(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def read(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def string(self,raw):
  count=len(raw)//2;h=self.alloc(16+len(raw)+2);self.write(h,self.string_allocator);self.write(h+4,count);self.write(h+8,count);self.write(h+12,1);self.u.mem_write(h+16,raw);return h+16
 def arg(self,n=0):return self.read(self.u.reg_read(UC_X86_REG_ESP)+4+4*n)
 def ret(self,n=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.read(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+n);self.u.reg_write(UC_X86_REG_EAX,value)
 def read_string(self,dest):
  count=struct.unpack('<H',self.r.take(2))[0];raw=self.r.take(count*2);self.write(dest,self.string(raw))
 def vector(self,ecx,count,stride,strings):
  ptr=self.alloc(count*stride);self.write(ecx,ptr);self.write(ecx+8,count)
  for i in range(count):
   for off in strings:self.write(ptr+i*stride+off,self.string(b''))
 def hook(self,u,pc,size,_):
  if P.base<=pc<P.base+P.image_size:
   raw=P.range(pc,size);assert raw==V.range(pc,size),hex(pc);VISITED[pc]=raw.hex()
  ecx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x2e00000:
   dest,n=self.arg(),self.arg(1);start=self.r.at;data=self.r.take(n);u.mem_write(dest,data)
   self.reads.append({'offset':start,'length':n});self.ret(8);return
  if pc==0x2e00010:self.ret(value=1);return
  if pc==0x2e00020:self.ret(4);return
  if pc==0x2e00030:self.ret(value=self.string(b'')-16);return
  if pc==0x8589b0:
   v=self.r.u32();assert v<=self.arg(1),(v,self.arg(1));self.write(ecx,v);self.ret(8,value=ecx);return
  if pc==0x856370:self.ret(value=self.string_allocator);return
  if pc==0x4012c0:self.ret(value=self.empty-16);return
  if pc==0x401330:
   n=self.arg();old=self.read(ecx);oldn=self.read(old-12);new=self.string(bytes(u.mem_read(old,min(n,oldn)*2))+b'\0'*max(0,n-oldn)*2)
   self.write(new-12,min(n,oldn));self.write(ecx,new);self.ret(4,value=ecx);return
  if pc==0x41d290:
   self.read_string(self.arg(1));self.ret(value=self.arg(0));return
  if pc==0x405b80:self.write(ecx,self.empty);self.ret(4,value=ecx);return
  if pc==0x59e520:self.ret(value=0);return
  if pc==0x4b9f40:self.ret(8,value=0xb43b40);return
  if pc==0x511d90:self.ret();return
  if pc in (0x470470,0x48d500):self.ret(4,value=0);return
  if pc==0x404f90:self.ret();return
  if pc==0x4f0110:raise RuntimeError('native reported error at '+str(self.r.at))
  if pc==0x6f7810:self.ret(value=self.alloc(self.arg()));return
  if pc==0x6f7430:self.ret();return
  if pc==0x50fb40:
   n=self.arg();self.vector(ecx,n,12,[0,4,8]);self.write(ecx+4,n);self.ret(4);return
  if pc==0x4a4ce0:self.vector(ecx,self.arg(),0x6c,[0,4,8,12,20,44,88]);self.ret(4);return
  if pc==0x4a2050:self.vector(ecx,self.arg(),8,[0,4]);self.ret(4);return
  if pc==0x4849f0:self.vector(ecx,self.arg(),4,[]);self.ret(4);return
  if pc==0x45f180:self.write(ecx,0);self.write(ecx+4,0);self.ret(4);return
  if pc in (0x512360,0x4e7e00,0x4e7ec0,0x4e7f80,0x50f7d0,0x4daaa0,0x4c2e60,0x4cdbb0,0x513700,0x4f9280):
   start=self.r.at;self.r.string();self.write(self.arg(1),0);self.seams.append({'va':hex(pc),'offset':start,'end':self.r.at,'operation':'reference UTF16 decode and unresolved registry lookup'});self.ret(value=self.arg(0));return
  if pc==0x5a1b2e:
   #Environment strings and UI localization consume no stream bytes. Skip that
   #host-specific block; the subsequent version-gated field loop is original.
   self.seams.append({'va':hex(pc),'operation':'skip nonserialized environment/localization side effects through5A1C68'});u.reg_write(UC_X86_REG_EIP,0x5a1c68);return
  if pc==0x5a1360:self.ret(4);return #entry publication: stream reads already executed
 def run(self):
  self.write(self.stack,self.stop)
  for i,v in enumerate((self.stream,0,0)):self.write(self.stack+4+4*i,v)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.object)
  self.u.emu_start(0x5a19c0,0x5a1e3d,count=3000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==0x5a1e3d,hex(self.u.reg_read(UC_X86_REG_EIP))
  prefix=self.r.at;header_extent=[self.read(self.object+off) for off in (0x30,0x34)]
  cls=self.r.string();assert cls=='KKC_WorldCreator'
  creator=self.alloc(0x60);self.write(creator+0x20,0xa31990);self.write(creator+4,0xa317a8)
  self.write(self.stack,self.stop);self.write(self.stack+4,self.stream)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,creator)
  self.u.emu_start(0x513910,self.stop,count=3000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
  return {'prefix_bytes':prefix,'decoded_bytes':self.r.at,'display_extents_bits':header_extent,
          'creator_size_bits':[self.read(creator+off) for off in (0x3c,0x40)],'rng_seed':self.read(creator+0x4c),
          'reads':self.reads,'seams':self.seams}

if __name__=='__main__':
 rows=[]
 for p in sorted((ROOT.parent/'reconstruction/assets/original/Data/Maps').rglob('*.rmp')):
  data=read_chunks(p)['MAP_HEADER'][0];expected=decode(data)
  try:got=Machine(data).run()
  except Exception as e:print(str(p),repr(e));raise
  assert got['prefix_bytes']==expected['prefix_bytes'],(p,got['prefix_bytes'],expected['prefix_bytes'])
  assert got['decoded_bytes']==len(data),(p,got['decoded_bytes'],len(data))
  assert got['creator_size_bits']==expected['creator_size_bits']
  assert got['rng_seed']==expected['seed']
  if expected['version']>=7:assert got['display_extents_bits']==expected['size_bits']
  rows.append({'path':str(p),'payload_sha256':hashlib.sha256(data).hexdigest(),'header_version':expected['version'],**got})
 out={'schema':1,'passed':True,'maps':rows,'visited_native_instructions_and_seam_entries':[{'va':hex(a),'bytes':b} for a,b in sorted(VISITED.items())],
      'scope':'Original serialized reader control flow with allocation/string/reference/I-O/environment seams; no runtime lookup/application or localized UI claim.'}
 (HERE/'native-header.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'passed':True,'maps':len(rows),'visited':len(VISITED)}))
