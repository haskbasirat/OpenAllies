"""Run actual554350 constructor,5531F0/5522D0/552250 and5522B0 setter.
Ordinary actor shape getters execute original bytes. Definition-view fixture
returns remain explicit; defaults are varied and proven fully overwritten.
"""
from pathlib import Path
import copy,json,sys,struct
import random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-spatial'))
from original_query import Query,bits,W,H,original,modified
from unicorn.x86_const import *
from unicorn import UC_HOOK_CODE
from original_rng import Machine as Base
class Context(Query):
 def full(self):
  a=self.QUERY;defs={self.r(ptr+4):member['actor']['definition']for member in self.row['members']for key,ptr in self.pointers.items()if key==member['key']}
  return {'placement':self.prepared()['placement'],'required_actor_flags':self.r(a+0x38),'descriptor':{
   'source_definition':defs.get(self.r(a+0x18)),'source_actor':self.inverse.get(self.r(a+0x1c)),
   'ignore_actor':self.inverse.get(self.r(a+0x20)),'ignore_definition':defs.get(self.r(a+0x24)),
   'collision_owner':self.r(a+0x28)or None,'hostility_context':self.r(a+0x2c)or None,
   'ignore_all':bool(self.u.mem_read(a+0x30,1)[0]),'ignore_moving':bool(self.u.mem_read(a+0x31,1)[0]),'custom_filter':bool(self.r(a+0x34)),'mask':self.r(a+0x3c)}}
 def context(self,defaults,facing):
  self.u.mem_write(self.QUERY,b'\xcd'*0x58);self.u.mem_write(0xb279c0,W(*defaults['point']));self.u.mem_write(0x200c104,W(*defaults['facing'],bits(42)))
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,self.QUERY);self.u.mem_write(self.STACK+4,W(self.pointers[1],int(self.row['strict'])));self.u.emu_start(0x554350,self.STOP,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+12
  assert self.r(self.QUERY+0x40)==self.QUERY and self.r(self.QUERY+0x54)==0
  before=self.full();self.w(self.QUERY+0x54,3)
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,self.QUERY);self.u.mem_write(self.STACK+4,W(*facing));self.u.emu_start(0x5522b0,self.STOP,count=1000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+12 and self.r(self.QUERY+0x54)==0
  return {'context':before,'after_facing':self.full()}
def goal_cases():
 cases=[];visited={};r=random.Random(0x531170)
 samples=[{'position':list(map(bits,[r.uniform(-1e5,1e5),r.uniform(-1e5,1e5)])),'direction':list(map(bits,[r.uniform(-4,4),r.uniform(-4,4)])),'distance':bits(r.uniform(.001,1e4))}for _ in range(96)]
 samples += [{'position':list(map(bits,[1.234567,-8.765432])),'direction':list(map(bits,d)),'distance':bits(distance)}for d in [[0,0],[-0.,-0.],[.317,.733],[1,-1]]for distance in [0,.001,1.23,1234.5]]
 for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  m=Base(cw)
  def hook(u,at,size,_):
   if original.base<=at<original.base+original.image_size:visited[at]=size
  m.u.hook_add(UC_HOOK_CODE,hook)
  for c in samples:
   for variant in ['current','explicit']:
    m.reset(0);m.u.reg_write(UC_X86_REG_ESI,0x200d000);m.w(0x200d004,0x200d100);m.w(0x200d104,0x200d200);m.u.mem_write(0x200d220,W(*c['position']))
    m.u.mem_write(m.STACK+0x70,W(*c['position'],*c['direction'],c['distance']))
    start,end=(0x531187,0x5311c6)if variant=='current'else(0x53136e,0x5313b1)
    m.u.emu_start(start,end,count=1000);assert m.u.reg_read(UC_X86_REG_EIP)==end
    out=m.STACK+0x14 if variant=='current'else m.STACK-8
    cases.append({'precision':pc,'variant':variant,**c,'result':[m.r(out),m.r(out+4)]})
 return cases,visited
def main():
 base=json.loads((ROOT/'research/phase7-spatial/native-query.json').read_text());cases=[];visited={}
 for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  for scenario in range(128):
   row=copy.deepcopy(base['cases'][scenario]['input']);row['strict']=bool(scenario&1);row['extra_flags']=0
   source=row['members'][0];source['actor']['flags']=(0x400 if scenario&2 else 0)|(1<<12 if scenario&4 else 0)|(1<<18 if scenario&8 else 0)|(1<<21 if scenario&16 else 0)
   source['actor']['position']=list(map(bits,[5.23,6.61]));source['actor']['facing']=list(map(bits,[.6,-.8]if scenario&32 else[0,-1]));source['blocking']['kind']=scenario%4
   row['definitions'][str(source['actor']['definition'])]['flags']=source['actor']['flags']|(1<<16 if scenario&64 else 0)
   row['definitions'][str(source['actor']['definition'])]['object_data']=scenario%3!=0
   defaults={'point':list(map(bits,[-99.125,73.42])),'facing':list(map(bits,[17.1,-13.3]))};facing=list(map(bits,[[0,0],[1.7,-.3],[-0.,-1],[.6,.8]][scenario%4]))
   m=Context(row,cw);out=m.context(defaults,facing);cases.append({'precision':pc,'input':row,'defaults':defaults,'facing':facing,'output':out});visited.update(m.visited)
 goals,gv=goal_cases();visited.update(gv);pins=[]
 for a,n in sorted(visited.items()):raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'query_producer_sha256':H((ROOT/'research/phase7-spatial/original_query.py').read_bytes()),'query_fixture_sha256':H((ROOT/'research/phase7-spatial/native-query.json').read_bytes()),'original_sha256':H(original.b),'accepted_sha256':H(modified.b),'cases':cases,'goal_cases':goals,'executed_windows':pins}
 p=HERE/'native-context.json';p.write_text(json.dumps(doc,indent=2)+'\n');print({'cases':len(cases),'sha256':H(p.read_bytes())})
if __name__=='__main__':main()
