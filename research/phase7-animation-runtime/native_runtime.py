"""Execute original Animator arithmetic, Character dispatch and animation selection.
FX/scheduler/actor virtuals are explicit mutable fixture leaves; arithmetic and
Character/selection branches execute original instructions unchanged.
"""
from pathlib import Path
import copy,json,struct,sys,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified,sha
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
F=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
C,O,W,A,B,D,T,R,V,M,P,VT=range(0x4000000,0x400c000,0x1000)
ACTIVATE=0x400d000
handles={1:A,2:B,3:D};ids={v:k for k,v in handles.items()}
def signed(n):return n-(1<<32) if n>=1<<31 else n

class Machine(Base):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=copy.deepcopy(c)
  self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x20000);self.events=[];self.executed={}
  self.w(0xb36534,W);self.w(W+0xe0,c['now']);self.w(0xb3ae88,T)
  for kind in range(7):self.w(T+kind*8+4,int(c['looping'][kind]))
  self.w(C,VT);self.w(VT+0x38,ACTIVATE);self.w(C+4,O);self.w(O,VT+0x100)
  self.w(VT+0x100+0xf4,ACTIVATE+0x10);self.w(O+4,O+0x800)
  self.w(ACTIVATE+0x100,c['scale']);self.u.mem_write(ACTIVATE+0x10,b'\xd9\x05'+struct.pack('<I',ACTIVATE+0x100)+b'\xc3')
  self.w(O+0x6c,M if c['speed'] is not None else 0);self.w(M+0x20,c['speed'] or 0)
  self.w(O+0x74,P if c['performer'] else 0);self.w(O+0xe4,C if c['active_component'] else P if c['performer_active'] else 0)
  self.w(P+4,O);self.w(P+0x34,1 if c['ability'] else 0)
  self.w(R,c['rng']);self.w(O+0x800+0x12c,c['repeat_min']);self.w(O+0x800+0x130,c['repeat_max'])
  self.w(V+0xc,V+0x400);self.w(V+0x10,1);self.w(V+0x400,handles[c['idle']])
  self.w(V,V+0x500)
  for kind in range(7):self.w(V+0x500+kind*4,handles[c['fallback']])
  for id,info in c['animations'].items():
   a=handles[int(id)];self.w(a+8,info['kind']);self.w(a+0xc,int(id));self.w(a+0x10,info['length']);self.w(a+0x14,info['use_fraction']);self.w(a+0x2c,info['walk_distance'])
   self.w(a+0x50,a+0x100 if info['cues'] else 0)
   for i,time in enumerate(info['cues']):self.put(a+0x100+i*16,time,0,0,a+0x100+(i+1)*16 if i+1<len(info['cues']) else 0)
  self.set_state(c['state']);self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,a,*words):self.u.mem_write(a,struct.pack('<'+'I'*len(words),*(n&0xffffffff for n in words)))
 def set_state(self,s):
  self.w(C+0xc,int(s['scheduled']));self.put(C+0x14,s['elapsed'],s['resumed_at'],s['rate'],handles[s['animation']],int(s['active']),s['word_28'],s['variant'])
 def state(self):
  return dict(scheduled=bool(self.r(C+0xc)),elapsed=self.r(C+0x14),resumed_at=self.r(C+0x18),rate=self.r(C+0x1c),animation=ids[self.r(C+0x20)],active=bool(self.u.mem_read(C+0x24,1)[0]),word_28=self.r(C+0x28),variant=signed(self.r(C+0x2c)))
 def arg(self,n=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+n*4)
 def ret(self,value=0,pop=0):
  s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.executed[pc]=raw.hex()
  if pc==0x42a590:
   ptr=u.reg_read(UC_X86_REG_ECX);kind=None;animation=None
   for id,a in handles.items():
    if ptr-a in [0x38,0x40,0x48]:animation=id;kind={0x38:'Start',0x40:'Use',0x48:'End'}[ptr-a]
    for i in range(len(self.c['animations'][str(id)]['cues'])):
     if ptr==a+0x100+i*16+4:animation=id;kind={'Cue':i}
   assert kind is not None,(hex(ptr),self.state())
   self.events.append(dict(name='effect',animation=animation,effect=kind))
   if kind=='End' and self.c['replace_on_end'] is not None:self.w(C+0x20,handles[self.c['replace_on_end']])
   self.ret(pop=8)
  elif pc==0x50b250:
   self.events.append(dict(name='cancel'));self.w(C+0xc,0)
   if self.c['replace_on_cancel'] is not None:self.w(C+0x20,handles[self.c['replace_on_cancel']]);self.w(C+0x28,9)
   self.ret()
  elif pc==0x50acc0:
   self.events.append(dict(name='schedule',delay=self.arg(),event=self.arg(1)));self.w(C+0xc,1);self.ret(pop=8)
  elif pc==ACTIVATE:self.events.append(dict(name='activate'));self.ret(int(self.c['activate']))
  #Virtual scale uses an explicit fld/ret thunk to retain its x87 return ABI.
  elif pc in [0x49b480,0x45e5e0]:self.ret(1,4 if pc==0x45e5e0 else 0)
  elif pc==0x45e690:self.ret(V,4)
  elif pc==0x45ebb0:
   self.events.append(dict(name='remap',variant=signed(self.r(C+0x2c)),requested=self.arg()))
   self.ret(handles.get(self.c['remap'],0),4)
  elif pc==0x404ed0:self.ret()
  elif pc==0x4cfd90:self.events.append(dict(name='variant',variant=signed(self.arg())));self.ret(1,8)
  elif pc==0x432e90:self.ret(int(self.arg() in self.c['flags']),4)
  elif pc==0x4d0750:self.ret(self.c['model_variant'])
  elif pc==0x4c81b0:self.ret(handles[self.c['idle']],4)
  elif pc==0x4d0720:self.ret(V)
  elif pc==0x4f02b0:self.ret(R)
  elif pc==0x49b4d0:self.events.append(dict(name='repetitions'))
 def run(self):
  self.reset(0);op=self.c['operation'];args=[];ecx=C;entry=None
  if op in ['pause','resume','direct','move']:
   ecx=C+0x10;entry={'pause':0x4daf20,'resume':0x4daf40,'direct':0x4db120,'move':0x4db170}[op]
   if op=='direct':args=[handles[self.c['selected']]]
   if op=='move':args=[handles[self.c['selected']],self.c['speed'],self.c['scale']]
  else:
   entry={'play':0x563770,'begin':0x563640,'stop':0x4db080,'use':0x563620,'cue':0x563aa0,'stance':0x563ba0}[op]
   if op=='play':args=[handles[self.c['selected']],self.c['duration']]
   if op=='cue':args=[self.c['cue']]
   if op=='stop':args=[17]
  self.put(self.STACK,self.STOP,*args);self.u.reg_write(UC_X86_REG_ECX,ecx)
  self.u.emu_start(entry,self.STOP,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP,(op,hex(self.u.reg_read(UC_X86_REG_EIP)))
  assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4+4*len(args),(op,hex(self.u.reg_read(UC_X86_REG_ESP)))
  return dict(state=self.state(),events=self.events,rng=self.r(R),restarted=bool(self.u.reg_read(UC_X86_REG_EAX)&255) if op in ['direct','move'] else None)

def seed_case():
 return dict(precision='Extended64',operation='play',now=F(4.25),state=dict(scheduled=False,elapsed=F(.12345),resumed_at=F(2.75),rate=F(1.1),animation=1,active=False,word_28=3,variant=-2),
 animations={str(i):dict(kind=k,length=F(l),use_fraction=F(.3),walk_distance=F(1.33333),cues=list(map(F,[.1,.25]))) for i,k,l in [(1,0,3),(2,4,1),(3,2,.75)]},
 selected=2,looping=[True,False,True,False,False,False,False],speed=F(2.2),scale=F(.33333),duration=0,cue=0,replace_on_end=None,replace_on_cancel=None,
 remap=3,fallback=1,active_component=False,activate=True,performer=False,performer_active=False,ability=False,flags=[],idle=1,model_variant=0,rng=0x12345678,repeat_min=1,repeat_max=4)

def selection_cases():
 rows=[];m=Base()
 for variants in [[],[[]],[[11]],[[11,22,33],[],[44,55]]]:
  for variant in range(max(1,len(variants))):
   for seed in [0,1,0xffffffff,0x12345678]:
    m.reset(seed);obj=m.STATE+0x100;array=m.STATE+0x200;lists=m.STATE+0x400
    m.w(obj+0x40,len(variants));m.w(obj+0x3c,array)
    for i,values in enumerate(variants):
     at=lists+i*0x100;m.w(array+i*4,at);m.w(at,at+0x10);m.w(at+4,len(values))
     for n,value in enumerate(values):m.w(at+0x10+n*4,value)
    m.w(m.STACK,m.STOP);m.w(m.STACK+4,variant);m.w(m.STACK+8,m.STATE);m.u.reg_write(UC_X86_REG_ECX,obj)
    m.u.emu_start(0x4be5c0,m.STOP,count=1000)
    chosen=m.u.reg_read(UC_X86_REG_EAX);after=m.r(m.STATE)
    contains=[]
    for item in [0,11,22,44,55,99]:
     m.reset(after);m.w(m.STACK+4,variant);m.w(m.STACK+8,item);m.u.reg_write(UC_X86_REG_ECX,obj)
     m.u.emu_start(0x4be610,m.STOP,count=1000);contains.append(dict(item=item,result=bool(m.u.reg_read(UC_X86_REG_EAX)&255)))
    rows.append(dict(variants=variants,variant=variant,seed=seed,chosen=chosen or None,after=after,contains=contains))
 modes=[]
 for flags in [0,0x800,0x1000,0x4000,0x4800,0x1ff800,0x4001,0xffffffff]:
  for mode in [0,1,2,3,4,20,21,31,32,255]:
   m.reset(0);m.w(m.STATE+0x54,flags);m.w(m.STACK+4,mode)
   m.u.emu_start(0x4be590,m.STOP,count=1000);modes.append(dict(flags=flags,mode=mode,result=bool(m.u.reg_read(UC_X86_REG_EAX)&255)))
 idle=[]
 for values in [[],[0],[11,0,22]]:
  for variant in range(max(1,len(values))):
   m.reset(0);m.w(m.STATE+0x48,m.STATE+0x100);m.w(m.STATE+0x4c,len(values));m.w(m.STACK+4,variant)
   for i,v in enumerate(values):m.w(m.STATE+0x100+i*4,v)
   m.u.emu_start(0x4be660,m.STOP,count=1000);idle.append(dict(values=values,variant=variant,result=m.u.reg_read(UC_X86_REG_EAX) or None))
 return dict(selection=rows,modes=modes,idle=idle)

def playback_cases():
 rows=[]
 for precision,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  m=Base(cw);rng=random.Random(0x44f8a1)
  for i in range(64):
   m.reset(0);a=m.STATE+0x100;obj=m.STATE+0x200;definition=m.STATE+0x400;array=m.STATE+0x500;sequence=m.STATE+0x600;resource=m.STATE+0x700;world=m.STATE+0x800
   start,pause,rate,now,length,kf=map(F,[rng.uniform(-3,3),rng.uniform(3,6),rng.uniform(.1,3),rng.uniform(6,10),rng.uniform(.01,5),rng.uniform(.01,5)])
   active=bool(i%2)
   m.w(a+4,start);m.w(a+8,pause);m.w(a+0xc,rate);m.w(a+0x14,active)
   m.w(obj+0x158,array);m.w(obj+0x170,0);m.w(array,sequence);m.w(sequence,resource);m.w(resource+0x4c,kf);m.w(definition+0x10,length)
   m.w(0xb36538,world);m.w(world+0x8c,now)
   m.u.reg_write(UC_X86_REG_EBP,a);m.u.reg_write(UC_X86_REG_ESI,obj);m.u.reg_write(UC_X86_REG_EDI,definition);m.u.reg_write(UC_X86_REG_EBX,0)
   m.u.emu_start(0x44f8a1,0x44f8e6,count=1000)
   rows.append(dict(precision=precision,start=start,pause=pause,rate=rate,now=now,length=length,kf=kf,active=active,result_start=m.r(m.STACK+0x10),result_rate=m.r(m.STACK+0x1c)))
 return rows

def actor_dispatch_cases():
 rows=[]
 for character in [False,True]:
  for old in [None,1,2]:
   for replacement in [None,1,2]:
    m=Base();m.reset(0);actor=m.STATE;components={1:m.STATE+0x100,2:m.STATE+0x200};vt=m.STATE+0x300;stop=m.STATE+0x400;resume=m.STATE+0x410;events=[]
    m.w(actor+0x68,m.STATE+0x500 if character else 0);m.w(actor+0xe4,components.get(old,0));m.w(vt+0x40,stop);m.w(vt+0x3c,resume)
    for a in components.values():m.w(a,vt)
    def hook(u,pc,size,_):
     if pc not in [stop,resume,0x563ba0]:return
     if pc==0x563ba0:events.append(dict(name='character'))
     else:
      handle=next(k for k,v in components.items()if v==u.reg_read(UC_X86_REG_ECX));active=next((k for k,v in components.items()if v==m.r(actor+0xe4)),None)
      events.append(dict(name='deactivate'if pc==stop else'resume',component=handle,active=active))
      if pc==stop:m.w(actor+0xe4,components.get(replacement,0))
     sp=u.reg_read(UC_X86_REG_ESP);u.reg_write(UC_X86_REG_EIP,m.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4+(0 if pc==0x563ba0 else 4))
    m.u.hook_add(UC_HOOK_CODE,hook);m.u.emu_start(0x4cfc80,m.STOP,count=1000)
    active=next((k for k,v in components.items()if v==m.r(actor+0xe4)),None)
    rows.append(dict(character=character,old=old,replacement=replacement,active=active,events=events))
 return rows

def main():
 cases=[];executed={}
 for precision in ['Single24','Double53','Extended64']:
  for op in ['pause','resume','direct','move','play','begin','stop','use','cue','stance']:
   for i in range(24):
    c=seed_case();c.update(precision=precision,operation=op);c['state'].update(active=bool(i%2),variant=[-2,-1,0][i%3] if op=='play' else -2,scheduled=i%4==0,word_28=i%6)
    c['selected']=[1,2,3][i%3];c['state']['animation']=[1,2,3][i//3%3];c['cue']=[-1,0,1,2][i%4]
    c['duration']=[0,F(.37),F(-.25)][i%3];c['speed']=[F(2.2),F(.00001),F(.0001),None][i%4] if op=='play' else F(2.2)
    c['replace_on_end']=2 if i%7==0 else None;c['replace_on_cancel']=3 if i%7==1 else None
    c.update(active_component=i%4==0,activate=i%8!=0,performer=bool(i%2),performer_active=i%3==0,ability=i%3!=1,flags=[4] if i%4==1 else [1] if i%4==2 else [],remap=None if i%4==0 else 3)
    m=Machine(c);out=m.run();executed.update(m.executed);cases.append(dict(input=c,output=out))
 ranges=[]
 for start in sorted(executed):
  if ranges and int(ranges[-1]['end'],16)==start:ranges[-1]['hex']+=executed[start];ranges[-1]['end']=hex(start+len(bytes.fromhex(executed[start])))
  else:ranges.append(dict(start=hex(start),end=hex(start+len(bytes.fromhex(executed[start]))),hex=executed[start]))
 for r in ranges:r['sha256']=sha(bytes.fromhex(r['hex']))
 extra_ranges={}
 for name,start,end in [('select',0x4be5c0,0x4be601),('belongs',0x4be610,0x4be652),('mode',0x4be590,0x4be5b2),('idle',0x4be660,0x4be679),('playback_fragment',0x44f8a1,0x44f8e6),('actor_default_stance',0x4cfc80,0x4cfcbe)]:
  raw=original.range(start,end-start);assert raw==modified.range(start,end-start);extra_ranges[name]=dict(start=hex(start),end=hex(end),sha256=sha(raw),hex=raw.hex())
 report=dict(schema=1,original_sha256=sha(original.b),v14_sha256=sha(modified.b),cases=cases,ranges=ranges,extra_ranges=extra_ranges,selection=selection_cases(),playback=playback_cases(),actor_dispatch=actor_dispatch_cases(),
   constants={'move_speed_threshold':struct.unpack('<I',original.range(0x9fc0bc,4))[0]},
   seams=['Original timing and Character branch instructions execute; FX, scheduler, view remap, virtual activation/scale are declared leaves.',
          'Animation loop policies and definitions are finite supplied fixture state, not a claim of source catalog binding.',
          'Playback fragment44F8A1..44F8E6 receives explicit animator/actor/animation/sequence pointers via EBP/ESI/EDI and EBX=0; it proves arithmetic at that bounded store boundary, not initialization of those caller registers.',
          'No full world/render-thread clock lifecycle, FX implementation or multiplayer claim.'])
 path=HERE/'native-runtime.json';path.write_text(json.dumps(report,separators=(',',':'))+'\n');print(json.dumps(dict(cases=len(cases),sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
