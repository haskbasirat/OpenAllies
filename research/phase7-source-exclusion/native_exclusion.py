"""Original4D5600/4EB7D0/4EE780; required virtual/component results are fixtures.
All identity, list, definition, ordered-XY, flags and component gates execute.
"""
from pathlib import Path
import json,copy,struct,sys,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ACT=0x4000000;DEF=0x4010000;VT=0x4020000;IDS=0x4030000;NODE=0x40a0000;ELEM=0x40b0000;MIND=0x40c0000;WALL=0x40d0000;HOOK=0x40e0000
F=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
A=lambda k:ACT+k*0x400
D=lambda k:DEF+k*0x800
M=lambda k:MIND+k*0x100
NV=(UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP)
def nonvolatile(u):return tuple(u.reg_read(r)for r in NV)
class Machine(Base):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=copy.deepcopy(c);self.events=[];self.visited={}
  self.u.mem_map(ACT,0x100000);self.w(0xb364c0,IDS)
  for k,x in self.c['actors'].items():
   k=int(k);self.put(A(k),VT+k*0x400,D(x['definition']),x['flags']);self.w(A(k)+0x14,x['reference']);self.put(A(k)+0x20,*x['position']);self.w(A(k)+0x50,x['linked_50']);self.w(A(k)+0x168,x['linked_168']);self.w(A(k)+0x80,ELEM+k*0x100 if x['has_element']else 0);self.w(A(k)+0x90,WALL+k*0x100 if x['has_wall']else 0)
   self.w(ELEM+k*0x100+0x10,A(c['organizations'][str(k)])if c['organizations'][str(k)]else 0);self.w(A(k)+0x70,M(c['minds'][str(k)])if c['minds'][str(k)]else 0)
   for off,code in [(0xb4,HOOK),(0x108,HOOK+1),(0x138,HOOK+2),(0x100,HOOK+4)]:self.w(VT+k*0x400+off,code)
   self.w(M(k),VT+0x2000);self.w(VT+0x2064,HOOK+3)
   ref=x['reference'];self.u.mem_write(IDS+4+(ref&0xffff)*2,struct.pack('<H',ref>>16));self.w(IDS+0x20004+(ref&0xffff)*4,0 if k in c['retired']else A(k))
  for k,d in c['definitions'].items():
   a=D(int(k));self.w(a+0x4e0,D(d['required_object'])if d['required_object']is not None else 0);self.u.mem_write(a+0x1a8,bytes([d['settlement_wall']]));self.u.mem_write(a+0x42c,bytes([d['gate']]))
  self.w(A(1)+0x110,NODE if c['ignored']else 0)
  for i,ref in enumerate(c['ignored']):self.put(NODE+i*12,ref,0,NODE+(i+1)*12 if i+1<len(c['ignored'])else 0)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,a,*v):self.u.mem_write(a,struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v)))
 def ret(self,value=0,n=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+n)
 def event(self,name,**values):
  self.events.append(dict(op=name,**values));m=self.c['mutations'].get(name,{})
  if m.get('dispatch2'):self.w(VT+2*0x400+0x138,HOOK+5)
  if 'target_flags'in m:self.w(A(2)+8,m['target_flags'])
  if 'linked168'in m:self.w(A(1)+0x168,m['linked168'])
  if m.get('remove_other_mind'):self.w(A(4)+0x70,0)
 def hook(self,u,pc,size,_):
  ecx=u.reg_read(UC_X86_REG_ECX);sp=u.reg_read(UC_X86_REG_ESP);cb=self.c['callbacks']
  if pc==HOOK:self.event('category',actor=(ecx-ACT)//0x400);self.ret(int(cb['category']));return
  if pc==HOOK+1:self.event('owner',actor=(ecx-ACT)//0x400);self.ret(cb['owner']);return
  if pc in[HOOK+2,HOOK+5]:self.event('accepts',actor=(ecx-ACT)//0x400,dispatch=2 if pc==HOOK+5 else 1,owner=self.r(sp+4));self.ret(int(cb['accepts2'if pc==HOOK+5 else'accepts']),4);return
  if pc==HOOK+3:
   mind=(ecx-MIND)//0x100;self.event('mind_ready',mind=mind);self.ret(int(cb['ready'][str(mind)]));return
  if pc==0x58cbc0:self.event('wall_allows',actor=(ecx-WALL)//0x100,source=(self.r(sp+4)-ACT)//0x400);self.ret(int(cb['wall_allows']),4);return
  if pc==HOOK+4:self.event('parent',actor=(ecx-ACT)//0x400);self.ret(A(cb['parent'])if cb['parent']else 0);return
  if pc==0x4d36f0:
   parent=self.r(sp+4);self.event('parent_collision',source=(ecx-ACT)//0x400,parent=(parent-ACT)//0x400 if parent else None);self.ret(int(cb['parent_collision']),4);return
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.visited[pc]=raw.hex()
 def run(self):
  self.reset(0);before=nonvolatile(self.u);self.u.reg_write(UC_X86_REG_ECX,A(1));self.w(self.STACK+4,A(self.c['candidate']));self.u.emu_start(0x4d5600,self.STOP,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  assert nonvolatile(self.u)==before
  return {'result':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'events':self.events}
def base():
 return {'candidate':2,'actors':{str(i):{'definition':i,'reference':0x10000+i,'flags':0,'position':[F(5),F(6)],'linked_50':0,'linked_168':0,'has_element':False,'has_wall':False}for i in range(1,5)},'definitions':{str(i):{'required_object':None,'settlement_wall':False,'gate':False}for i in range(1,5)},'ignored':[],'organizations':{'1':3,'2':4,'3':None,'4':None},'minds':{'1':None,'2':None,'3':3,'4':4},'retired':[],'callbacks':{'category':True,'owner':51,'accepts':False,'accepts2':True,'ready':{'3':False,'4':False},'wall_allows':False,'parent':3,'parent_collision':False},'mutations':{}}
def cases():
 rows=[]
 for i in range(112):
  c=base();a=c['actors'];d=c['definitions'];cb=c['callbacks']
  if i==0:c['candidate']=1
  elif i<8:a['1']['linked_50']=[0x10002,0x20002,0x10003,0][(i-1)%4];c['retired']=[2]if i>4 else[]
  elif i<16:c['ignored']=[[0x10003,0x10002,0x10004],[0x20002],[0],[0x10003]][i%4]
  elif i<26:
   d['1']['required_object']=2 if i%2 else 3
   if i==20:a['2']['position']=[F(5.01),F(6)]
   if i==21:a['1']['position']=[0x80000000,0];a['2']['position']=[0,0x80000000]
   if i==23:a['2']['position']=[0x7fc00000,F(6)]
  elif i<48:
   a['1']['has_element']=True;a['2']['has_element']=i%7!=0;cb['accepts']=i%3!=0;cb['ready']['3']=i%4!=0;cb['ready']['4']=i%5!=0
   if i==40:c['mutations']['owner']={'dispatch2':True};cb['accepts']=False
   if i==41:c['mutations']['mind_ready']={'remove_other_mind':True}
   if i==42:c['organizations']['1']=None
   if i==43:c['organizations']['2']=None
   if i==44:c['minds']['3']=None
   if i==45:c['minds']['4']=None
  elif i<60:
   d['1']['settlement_wall']=i%6==0;d['2']['settlement_wall']=True;a['1']['flags']=(1<<10 if i%2 else 0)|(1<<29 if i%3 else 0);a['1']['has_wall']=i%4==0
  elif i<96:
   a['1']['has_wall']=i%7==0;a['2']['has_wall']=i%5!=0;d['2']['gate']=i%3!=0;cb['accepts']=i%4==0;cb['wall_allows']=i%2==0;cb['parent_collision']=i%6==0
   if i==79:cb['parent']=None
  else:
   a['1']['linked_168']=[0x10002,0x20002,0x10003,0][i%4];cb['category']=i%5!=0
   if i==105:c['mutations']['category']={'linked168':0x10002}
   if i==107:c['retired']=[2]
  rows.append(c)
 return [{**copy.deepcopy(c),'precision':pc,'name':f'{pc}_{i}'}for pc in ['Single24','Double53','Extended64']for i,c in enumerate(rows)]

class OrdinaryMachine(Machine):
 def __init__(self,c):
  b=base();b['precision']=c['precision'];super().__init__(b);self.ordinary=c
  self.w(A(2)+0xe8,c['candidate_owner'] or 0);self.w(VT+2*0x400+0x144,HOOK+6)
 def hook(self,u,pc,size,user):
  if pc==HOOK+6:
   self.events.append({'op':'relation','owner':self.r(u.reg_read(UC_X86_REG_ESP)+4) or None});self.ret(self.ordinary['relation'],4);return
  super().hook(u,pc,size,user)
 def run(self):
  c=self.ordinary;self.reset(0);before=nonvolatile(self.u);self.u.reg_write(UC_X86_REG_ECX,A(2));self.w(self.STACK+4,c['owner'] or 0)
  self.u.emu_start(0x4d1eb0,self.STOP,count=1000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  assert nonvolatile(self.u)==before
  result=bool(self.u.reg_read(UC_X86_REG_EAX)&255)
  self.reset(0);before=nonvolatile(self.u);self.u.reg_write(UC_X86_REG_ECX,A(2));self.u.emu_start(0x4d1e80,self.STOP,count=1000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  assert nonvolatile(self.u)==before
  owner=self.u.reg_read(UC_X86_REG_EAX)or None
  self.reset(0);before=nonvolatile(self.u);self.u.reg_write(UC_X86_REG_ECX,A(2));self.u.emu_start(0x6a71d0,self.STOP,count=1000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  assert nonvolatile(self.u)==before
  return {'result':result,'events':self.events,'owner':owner,'category':bool(self.u.reg_read(UC_X86_REG_EAX)&255)}

def ordinary_cases():
 return [{'precision':pc,'candidate_owner':a,'owner':b,'relation':r}for pc in ['Single24','Double53','Extended64']for a,b in [(None,None),(None,10),(10,None),(10,10),(10,20)]for r in [0,1,2,3,0xffffffff]]
def main():
 rows=[];visited={}
 for c in cases():
  try:m=Machine(c);out=m.run()
  except Exception as e:raise RuntimeError(c['name'])from e
  rows.append({'input':c,'output':out});visited.update(m.visited)
 ordinary=[]
 for c in ordinary_cases():
  m=OrdinaryMachine(c);ordinary.append({'input':c,'output':m.run()});visited.update(m.visited)
 table=[]
 for off,expected in [(0x30,0x4d5600),(0xb4,0x6a71d0),(0x100,0x4ced60),(0x108,0x4d1e80),(0x138,0x4d1eb0),(0x144,0x4cf040)]:
  at=0xa273f0+off;raw=original.range(at,4);assert raw==modified.range(at,4)and struct.unpack('<I',raw)[0]==expected
  table.append({'va':hex(at),'bytes':raw.hex(),'target':hex(expected)})
 result={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),'cases':rows,'ordinary_cases':ordinary,'ordinary_vtable':table,'executed_windows':[{'va':hex(a),'bytes':b}for a,b in sorted(visited.items())]}
 p=HERE/'native-exclusion.json';p.write_text(json.dumps(result,indent=2)+'\n');print({'cases':len(rows),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
if __name__=='__main__':main()
