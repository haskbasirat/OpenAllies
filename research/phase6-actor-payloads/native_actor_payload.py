"""Original GActor serialization tail, real component dispatch/Properties/Updatable.

Enter 4D919C with the original frame's version and input context already restored.
Stop before post-load fixups at 4D9714. No field-read branch is intercepted.
Allocation, localization and definition lookups are explicit deterministic seams.
"""
from pathlib import Path
import hashlib,json,random,struct,sys
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase6-mapworld'))
from native_load_oracle import Machine,P,V,H,W
from unicorn.x86_const import *

class TailMachine(Machine):
 def __init__(self,payload):
  super().__init__(payload)
  self.heap=0x2090000;self.component=0x2085000;self.cvtable=0x2086000
  self.alloc_context=0x2087000;self.alloc_vtable=0x2087100;self.last_string=0
  self.nodes=[];self.executed={};self.string_reads=[];self.lookup_strings=[]
  self.w(self.thing+4,self.definition);self.w(self.component,self.cvtable)
  self.w(self.cvtable+0x2c,0x581ae0);self.w(self.cvtable+0x20,0x301f300)
  self.w(self.alloc_context,self.alloc_vtable);self.w(self.alloc_vtable+0xc,0x301f400);self.w(self.alloc_vtable+4,0x301f600)
  self.w(self.table+0x48,0x301f500);self.w(0xb36534,self.world)
  self.w(self.world+0x284,0x2088000);self.w(0x2088000,7);self.w(0x2088004,0x2088100)
 def alloc(self,n):
  p=self.heap;self.heap+=(n+15)&~15;assert self.heap<0x2100000;return p
 def string_alloc(self,text=b''):
  p=self.alloc(0x210);self.w(p,self.alloc_context);self.w(p+4,len(text)//2);self.w(p+8,255);self.w(p+12,1)
  self.u.mem_write(p+16,text+b'\0\0');return p+16
 def hook(self,u,pc,size,data):
  if P.base<=pc<P.base+P.image_size:self.executed[pc]=bytes(u.mem_read(pc,size))
  if pc==0x4cf480:
   slot=self.arg();assert slot==6,slot
   self.w(self.thing+0x60+slot*4,self.component);self.ret(pop=4)
  elif pc==0x50b250:self.ret()
  elif pc==0x301f300:self.ret(0xffffffff)
  elif pc==0x6f7810:
   p=self.alloc(self.arg());self.nodes.append(p);self.ret(p)
  elif pc==0x856370:self.ret(self.alloc_context)
  elif pc==0x301f400:self.ret(self.string_alloc()-16)
  elif pc==0x4e2450:
   p=self.r(self.arg());n=self.r(p-12)
   self.lookup_strings.append(list(struct.unpack('<'+'H'*n,u.mem_read(p,n*2))))
   self.ret(0x13572468,pop=4)
  elif pc==0x41d290:
   stream,dst=self.arg(),self.arg(1);start=self.at;n=struct.unpack('<H',self.take(2))[0];b=self.take(n*2)
   self.w(dst,self.string_alloc(b));self.string_reads.append({'offset':start,'length':2+n*2,'utf16':list(struct.unpack('<'+'H'*n,b))});self.ret(stream)
  elif pc==0x301f600:self.ret(pop=4)
  elif pc==0x301f500:self.ret(1)
  elif pc==0x59e520:self.ret(0)
  elif pc==0x4b9f40:self.ret(self.string_alloc(),pop=8)
  elif pc==0x405b80:self.w(u.reg_read(UC_X86_REG_ECX),self.arg());self.ret(pop=4)
  elif pc==0x404f60:self.ret()
  elif pc in (0x401330,0x858ad0,0x858a00):raise RuntimeError(hex(pc))
  else:super().hook(u,pc,size,data)
 def run_tail(self,version,definitionmask):
  self.w(self.definition+0x174,definitionmask)
  self.w(self.stack+0x20,version);self.w(self.stack+0xa8,self.world)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ESI,self.thing);self.u.reg_write(UC_X86_REG_EBX,0)
  self.u.emu_start(0x4d919c,0x4d9714,count=100000)
  assert not self.error and self.u.reg_read(UC_X86_REG_EIP)==0x4d9714,(version,self.at,self.error,hex(self.u.reg_read(UC_X86_REG_EIP)),self.payload.hex())
  return self.collect()
 def run_actor(self,version,definitionmask):
  self.w(self.definition+0x174,definitionmask)
  self.w(0xb36530,0x208a000);self.w(0x208a000+0x2e4,0x208b000)
  for index in range(42):self.w(0x208b000+index*4,0x208c000)
  self.w(self.stack,self.stop);self.w(self.stack+4,self.world)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.thing)
  self.u.emu_start(0x4d8cf0,0x4d9714,count=100000)
  assert not self.error and self.u.reg_read(UC_X86_REG_EIP)==0x4d9714,(version,self.at,self.error,hex(self.u.reg_read(UC_X86_REG_EIP)),self.payload.hex())
  result=self.collect();result['state']=[self.r(self.thing+0x100),self.r(self.thing+0x104)];return result
 def collect(self):
  lists=[]
  for offset,width in [(0x118,1),(0x120,1),(0x128,1),(0x130,3)]:
   p=self.r(self.thing+offset);items=[]
   while p:items.append([self.r(p+i*4) for i in range(width)]);p=self.r(p+width*4);assert len(items)<100
   lists.append(items)
  return {'consumed':self.at,'fields':{f'word_{o:x}':self.r(self.thing+o) for o in [0x158,0x15c,0x160,0x164,0x168,0x16c,0x170,0x174]},'lists':lists,'owner_lookup_strings':self.lookup_strings,'updatable_registered':self.r(0x2088100)!=0,'current_component_pointer':self.r(self.thing+0xe4),'reads':self.reads,'string_reads':self.string_reads}

def string(units):return struct.pack('<H',len(units))+struct.pack('<'+'H'*len(units),*units)
def main():
 rng=random.Random(0x4d919c);cases=[];executed={}
 for version in range(1,15):
  for sample in range(8):
   mask=0x40 if sample&1 else 0;default=0x40 if sample&2 else 0;classes=[0xffffffff]*(sample%3);upid=0 if sample&2 else 0xffffffff
   upversion=(sample//2)%2;propsversion=(sample//4)%2
   payload=W(default,mask)+(W(upversion,*([upid] if upversion else []),propsversion,len(classes),*classes) if mask else b'')
   current=6 if mask and sample&4 else 0xffffffff;payload+=W(current)
   expected={'saved_components_mask':mask,'default_components_mask':default,'current_component':current if current<0x80000000 else -1,'properties_count':len(classes) if mask else None,'updatable_version':upversion,'properties_version':propsversion,'updatable_id':(upid if upid<0x80000000 else -1) if upversion else None}
   for name,condition in [('word_158',version>=2),('legacy_word_15c',3<=version<12)]:
    value=rng.getrandbits(32) if condition else None;expected[name]=value
    if condition:payload+=W(value)
   ref=[0x41,0xd800,sample] if sample&2 else [];presence=255 if sample&2 else 0
   expected['word_160_source']={'presence':presence,'utf16':ref if presence else None} if version>=4 else None
   if version>=4:payload+=bytes([presence])+(string(ref) if presence else b'')
   presence=2 if sample&4 else 0;value=rng.getrandbits(32) if presence else None
   expected['word_164']={'presence':presence,'value':value} if version>=5 else None
   if version>=5:payload+=bytes([presence])+(W(value) if presence else b'')
   key=[0x4e,0x61,0x6d,0x65,sample] if sample&1 else []
   expected['localized_name_key_utf16']=key if version>=6 else None
   if version>=6:payload+=string(key)
   for name,condition in [('word_168',version>=8),('word_16c',version>=8),('word_170',version>=10)]:
    value=rng.getrandbits(32) if condition else None;expected[name]=value
    if condition:payload+=W(value)
   expected['lists']=None
   if version>=11:
    expected['lists']={}
    for index,(name,width) in enumerate([('list_118',1),('list_120',1),('list_128',1),('list_130',3)]):
     count=[0,1,3,0xffffffff][(sample+index)%4];items=[[rng.getrandbits(32) for _ in range(width)] for _ in range(count if count<4 else 0)]
     payload+=W(count,*[v for row in items for v in row]);expected['lists'][name]={'serialized_count':count,'values':items}
   value=rng.getrandbits(32) if version>=13 else None;expected['word_174']=value
   if value is not None:payload+=W(value)
   m=TailMachine(payload+b'FOLLOWING_RECORD');native=m.run_tail(version,mask|default);assert native['consumed']==len(payload)
   for name in ['word_158','word_168','word_16c','word_170','word_174']:
    if expected[name] is not None:assert native['fields'][name]==expected[name]
   if expected['legacy_word_15c'] is not None:assert native['fields']['word_15c']==0xffffffff
   if version>=11:assert native['lists']==[v['values'] for v in expected['lists'].values()]
   if version>=4:assert native['owner_lookup_strings']==([ref] if expected['word_160_source']['presence'] else [])
   executed.update(m.executed)
   state=[rng.getrandbits(32),rng.getrandbits(32) if version>=9 else 0];prefix=W(version,0,*state[:(2 if version>=9 else 1)])
   full=TailMachine(prefix+payload+b'FOLLOWING_RECORD');native_actor=full.run_actor(version,mask|default);assert native_actor['consumed']==len(prefix+payload);assert native_actor['state']==state
   executed.update(full.executed);cases.append({'version':version,'sample':sample,'tail_hex':payload.hex(),'actor_hex':(prefix+payload).hex(),'expected_state':state,'expected':expected,'native':native,'native_actor':native_actor})
 crete_path=ROOT.parent/'reconstruction/assets/map-chunks/Campaign/axis01_crete/KINGDOMS.bin';crete=crete_path.read_bytes()
 m=TailMachine(crete[127:]);native=m.run_tail(14,0x40);assert native['consumed']==68
 executed.update(m.executed)
 full=TailMachine(crete[111:]);native_actor=full.run_actor(14,0x40);assert native_actor['consumed']==84
 executed.update(full.executed)
 dunkirk_path=ROOT.parent/'reconstruction/assets/map-chunks/DeveloperMaps/axis00_dunkirk/KINGDOMS.bin';dunkirk=dunkirk_path.read_bytes()
 d=TailMachine(dunkirk[127:]);d_native=d.run_tail(14,0x40);assert d_native['consumed']==52;executed.update(d.executed)
 d=TailMachine(dunkirk[111:]);d_actor=d.run_actor(14,0x40);assert d_actor['consumed']==68;executed.update(d.executed)
 ranges={}
 for name,a,n in [('actor_load',0x4d8cf0,0xcd4),('component_dispatch',0x4d8290,0x1bf),('properties_load',0x581ae0,0x22f),('updatable_load',0x50bcd0,0x96),('version',0x8589b0,0x50),('localized',0x49ad70,0x83)]:
  b=P.range(a,n);assert b==V.range(a,n);ranges[name]={'start':hex(a),'length':n,'sha256':H(b)}
 instruction_rows=[]
 for a,b in sorted(executed.items()):
  assert b==V.range(a,len(b));instruction_rows.append({'address':hex(a),'bytes':b.hex(),'sha256':H(b)})
 out={'source_sha256':H(P.b),'v14_sha256':H(V.b),'scope':'Native 4D919C..4D9714 versioned serialization tail and 4D8CF0..4D9714 entire actor serialization layer with GObjectLoad seam, original component dispatch, Properties/Updatable and localized read decisions. Synthetic pre-restored frame and definition masks; 42 state descriptors are nontransient fixture descriptors. Allocation/component factory, string storage, localization catalog/owner lookup and Updatable scheduling cleanup are fixture callbacks; no complete GActor runtime restore claimed. Mode zero only. Sentinel-only Properties classes. Earlier ActorPrefix has separate phase6-mapworld native fixtures.','ranges':ranges,'cases':cases,'source_crete':{'path':str(crete_path.relative_to(ROOT.parent)),'sha256':H(crete),'actor_offset':111,'actor_hex':crete[111:195].hex(),'tail_offset':127,'native':native,'native_actor':native_actor,'actor_consumed':84,'following_word':struct.unpack_from('<I',crete,195)[0]}}
 out['source_dunkirk']={'path':str(dunkirk_path.relative_to(ROOT.parent)),'sha256':H(dunkirk),'actor_offset':111,'actor_hex':dunkirk[111:179].hex(),'tail_offset':127,'native':d_native,'native_actor':d_actor,'actor_consumed':68,'following_word':struct.unpack_from('<I',dunkirk,179)[0]}
 dest=Path(__file__).with_name('native-actor-payload-fixtures.json');dest.write_text(json.dumps(out,indent=2)+'\n')
 Path(__file__).with_name('executed-instructions.json').write_text(json.dumps(instruction_rows,indent=2)+'\n')
 print(json.dumps({'cases':len(cases),'native_executions':2*(len(cases)+2),'crete_consumed':84,'executed_instructions':len(instruction_rows),'fixture_sha256':H(dest.read_bytes())}))
if __name__=='__main__':main()
