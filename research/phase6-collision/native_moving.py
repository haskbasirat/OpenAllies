"""Unchanged 4CEDE0 recursive moving-state predicate; no behavior hooks."""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn.x86_const import *
bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
sha=lambda b:hashlib.sha256(b).hexdigest()
ap=lambda n:0 if n is None else 0x4000000+int(n)*0x1000
cp=lambda n:0 if n is None else 0x4100000+int(n)*0x1000
sp=lambda n:0 if n is None else 0x4200000+int(n)*0x1000

class MovingMachine(Machine):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=c
  self.u.mem_map(0x4000000,0x400000)
  for key,a in c['actors'].items():
   p=ap(key)
   for off,value in [(0x7c,cp(a['component7c'])),(0x6c,cp(a['component6c'])),(0xe4,cp(a['active_component_e4'])),(0x70,sp(a['active_state70'])),(0x20,a['position'][0]),(0x24,a['position'][1])]:self.w(p+off,value)
  for key,a in c['components'].items():
   p=cp(key)
   for off,value in [(4,ap(a['owner'])),(0x40,ap(a['linked_actor40'])),(0xbc,a['comparison_position'][0]),(0xc0,a['comparison_position'][1])]:self.w(p+off,value)
  for key,kind in c['states'].items():self.w(sp(key)+0x74,kind)
 def run(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,ap(self.c['actor']))
  self.u.emu_start(0x4cede0,self.STOP,count=30000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  return bool(self.u.reg_read(UC_X86_REG_EAX)&255)

def cases():
 a={'component7c':1,'component6c':None,'active_component_e4':None,'active_state70':1,'position':[bits(3.),bits(7.)]}
 c={'owner':1,'linked_actor40':None,'comparison_position':list(a['position'])}
 base={'actor':1,'actors':{'1':a,'2':{**a,'component7c':None}},'components':{'1':c},'states':{'1':2}}
 rows=[]
 def add(name,change):
  x=copy.deepcopy(base);change(x);rows.append({'name':name,**x})
 add('unchanged',lambda x:None)
 for c6,ce in [(None,None),(2,None),(None,2),(2,2),(2,3)]:add(f'no7c_{c6}_{ce}',lambda x,c6=c6,ce=ce:x['actors']['1'].update(component7c=None,component6c=c6,active_component_e4=ce))
 add('no_state',lambda x:x['actors']['1'].update(active_state70=None))
 for k in [0,1,2,3,0xffffffff]:
  for linked in [None,2]:
   for target_moving in [False,True]:
    add(f'kind{k}_link{linked}_moving{target_moving}',lambda x,k=k,l=linked,m=target_moving:(x['states'].update({'1':k}),x['components']['1'].update(linked_actor40=l),x['actors']['2'].update(component6c=2,active_component_e4=2 if m else None)))
 for axis in [0,1]:
  for actual,previous in [(0,0x80000000),(1,0),(bits(8),bits(7)),(0x7f800000,0x7f800000),(0xff800000,0xff800000),(0x7fc00001,0x7fc00001),(0x7f800001,0),(0x80000001,0),(0x7f7fffff,0x7f7ffffe)]:
   add(f'float_{axis}_{actual}_{previous}',lambda x,a=axis,v=actual,p=previous:(x['actors']['1']['position'].__setitem__(a,v),x['components']['1']['comparison_position'].__setitem__(a,p)))
 add('other_owner',lambda x:(x['components']['1'].update(owner=2),x['actors']['2'].update(active_state70=1,position=[bits(99),bits(7)])))
 for depth in [2,3,16,64]:
  for moving in [False,True]:
   x=copy.deepcopy(base)
   for i in range(1,depth):
    x['actors'][str(i)]={**copy.deepcopy(a),'component7c':i,'active_state70':i}
    x['components'][str(i)]={**copy.deepcopy(c),'owner':i,'linked_actor40':i+1}
    x['states'][str(i)]=2
   x['actors'][str(depth)]={**copy.deepcopy(a),'component7c':None,'component6c':2,'active_component_e4':2 if moving else None}
   rows.append({'name':f'chain_{depth}_{moving}',**x})
 for p in ['Single24','Double53','Extended64']:
  for row in rows:yield {**row,'precision':p}

def main():
 rows=[{'input':c,'output':MovingMachine(c).run()} for c in cases()]
 ranges={'moving':(0x4cede0,0x4cee37),'active_state':(0x578d00,0x578d07),'linked_actor':(0x5780f0,0x578109),'position_equal':(0x578e10,0x578e43)}
 for a,b in ranges.values():assert original.range(a,b-a)==modified.range(a,b-a)
 doc={'schema':1,'source_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'method':'Unchanged original4CEDE0 and all recursive578D00/5780F0/578E10 leaves. No behavior hooks. Stable input component field graph, including NaN, signedzero, infinity and long linked chains.','ranges':{n:{'start':hex(a),'end_exclusive':hex(b),'sha256':sha(original.range(a,b-a))}for n,(a,b)in ranges.items()},'cases':rows}
 path=HERE/'native-moving-fixtures.json';path.write_text(json.dumps(doc,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
