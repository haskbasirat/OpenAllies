"""Execute concrete53F320/53EDE0 region search with supplied original-layout graph.
Only allocation/free are hooked. Queue, category checks, distance cache/fallback,
visit/parent writes, neighbor order and final region-list construction execute.
"""
from pathlib import Path
import copy,hashlib,json,random,struct,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
MANAGER,SOLVER,ARRAY,READY,OUTPUT,HEAP,NODES,ALLOC=0x4000000,0x4001000,0x4002000,0x4004000,0x4005000,0x4100000,0x4200000,0x4800000
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
class Machine(Base):
 def __init__(self,case):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[case['precision']]);self.c=case;self.u.mem_map(0,0x1000);self.u.mem_map(0x4000000,0x1000000);self.executed={};self.alloc=ALLOC;self.popped=[];self.discovered=[];self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,a,*v):self.u.mem_write(a,struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v)))
 def hook(self,u,a,size,_):
  if original.base<=a<original.base+original.image_size:self.executed[a]=size
  if a in [0x6f7810,0x6f7430]:
   sp=u.reg_read(UC_X86_REG_ESP);value=self.alloc
   if a==0x6f7810:self.alloc+=(self.r(sp+4)+15)&~15
   u.reg_write(UC_X86_REG_EIP,self.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4);u.reg_write(UC_X86_REG_EAX,value);return
  if a==0x53eef9:self.popped.append(list(struct.unpack('<4I',u.mem_read(SOLVER+0x1c,16))))
  if a==0x53e840:self.discovered.append([self.r(SOLVER+0x18),self.r(SOLVER+0x2c)])
 def run(self):
  c=self.c;self.reset(0);self.put(0xb3652c,MANAGER);self.put(0xb36528,READY);self.u.mem_write(READY+0x20,bytes([c['ready']]));self.u.mem_write(0xb3d43d,bytes([c['busy']]))
  self.put(MANAGER+0x14,ARRAY,len(c['nodes']));self.u.mem_write(MANAGER+0x3c,bytes([c['epoch']]))
  for i,node in enumerate(c['nodes']):
   ptr=NODES+i*0x1000;self.put(ARRAY+i*4,ptr);self.put(ptr,i,ptr+0x100);self.put(ptr+0x100,ptr+0x110);self.put(ptr+0x118,node['category'])
   self.put(ptr+0x54,*node['position_bits']);self.put(ptr+0x5c,ptr+0x200,1,0,0)
   self.u.mem_write(ptr+0x6d,bytes([node['visit_epoch'],node['parent_neighbor']]))
   self.u.mem_write(ptr+0x70,bytes([len(node['neighbors'])]));self.put(ptr+0x74,ptr+0x300)
   for j,n in enumerate(node['neighbors']):self.put(ptr+0x300+j*4,NODES+n*0x1000)
   previous=0
   for j,(key,value) in enumerate(node['distance_cache'].items()):
    cache=ptr+0x800+j*16;self.put(cache,int(key),value,previous);previous=cache
   self.put(ptr+0x200,previous)
  strategy=SOLVER+0x34;queue=SOLVER+0x48
  self.put(SOLVER,0xa3680c,MANAGER,queue,strategy);self.put(strategy,0xa367b0,0xffffffff,0xffffffff,0,0)
  self.put(queue,0xa367f0,HEAP,0,65536,c['queue_limit'] or 0)
  self.put(self.STACK,self.STOP,OUTPUT,c['start'],c['goal'],0);self.u.reg_write(UC_X86_REG_ECX,SOLVER)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x53f320,self.STOP,count=20000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+20
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  ids=[];ptr=self.r(OUTPUT);previous=0
  while ptr:
   assert self.r(ptr+4)==previous and len(ids)<=len(c['nodes'])
   ids.append(self.r(ptr));previous=ptr;ptr=self.r(ptr+8)
  assert previous==self.r(OUTPUT+4)
  states=[]
  for i in range(len(c['nodes'])):states.append(list(self.u.mem_read(NODES+i*0x1000+0x6d,2)))
  return {'status':self.u.reg_read(UC_X86_REG_EAX),'route':ids,'popped':self.popped,'discovered':self.discovered,'node_states':states,'epoch':self.u.mem_read(MANAGER+0x3c,1)[0],'busy':bool(self.u.mem_read(0xb3d43d,1)[0]),'output_linked':self.r(strategy+0x10)!=0,'queue_remaining':self.r(queue+8),'abi_preserved':True}
def graph(name,positions,edges,start=0,goal=None,**extra):
 ns=[{'position_bits':list(map(bits,p)),'category':1,'neighbors':[],'distance_cache':{},'visit_epoch':0,'parent_neighbor':255} for p in positions]
 for a,b in edges:ns[a]['neighbors'].append(b);ns[b]['neighbors'].append(a)
 return {'name':name,'precision':'Double53','ready':True,'busy':False,'epoch':0,'start':start,'goal':len(ns)-1 if goal is None else goal,'queue_limit':None,'nodes':ns,**extra}
def cases():
 rows=[graph('single',[(0,0)],[]),graph('line',[(0,0),(1,0),(2,0),(3,0)],[(0,1),(1,2),(2,3)]),graph('diamond',[(0,0),(1,1),(1,-1),(2,0)],[(0,1),(0,2),(1,3),(2,3)]),graph('triangle',[(.1,.2),(3.7,4.2),(8.3,-2.1)],[(0,1),(0,2),(1,2)]),graph('cycle',[(0,0),(2,0),(2,2),(0,2),(3,3)],[(0,1),(1,2),(2,3),(3,0),(2,4)])]
 for label,changes in [('busy',{'busy':True}),('not_ready',{'ready':False}),('rollover',{'epoch':255}),('same_region',{'goal':0})]:
  c=copy.deepcopy(rows[1]);c.update(changes);c['name']=label;rows.append(c)
 c=copy.deepcopy(rows[1]);c['name']='category_mismatch';c['nodes'][-1]['category']=2;rows.append(c)
 c=copy.deepcopy(rows[2]);c['name']='cached_distance';c['nodes'][0]['distance_cache']={'3':bits(9.75),'1':bits(4.5),'2':bits(.125)};rows.append(c)
 c=copy.deepcopy(rows[2]);c['name']='neighbor_order_reversed';c['nodes'][0]['neighbors'].reverse();rows.append(c)
 c=copy.deepcopy(rows[2]);c['name']='equal_centers';c['nodes']=[{**n,'position_bits':[0,0]} for n in c['nodes']];rows.append(c)
 c=copy.deepcopy(rows[2]);c['name']='foreign_category_neighbor';c['nodes'][1]['category']=2;rows.append(c)
 c=copy.deepcopy(rows[2]);c['name']='bounded_two';c['queue_limit']=2;rows.append(c)
 c=copy.deepcopy(rows[1]);c['name']='bounded_line_one';c['queue_limit']=1;rows.append(c)
 c=copy.deepcopy(rows[2]);c['name']='rollover_stale_nodes';c['epoch']=255
 for node in c['nodes']:node['visit_epoch']=17;node['parent_neighbor']=0
 rows.append(c)
 c=copy.deepcopy(rows[2]);c['name']='negative_cached_pair';c['nodes'][0]['distance_cache']={'3':bits(-1.25),'1':bits(-.5)};rows.append(c)
 rows.append(graph('depth255_failure',[(i*2,0) for i in range(260)],[(i,i+1) for i in range(259)]))
 c=graph('heap_equal_children',[(0,0)]*10,[(0,i) for i in range(1,9)]+[(8,9)])
 c['nodes'][0]['distance_cache']={str(i):bits(value) for i,value in enumerate([1,1,1,1,1,2,2,9],1)}
 c['nodes'][0]['distance_cache']['9']=0;rows.append(c)
 rng=random.Random(0x53ede0)
 for trial in range(12):
  positions=[(rng.uniform(-20,20),rng.uniform(-20,20)) for _ in range(12)];edges=[(i,i+1) for i in range(11)]
  for i in range(12):
   for j in range(i+2,12):
    if rng.randrange(6)==0:edges.append((i,j))
  rows.append(graph('generated_'+str(trial),positions,edges))
 return [{**copy.deepcopy(c),'name':c['name']+'_'+p,'precision':p} for p in ['Single24','Double53','Extended64'] for c in rows]
def main():
 fixtures=[];executed={}
 for c in cases():
  try:m=Machine(c);output=m.run()
  except Exception as e:raise RuntimeError(c['name']+' at '+hex(m.u.reg_read(UC_X86_REG_EIP))) from e
  fixtures.append({'input':c,'output':output});executed.update(m.executed)
 ranges=[]
 for a,n in sorted(executed.items()):
  if ranges and ranges[-1][1]==a:ranges[-1][1]=a+n
  else:ranges.append([a,a+n])
 pins=[]
 for a,z in ranges:
  raw=original.range(a,z-a);assert raw==modified.range(a,z-a);pins.append({'start':hex(a),'end_exclusive':hex(z),'sha256':hashlib.sha256(raw).hexdigest()})
 doc={'schema':1,'passed':True,'original_sha256':hashlib.sha256(original.b).hexdigest(),'v14_sha256':hashlib.sha256(modified.b).hexdigest(),'oracle_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'case_count':len(fixtures),'seams':['allocation/free only'],'fixtures':fixtures}
 (OUT/'graph-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n');(OUT/'graph-ranges.json').write_text(json.dumps(pins,indent=2)+'\n');print(json.dumps({'passed':True,'cases':len(fixtures)}))
if __name__=='__main__':main()
