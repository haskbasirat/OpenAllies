"""Independent actual570020 outer refresh with real coupled Kingdom arithmetic.

Only source/Organization vector getters, provider predicate, allegiance and
parent refresh are external world seams. Resource ctor/copy/equality/clear and
4E48C0/4E4930/shortage arithmetic execute from original bytes.
"""
from pathlib import Path
import copy,csv,hashlib,importlib.util,itertools,json,random,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
spec=importlib.util.spec_from_file_location('economy_base',ROOT/'research/evidence/economy-phase2-oracle.py');base=importlib.util.module_from_spec(spec);spec.loader.exec_module(base)
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
ACTOR,COMPONENT,ELEMENT,ORGANIZATION,DATA,VT,OWNER=range(0x2070000,0x2077000,0x1000)
K2=0x2060000
class Machine(base.Machine):
 def __init__(self,c):
  self.c=copy.deepcopy(c);self.events=[];self.visited={};self.calls={};self.owners=[]
  super().__init__(c['catalog'],c['kingdoms'][0],c['limits'],dict(base.MODES)[c['precision']])
  self.kingdoms=[self.KINGDOM,K2];self.arrays=[self.ARRAYS,{k:K2+0x1000+i*0x1000 for i,k in enumerate(self.ARRAYS)}]
  for field,offset in [('raw_provided',0x1ac),('effective_provided',0x1a0),('consumed',0x1b8),('stockpiled',0x1c4)]:
   self.put(K2+offset,self.arrays[1][field]);self.put(K2+offset+4,self.count);self.vector(self.arrays[1][field],c['kingdoms'][1][field])
  self.put(ACTOR,VT);self.put(VT+0x108,OWNER);self.put(ACTOR+4,DATA);self.put(COMPONENT+4,ACTOR)
  reason=c['block_reason'];self.put(ACTOR+8,1 if reason=='destructed' else 0);self.put(ACTOR+0x80,ELEMENT if reason=='linked' or c['parent'] else 0);self.put(ELEMENT+0x10,ORGANIZATION if reason=='linked' or c['parent'] else 0)
  self.put(ACTOR+0x7c,ORGANIZATION if reason=='nonpaying_organization' else 0);self.put(DATA+0x178,0 if reason=='nonpaying_organization' else 1)
  self.uc.mem_write(COMPONENT+0x10,bytes(c['flags']))
  for i,values in enumerate(c['resources']):
   self.call(0x4cd440,[],this=COMPONENT+0x14+i*16);self.set_resource(i,values)
  self.events=[];self.calls={}
 def get_resource(self,i):
  q=COMPONENT+0x14+i*16;n=self.read(q+8);return list(struct.unpack('<'+'I'*n,self.uc.mem_read(self.read(q+4),4*n))) if n else []
 def set_resource(self,i,values):self.vector(self.read(COMPONENT+0x18+i*16),values)
 def flags(self):return list(self.uc.mem_read(COMPONENT+0x10,2))
 def snapshot(self):return {'flags':self.flags(),'resources':[self.get_resource(i) for i in range(4)]}
 def event(self,name,extra=None):
  call=self.calls.get(name,0);self.calls[name]=call+1;event={'name':name}
  if name not in ['Kingdom','RefreshParent']:event['component']=self.snapshot()
  if extra is not None:event['value']=extra
  self.events.append(event)
  for m in self.c.get('mutations',[]):
   if m['on']!=name or m.get('call',0)!=call:continue
   if 'flags' in m:self.uc.mem_write(COMPONENT+0x10,bytes(m['flags']))
   for index,values in m.get('resources',{}).items():self.set_resource(int(index),values)
   if 'parent' in m:self.put(ACTOR+0x80,ELEMENT if m['parent'] else 0);self.put(ELEMENT+0x10,ORGANIZATION if m['parent'] else 0)
  return call
 def ret(self,pop=0,value=0):
  sp=self.uc.reg_read(UC_X86_REG_ESP);self.uc.reg_write(UC_X86_REG_EIP,self.read(sp));self.uc.reg_write(UC_X86_REG_ESP,sp+4+pop);self.uc.reg_write(UC_X86_REG_EAX,value)
 def observe(self,u,a,n,_):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  sp=u.reg_read(UC_X86_REG_ESP)
  if a==0x6f7810:
   size=self.read(sp+4);assert size<100000;result=self.heap;self.heap+=max(16,(size+15)&~15);assert self.heap<0x20f0000;self.ret(value=result)
  elif a==0x6f7430:self.ret()
  elif a==OWNER:
   index=self.calls.get('Owner',0);owners=self.c['owners'];owner=owners[min(index,len(owners)-1)];self.event('Owner',owner);self.ret(value=0 if owner is None else self.kingdoms[owner])
  elif a==0x570051:self.event('UpkeepBlocked')
  elif a in [0x56fb10,0x56fc20,0x56fa30]:
   if a==0x56fc20:assert self.read(sp+12)==1
   name={0x56fb10:'UpkeepValues',0x56fc20:'ProvidedValues',0x56fa30:'CostValues'}[a];self.event(name,True if a==0x56fc20 else None)
   q=self.read(sp+4);values=self.c[{'UpkeepValues':'upkeep','ProvidedValues':'provided','CostValues':'cost'}[name]]
   assert self.read(q+8)==self.count;self.vector(self.read(q+4),values);self.ret()
  elif a==0x56f9a0:
   index=self.calls.get('Provides',0);values=self.c['provides'];value=values[min(index,len(values)-1)];self.event('Provides',value);self.ret(value=int(value))
  elif a in [0x4e48c0,0x4e4930]:
   owner=self.kingdoms.index(u.reg_read(UC_X86_REG_ECX));self.event('Kingdom',owner)
  elif a==0x5701d7:self.event('Parent')
  elif a==0x4cf0f0:self.event('RefreshParent',2);self.ret()
 def run(self):
  self.call(0x570020,[],this=COMPONENT)
  return {'component':self.snapshot(),'kingdoms':[{field:self.values(address) for field,address in arrays.items()} for arrays in self.arrays],'events':self.events,'abi_preserved':True}
def cases():
 catalog=base.catalog(['Stockpiled','Upkeep','Upkeep','Upkeep','LimitedConsumed'],[[0]*5,[.1,0,0,0,0],[.3,0,0,0,0],[0]*5,[0]*5],limited=(3,))
 def vec(values):return [base.bits(x) for x in values]
 state={'raw_provided':vec([50,10,7,20,0]),'effective_provided':vec([50,10,7,20,0]),'consumed':vec([3,12,9,0,1]),'stockpiled':vec([500,0,0,0,0])}
 initial={'catalog':catalog,'kingdoms':[state,copy.deepcopy(state)],'limits':{'provided_maximum':100,'stockpiled_maximum':base.bits(99999)},'resources':[vec([1,2,3,4,0]),vec([.1,2,3,0,1]),vec([5,6,7,8,9]),vec([9,8,7,6,5])],'flags':[0,0],'block_reason':'none','parent':False,'owners':[0],'provides':[True],'upkeep':vec([.1,2,3,0,1]),'provided':vec([1,2,3,4,0]),'cost':vec([11,12,13,14,15]),'mutations':[]};rows=[]
 def add(name,fn=None):
  c=copy.deepcopy(initial);c['name']=name
  if fn:fn(c)
  rows.append(c)
 for f10,f11,equal,providers in itertools.product([0,1],[0,1],[False,True],[[False],[True],[True,False]]):
  add(f'flags{f10}{f11}_equal{equal}_provides{providers}',lambda c,a=f10,b=f11,e=equal,p=providers:c.update(flags=[a,b],upkeep=c['resources'][1] if e else vec([.2,4,5,0,2]),provided=c['resources'][0] if e else vec([1.1,5,6,101,0]),provides=p))
 for reason in ['linked','destructed','nonpaying_organization']:
  for enabled in [0,1]:add(f'blocked_{reason}_{enabled}',lambda c,r=reason,e=enabled:c.update(block_reason=r,flags=[1,e]))
 add('null_initial_owner',lambda c:c.update(owners=[None],flags=[1,1]))
 add('removal_owners_differ',lambda c:c.update(owners=[0,1,1],flags=[1,1],upkeep=vec([2,3,4,0,2]),provided=vec([2,4,6,101,0])))
 add('removal_owner_null',lambda c:c.update(owners=[0,None,None],flags=[1,1],upkeep=vec([2,3,4,0,2]),provided=vec([2,4,6,101,0])))
 add('blocked_remove_owner_null',lambda c:c.update(owners=[0,None,None],flags=[1,1],block_reason='destructed',provides=[False]))
 add('parent_appears_at_cost',lambda c:c.update(mutations=[{'on':'CostValues','parent':True}]))
 add('parent_disappears_at_cost',lambda c:c.update(parent=True,mutations=[{'on':'CostValues','parent':False}]))
 add('getter_changes_flags',lambda c:c.update(flags=[1,1],mutations=[{'on':'UpkeepValues','flags':[0,0]},{'on':'ProvidedValues','flags':[0,1]}]))
 add('owner_callback_changes_old_vectors',lambda c:c.update(flags=[1,1],upkeep=vec([2,3,4,0,2]),provided=vec([2,4,6,101,0]),mutations=[{'on':'Owner','call':1,'resources':{'1':vec([.5,1,2,0,1])}},{'on':'Owner','call':2,'resources':{'0':vec([.5,1,2,3,0])}}]))
 add('provided_member_cache_written',lambda c:c.update(mutations=[{'on':'ProvidedValues','resources':{'3':vec([1,1,2,3,5])}}]))
 add('signed_zero_equal',lambda c:(c.update(flags=[1,1],upkeep=[0]*5,provided=[0]*5),c['resources'].__setitem__(0,[0x80000000]*5),c['resources'].__setitem__(1,[0x80000000]*5)))
 add('empty_resource_domain',lambda c:c.update(catalog={'resources':[]},kingdoms=[{k:[] for k in state},{k:[] for k in state}],resources=[[],[],[],[]],upkeep=[],provided=[],cost=[]))
 rng=random.Random(0x570020)
 for i in range(12):
  add(f'finite_rounding_{i}',lambda c:c.update(flags=[1,1],upkeep=vec([rng.uniform(-.001,.001),rng.uniform(0,20),rng.uniform(0,20),0,1]),provided=vec([rng.uniform(-.001,.001),rng.uniform(0,20),rng.uniform(0,20),rng.uniform(90,110),0])))
 for pc,_ in base.MODES:
  for c in rows:yield {**copy.deepcopy(c),'name':c['name']+'_'+pc,'precision':pc}
def main():
 rows=[];visited={};equality=[]
 for c in cases():
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 # Execute the original vector equality independently. Nonfinite arithmetic is
 # outside aa-economy's domain, but equality itself has a defined native result.
 words=[0,0x80000000,1,0x80000001,0x3f800000,0xbf800000,0x7f7fffff,0x7f800000,0xff800000,0x7fc00000,0x7f800001,0xffc00001]
 pairs=[([a],[b]) for a,b in itertools.product(words,repeat=2)]
 pairs += [([],[]),([],[0]),([0],[]),([0,0x3f800000],[0x80000000,0x3f800000]),([0,0x7fc00000],[0,0x7fc00000]),([0,1],[0,2])]
 seed=next(cases())
 for pc,_ in base.MODES:
  c={**copy.deepcopy(seed),'precision':pc};m=Machine(c)
  for a,b in pairs:
   for i,values in enumerate([a,b]):
    q=COMPONENT+0x14+i*16;m.put(q+8,len(values));m.set_resource(i,values)
   m.call(0x4cce70,[COMPONENT+0x24],this=COMPONENT+0x14)
   equality.append({'precision':pc,'a':a,'b':b,'equal':bool(m.uc.reg_read(UC_X86_REG_EAX)&255),'abi_preserved':True})
  visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=base.ORIGINAL.at(a,n);assert raw==base.ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':base.ORIGINAL_SHA,'v14_sha256':base.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'equality_cases':equality,'visited_windows':windows,'byte_window_scope':'Visited instruction bytes include replaced seam entry instructions; bytes behind seams are not claimed as executed.','seams':['allegiance virtual108, with captured/requeried owners','56FB10/56FC20/56FA30 value computations supplied by explicit fixtures','56F9A0 provider gate','4CF0F0 parent refresh','heap allocation/free'],'scope':'Actual570020 and removal/vector/Kingdom arithmetic; finite resource arithmetic, coherent immutable component owner and registry during each call. Separate4CCE70 equality includes signed zero, infinities, qNaN and sNaN with masked exceptions.'}
 path=HERE/'native-refresh-fixtures.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'equality_cases':len(equality),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
