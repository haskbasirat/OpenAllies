"""Original5705E0/57C800/57C680 plus578E90/578E10/4515F0 execute unchanged.

Mind predicates, ground relocation, Organization interpolation57C080, and queue
lookup/schedule are required live services. No movement or clear-collision result
is fabricated by the Rust implementation.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ACTORS,ORG,ELEMENT,MINDS,VT,WORLD,STUB=0x4000000,0x4100000,0x4200000,0x4300000,0x4400000,0x4500000,0x4600000
ptr=lambda base,n:0 if n is None else base+n*0x1000
identity=lambda base,p:None if p==0 else(p-base)//0x1000
bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
sha=lambda b:hashlib.sha256(b).hexdigest()
class PositionMachine(Machine):
 def __init__(self,c):
  super().__init__();self.c=copy.deepcopy(c);self.events=[];self.visited={};self.calls={0x64:0,0x7c:0};self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x800000)
  self.w(0xb36534,WORLD);self.w(WORLD+0xd8,c['world_mode']);self.w(ELEMENT+4,ptr(ACTORS,1));self.w(ELEMENT+0x10,ptr(ACTORS,c['element_organization']));self.w(ptr(ACTORS,2)+0x7c,ORG)
  for i in [10,11,12]:self.w(ptr(MINDS,i),VT)
  self.w(VT+0x64,STUB);self.w(VT+0x7c,STUB+16);self.write_state();self.u.hook_add(UC_HOOK_CODE,self.hook)
 def write_state(self):
  self.w(ORG+4,ptr(ACTORS,self.c['owner']));self.w(ORG+0x40,ptr(ACTORS,self.c['selected_member']))
  for i,v in enumerate(self.c['goal']):self.w(ORG+0xbc+i*4,v)
  for k,a in self.c['actors'].items():
   p=ptr(ACTORS,int(k));self.w(p+8,a['flags']);self.w(p+0x70,ptr(MINDS,a['mind']))
   for i,v in enumerate(a['position']):self.w(p+0x20+i*4,v)
 def ret(self,value=0,pop=0):s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop)
 def event(self,name,args):
  self.c['goal']=[self.r(ORG+0xbc),self.r(ORG+0xc0)];self.events.append([name,*args,self.c['goal'].copy()]);mutations=self.c['mutations']
  if mutations and mutations[0]['event']==name:
   m=mutations.pop(0)
   for key in ['owner','selected_member','goal']:
    if key in m:self.c[key]=m[key]
   for k,fields in m.get('actors',{}).items():self.c['actors'][k].update(fields)
   self.write_state()
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  cx=u.reg_read(UC_X86_REG_ECX);s=u.reg_read(UC_X86_REG_ESP)
  if pc in [STUB,STUB+16]:
   slot=0x64 if pc==STUB else 0x7c;self.event('predicate',[identity(MINDS,cx),slot]);answers=self.c['predicates'][str(slot)];n=self.calls[slot];self.calls[slot]+=1;self.ret(int(answers[min(n,len(answers)-1)]))
  elif pc==0x4eeb40:self.event('relocate',[identity(ACTORS,cx),[self.r(s+4),self.r(s+8)]]);self.ret(pop=8)
  elif pc==0x57c080:self.event('interpolate',[2]);self.ret(int(self.c['interpolate']))
  elif pc==0x50b220:self.event('queued',[2,self.r(s+4),self.r(s+8)]);self.ret(int(self.c['queued']),8)
  elif pc==0x50ac00:self.event('schedule',[2,self.r(s+4)]);self.ret(pop=4)
 def run(self):
  self.reset(0);op=self.c['operation'];self.u.reg_write(UC_X86_REG_ECX,ELEMENT if op=='element' else ORG)
  if op=='set':
   for i,v in enumerate(self.c['point']):self.w(self.STACK+4+i*4,v)
  else:self.w(self.STACK+4,ptr(ACTORS,self.c['member']))
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start({'element':0x5705e0,'notify':0x57c800,'set':0x57c680}[op],self.STOP,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+{'element':4,'notify':8,'set':12}[op]
  assert[self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  assert not self.c['mutations'],self.c['name'];return {'goal':[self.r(ORG+0xbc),self.r(ORG+0xc0)],'events':self.events}
def cases():
 base={'name':'base','operation':'element','world_mode':0,'element_organization':2,'owner':2,'member':1,'selected_member':1,'goal':[bits(10),bits(20)],'point':[bits(30),bits(40)],'actors':{'1':{'flags':0,'position':[bits(30),bits(40)],'mind':10},'2':{'flags':0,'position':[bits(10),bits(20)],'mind':11},'3':{'flags':0,'position':[bits(50),bits(60)],'mind':12},'4':{'flags':0,'position':[bits(10),bits(20)],'mind':12}},'predicates':{'100':[False],'124':[True]},'interpolate':False,'queued':False,'mutations':[]}
 rows=[]
 def add(name,change):c=copy.deepcopy(base);c['name']=name;change(c);rows.append(c)
 for op in ['element','notify','set']:
  for flags in [0,1<<30]:
   for p64,p7c in [(False,False),(False,True),(True,False),(True,True)]:add(f'{op}_flags{flags}_pred{p64}_{p7c}',lambda c,o=op,f=flags,a=p64,b=p7c:(c.update(operation=o,predicates={'100':[a],'124':[b]}),c['actors']['2'].update(flags=f)))
 for mode in [0,5,6,7]:add(f'worldmode{mode}',lambda c,m=mode:c.update(world_mode=m))
 for selected in [None,1,3]:add(f'selected{selected}',lambda c,s=selected:c.update(selected_member=s))
 add('no_organization',lambda c:c.update(element_organization=None))
 add('owner_mind_absent',lambda c:c['actors']['2'].update(mind=None))
 add('dual_flag30',lambda c:(c['actors']['1'].update(flags=1<<30),c['actors']['2'].update(flags=1<<30)))
 for interpolate in [False,True]:
  for queued in [False,True]:add(f'interpolate{interpolate}_queued{queued}',lambda c,i=interpolate,q=queued:c.update(interpolate=i,queued=q))
 for goal in [[bits(30),bits(40)],[bits(9),bits(20)],[bits(10),bits(21)],[0,0x80000000],[0x7fc00001,bits(20)],[0x7f800000,bits(20)]]:add(f'goal{goal}',lambda c,g=goal:c.update(goal=g))
 for pos in [[bits(30),bits(40)],[bits(10),bits(20)],[bits(10),bits(21)],[0,0],[0x80000000,0],[0x7fc00001,bits(20)],[0x7f800000,bits(20)]]:add(f'ownerXY{pos}',lambda c,p=pos:c['actors']['2'].update(position=p))
 add('predicate_changes_mind',lambda c:c.update(mutations=[{'event':'predicate','actors':{'2':{'mind':12}}}]))
 add('predicate_changes_selected_member',lambda c:c.update(mutations=[{'event':'predicate','selected_member':3}]))
 add('predicate_changes_owner',lambda c:c.update(mutations=[{'event':'predicate','owner':4}]))
 add('predicate_changes_goal',lambda c:c.update(mutations=[{'event':'predicate','goal':[bits(30),bits(40)]}]))
 add('interpolate_changes_goal',lambda c:c.update(mutations=[{'event':'interpolate','goal':[bits(70),bits(80)]}]))
 return rows
def main():
 rows=[];visited={}
 for c in cases():
  try:m=PositionMachine(c);result=m.run()
  except Exception:print(c['name']);raise
  rows.append({'input':c,'output':result});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
 d={'schema':1,'method':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'cases':rows}
 p=HERE/'native-position.json';p.write_text(json.dumps(d,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
