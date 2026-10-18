"""Execute actual52FE60 soft branch,4EA0F0 descriptors and46F100 geometry.
Prepared destination forces the preceding near-point gate false and target GTID
is valid with flag10. Ordinary4EA020 executes unhooked; derived virtual+10
getters have explicit mutable fixture effects. No range helper is intercepted.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-range'))
from native_range import RangeMachine,SHAPES,bits,placement,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
A,B,VT,GETTER,DEFA,DEFB,MIND,IDS=range(0x4000000,0x4008000,0x1000)
sha=lambda b:hashlib.sha256(b).hexdigest()
class Machine(RangeMachine):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=copy.deepcopy(c);self.events=[];self.visited={};self.u.mem_map(0x4000000,0x50000)
  self.put(VT+0x10,0x4ea020 if c['ordinary'] else GETTER);self.put(VT+0x14,0x4ea030)
  for i,actor in [(1,A),(2,B)]:
   p=c['actors'][str(i)];d=DEFA if p['shape']==1 else DEFB;self.put(actor,VT,d);self.put(actor+8,1<<10);self.put(actor+0x20,*p['position'],0,*p['facing'])
  for i,d in [(1,DEFA),(2,DEFB)]:
   self.shape(d+0x50,c['shapes'][str(i)]);self.shape(d+0x68,SHAPES[0]);self.put(d+0x80,c['override'])
  self.put(self.STATE+4,MIND);self.put(MIND+4,A);self.put(self.STATE+0x10,0x10002,bits(1000),bits(1000));self.put(0xb364c0,IDS);self.u.mem_write(IDS+8,struct.pack('<H',1));self.put(IDS+0x2000c,B)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def snapshot(self,i):
  p=A if i==1 else B;return {'position':[self.r(p+0x20),self.r(p+0x24)],'facing':[self.r(p+0x2c),self.r(p+0x30)]}
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  if pc==0x4ea0f0:
   i=1 if u.reg_read(UC_X86_REG_ECX)==A else 2;self.events.append({'op':'snapshot','actor':i,**self.snapshot(i)})
  if pc in [GETTER,0x4ea020]:
   i=1 if u.reg_read(UC_X86_REG_ECX)==A else 2;self.events.append({'op':'getter','actor':i})
   if pc==0x4ea020:return
   for m in self.c['mutations']:
    if m['on_actor']==i:
     actor=A if m['actor']==1 else B
     if 'position'in m:self.put(actor+0x20,*m['position'])
     if 'facing'in m:self.put(actor+0x2c,*m['facing'])
     if 'shape'in m:self.shape((DEFA if m['shape_id']==1 else DEFB)+0x50,m['shape'])
   shape=self.c['actors'][str(i)]['shape'];sp=u.reg_read(UC_X86_REG_ESP);u.reg_write(UC_X86_REG_EAX,(DEFA if shape==1 else DEFB)+0x50);u.reg_write(UC_X86_REG_EIP,self.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4)
 def run(self):
  self.reset(0);self.u.emu_start(0x52fe60,self.STOP,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  return {'result':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'events':self.events}
def cases():
 rows=[]
 for ordinary in [True,False]:
  for kind in range(4):
   for target in range(4):
    for position in [[1.9,0],[3.1,0],[2.71,-1.333]]:
     rows.append({'name':f'{ordinary}_{kind}_{target}_{position}','ordinary':ordinary,'override':255,'actors':{'1':{'position':[0,0],'facing':list(map(bits,[.6,-.8])),'shape':1},'2':{'position':list(map(bits,position)),'facing':list(map(bits,[-.8,-.6])),'shape':2}},'shapes':{'1':SHAPES[kind],'2':SHAPES[target]},'mutations':[]})
 base=copy.deepcopy(rows[-1]);base['shapes']={'1':SHAPES[1],'2':SHAPES[1]};base['actors']['2']['position']=list(map(bits,[5,0]))
 for on_actor in [1,2]:
  for changed in [1,2]:
   for field,value in [('position',list(map(bits,[20,0]))),('facing',list(map(bits,[1,0]))),('shape',{**SHAPES[1],'radius_bits':bits(20)})]:
    c=copy.deepcopy(base);c.update(name=f'mutation_{on_actor}_{changed}_{field}',mutations=[{'on_actor':on_actor,'actor':changed,field:value,'shape_id':changed}]);rows.append(c)
 for ordinary in [True,False]:
  c=copy.deepcopy(base);c['name']='aliased_'+str(ordinary);c['ordinary']=ordinary;c['actors']['2']['shape']=1
  if not ordinary:c['mutations']=[{'on_actor':2,'actor':1,'shape_id':1,'shape':{**SHAPES[1],'radius_bits':bits(20)}}]
  rows.append(c)
 for pc in ['Single24','Double53','Extended64']:
  for c in rows:yield {**copy.deepcopy(c),'precision':pc,'name':c['name']+'_'+pc}
def main():
 rows=[];visited={}
 for c in cases():m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b))
 ranges={}
 for label,a,end in [('soft_descriptor',0x4ea0f0,0x4ea143),('ordinary_soft_getter',0x4ea020,0x4ea027),('arrival_caller',0x52fe60,0x52fef5)]:
  b=original.range(a,end-a);assert b==modified.range(a,end-a);ranges[label]={'va':hex(a),'length':end-a,'sha256':sha(b)}
 doc={'schema':1,'method':__doc__,'original_sha256':sha(original.b),'accepted_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'ranges':ranges,'cases':rows,'visited_windows':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())]}
 (HERE/'native-soft-range.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
