"""Execute original x86 GThing::Load and GTIDManager::Load against bounded streams.

Only stream virtual read/write-mode and CString storage are seams. Native version
helper, field copies, flags merge and GTID dimension checks execute unmodified.
No live game process or source assets are changed.
"""
from pathlib import Path
import hashlib,json,random,struct,sys
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/engine-baseline'))
sys.path.insert(0,str(ROOT.parent/'reconstruction/tools/python-packages'))
from audit_baseline import PE
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
P=PE(ROOT.parent/'AA.exe');V=PE(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe')
H=lambda b:hashlib.sha256(b).hexdigest()
assert H(P.b)=='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
assert H(V.b)=='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
W=lambda *v:struct.pack('<'+'I'*len(v),*v)

class Machine:
 def __init__(self,payload):
  self.u=Uc(UC_ARCH_X86,UC_MODE_32);self.u.mem_map(P.base,(P.image_size+4095)&~4095)
  for vs,r,size,off in P.sections:self.u.mem_write(P.base+r,P.b[off:off+size])
  self.u.mem_map(0,4096);self.u.mem_map(0x2000000,0x100000);self.u.mem_map(0x3000000,0x20000)
  self.stack=0x3010000;self.stop=0x301f000;self.manager=0x2000000;self.thing=0x2080000;self.definition=0x2081000;self.world=0x2082000;self.table=0x2083000;self.string=0x2084000
  self.w(self.world+0x3c,self.table);self.w(self.table,0x301f100);self.w(self.table+0xc,0x301f200)
  self.payload=payload;self.at=0;self.reads=[];self.error=None;self.u.hook_add(UC_HOOK_CODE,self.hook)
 def w(self,p,v):self.u.mem_write(p,W(v))
 def r(self,p):return struct.unpack('<I',self.u.mem_read(p,4))[0]
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+4*i)
 def ret(self,value=0,pop=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop)
 def take(self,n):
  assert self.at+n<=len(self.payload),(self.at,n,len(self.payload))
  p=self.payload[self.at:self.at+n];self.at+=n;return p
 def hook(self,u,pc,size,_):
  if pc==0x301f100:
   dst,n=self.arg(),self.arg(1);start=self.at;b=self.take(n)
   if b:u.mem_write(dst,b)
   self.reads.append({'offset':start,'length':n,'destination':dst-self.thing if self.thing<=dst<self.thing+0x1000 else None});self.ret(pop=8)
  elif pc==0x301f200:self.ret(0)
  elif pc==0x41d290:
   stream,dst=self.arg(),self.arg(1);count=struct.unpack('<H',self.take(2))[0];text=self.take(count*2);self.w(self.string-12,count);self.w(dst,self.string)
   u.mem_write(self.string,text+b'\0\0');self.ret(stream)
  elif pc==0x4ec7a0:self.ret(pop=4)
  elif pc in (0x41dfb0,0x4f0110):self.error='native_version_or_capacity_rejection';u.emu_stop()
 def call(self,pc,this):
  self.w(self.stack,self.stop);self.w(self.stack+4,self.world);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,this);self.u.emu_start(pc,self.stop,count=100000)
  if not self.error:assert self.u.reg_read(UC_X86_REG_EIP)==self.stop and self.u.reg_read(UC_X86_REG_ESP)==self.stack+8

def main():
 rng=random.Random(0x4ef160);things=[];ids=[];worlds=[];actors=[]
 for version in [0,1,2]:
  for sample in range(16):
   existing=rng.getrandbits(32);definition=rng.getrandbits(32);flags=rng.getrandbits(32);field=rng.getrandbits(32);position=[rng.getrandbits(32) for _ in range(3)];facing=[rng.getrandbits(32) for _ in range(3)];name=[0x41,0x20,sample,0xd800] if sample%2 else [];legacy=rng.getrandbits(32) if version==1 else None
   payload=W(version)+struct.pack('<H',len(name))+struct.pack('<'+'H'*len(name),*name)+W(field,*position,*facing,flags)+(W(legacy) if legacy is not None else b'')
   m=Machine(payload);m.w(m.thing+4,m.definition);m.w(m.thing+8,existing);m.w(m.definition+0x1c,definition);m.call(0x4ef160,m.thing)
   assert not m.error and m.at==len(payload)
   things.append({'input_hex':payload.hex(),'existing_flags':existing,'definition_flags':definition,'fields':{'version':version,'name_utf16':name,'word_1c':field,'position_bits':position,'facing_bits':facing,'serialized_flags':flags,'legacy_version_one_word':legacy},'native_flags':m.r(m.thing+8),'native_word_1c':m.r(m.thing+0x1c),'native_position_bits':list(struct.unpack('<3I',m.u.mem_read(m.thing+0x20,12))),'native_facing_bits':list(struct.unpack('<3I',m.u.mem_read(m.thing+0x2c,12))),'consumed':m.at})
 for highest in [0,1,2,15,256,65535]:
  generations=[rng.randrange(65536) for _ in range(highest)];cursor=1 if highest==65535 else highest+1;payload=W(0,cursor,highest,65536)+struct.pack('<'+'H'*len(generations),*generations);m=Machine(payload);m.call(0x4f0230,m.manager)
  assert not m.error and m.at==len(payload)
  got=list(struct.unpack('<65536H',m.u.mem_read(m.manager+4,131072)));assert got==generations+[0]*(65536-highest)
  ids.append({'input_hex':payload.hex(),'next_slot':m.r(m.manager+0x60008),'highest_slot':m.r(m.manager+0x6000c),'generations_sha256':H(m.u.mem_read(m.manager+4,131072)),'consumed':m.at})
 for payload in [W(1,1,0,65536),W(0,1,0,65535)]:
  m=Machine(payload);m.call(0x4f0230,m.manager);assert m.error;ids.append({'input_hex':payload.hex(),'error':m.error})
 for version in range(29):
  offsets=[0xd4,0xdc,0xe0,0xe4,0xe8,0x140]+([0x184,0x194] if version>=4 else [])+([0x198] if version>=27 else [])+([0x44] if version>=5 else [])+([0x48] if version>=7 else [])
  values=[rng.getrandbits(32) if o!=0x44 else 1 for o in offsets]
  payload=W(version,*values);m=Machine(payload);m.at=4;m.w(m.stack+0x14,version);m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_EDI,m.thing);m.u.reg_write(UC_X86_REG_EBX,m.world);m.u.reg_write(UC_X86_REG_EBP,0)
  m.u.emu_start(0x4fc6c6,0x4fc7df,count=100000)
  assert m.at==len(payload) and m.u.reg_read(UC_X86_REG_EIP)==0x4fc7df
  worlds.append({'input_hex':payload.hex(),'version':version,'native_fields':{hex(o):m.r(m.thing+o) for o in offsets},'consumed':m.at})
 for version in [0,8,9,14]:
  for flag,owner in [(0,0x90002),(1<<23,0x90002),(1<<23,0)]:
   low=rng.getrandbits(32);high=rng.getrandbits(32);priorlow=0x12345678;priorhigh=0xabcd1234
   payload=W(version,owner,low,*([high] if version>=9 else []));m=Machine(payload)
   m.w(m.thing+8,flag);m.w(m.thing+0x100,priorlow);m.w(m.thing+0x104,priorhigh);m.w(0xb364c0,m.manager);m.w(0xb36534,m.world);m.w(m.world+0x2c,14)
   m.u.mem_write(m.manager+4+2*2,struct.pack('<H',9));m.w(m.manager+0x20004+2*4,0x2089000)
   m.w(m.stack,m.stop);m.w(m.stack+4,m.world);m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_ECX,m.thing)
   m.u.emu_start(0x4d8cf0,0x4d8e1f,count=100000)
   assert not m.error and m.at==len(payload)
   actors.append({'input_hex':payload.hex(),'version':version,'owner_handle':owner,'thing_flags':flag,'state_low':low,'state_high':high if version>=9 else None,'prior_state':[priorlow,priorhigh],'native_owner_present':bool(m.r(m.thing+0xe8)),'native_state':[m.r(m.thing+0x100),m.r(m.thing+0x104)],'native_world_version':m.r(m.world+0x2c),'consumed':m.at})
 ranges={}
 for name,a,n in [('thing_load',0x4ef160,0x106),('id_manager_load',0x4f0230,0x7d),('version_helper',0x8589b0,0x50),('world_prefix',0x4fc600,0x1df),('actor_prefix',0x4d8cf0,0x140)]:
  b=P.range(a,n);assert b==V.range(a,n);ranges[name]={'start':hex(a),'bytes':n,'sha256':H(b)}
 result={'source_sha256':H(P.b),'v14_sha256':H(V.b),'ranges':ranges,'thing_cases':things,'id_manager_cases':ids,'world_cases':worlds,'actor_cases':actors,'scope':'Original x86 full4EF160/4F0230, including original8589B0; GWORLD field-read segment4FC6C6..4FC7DF with native version local restored; actor prefix4D8CF0..4D8E1F with GObjectLoad seam already decoded. CString allocation/storage and virtualstream transport mocked; source-field decisions are not.'}
 path=Path(__file__).with_name('native-load-fixtures.json');path.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'things':len(things),'id_manager':len(ids),'worlds':len(worlds),'sha256':H(path.read_bytes()),'report':str(path)}))
if __name__=='__main__':main()
