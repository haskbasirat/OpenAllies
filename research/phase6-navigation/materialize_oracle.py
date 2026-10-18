"""Original predecessor-path materialization with explicit LOS/collision seams."""
from pathlib import Path
import copy,hashlib,json,struct,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase3-terrain'))
from terrain_uv_oracle import Pe,ORIGINAL_SHA,ACCEPTED_SHA
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
STRATEGY,DESCRIPTOR,PATH,WORLD,GRID,CELLS,VTABLE,STUB,POOL,STOP=[0x2000000+i*0x1000 for i in range(10)]
STACK=0x3008000
DELTAS=[(0,0),(-1,-1),(-1,0),(-1,1),(0,1),(1,1),(1,0),(1,-1),(0,-1)]
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
class Machine:
 def __init__(self,pe,c):
  self.c=c;self.u=Uc(UC_ARCH_X86,UC_MODE_32);pe.load(self.u);self.u.mem_map(0x2000000,0x20000);self.u.mem_map(0x3000000,0x10000)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']])
  s=c['state'];g=c['grid'];self.put(STRATEGY,VTABLE,DESCRIPTOR);self.put(STRATEGY+0xc,*s['start_cell'],*s['goal_cell'],PATH,*s['start_bits'],*s['goal_bits'])
  self.put(DESCRIPTOR+0x28,s['collision_word28']);self.put(VTABLE+0x1c,STUB)
  self.put(0xb36534,WORLD);self.put(WORLD+0x30,GRID);self.put(GRID+4,g['coordinate_shift'],g['cell_size_bits']);self.put(GRID+0x1c,g['width'],g['height']);self.put(GRID+0x28,CELLS)
  for i,d in enumerate(g['predecessors']):self.u.mem_write(CELLS+i*0x30+0x2d,bytes([d]))
  self.next_node=0x2010000;self.put(0xb27ae0,POOL);self.write_path(c['path']);self.events=[];self.trace=-1;self.trace_index=0;self.executed={}
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,a,*v):self.u.mem_write(a,struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v)))
 def get(self,a,n=1):return list(struct.unpack('<'+'I'*n,self.u.mem_read(a,n*4)))
 def args(self,n):return self.get(self.u.reg_read(UC_X86_REG_ESP)+4,n)
 def ret(self,n=0,value=1):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.get(sp)[0]);self.u.reg_write(UC_X86_REG_ESP,sp+4+n);self.u.reg_write(UC_X86_REG_EAX,value)
 def alloc(self):p=self.next_node;self.next_node+=0x100;return p
 def write_path(self,p):
  ptrs=[self.alloc() for _ in p['nodes']];self.put(PATH,0,ptrs[0] if ptrs else 0,ptrs[-1] if ptrs else 0,p['kind1_count'],p['kind2_count']);self.u.mem_write(PATH+0x14,bytes([p['dirty']]));self.put(PATH+0x18,p['word18'])
  for i,(ptr,node) in enumerate(zip(ptrs,p['nodes'])):self.put(ptr,*node['words'],ptrs[i-1] if i else 0,ptrs[i+1] if i+1<len(ptrs) else 0)
 def read_path(self):
  first,last,k1,k2=self.get(PATH+4,4);nodes=[];prev=0;ptr=first
  while ptr:
   assert len(nodes)<100 and self.get(ptr+0x20)[0]==prev
   nodes.append({'words':self.get(ptr,8)});prev=ptr;ptr=self.get(ptr+0x24)[0]
  assert prev==last
  signed=lambda n:n if n<0x80000000 else n-0x100000000
  return {'nodes':nodes,'kind1_count':signed(k1),'kind2_count':signed(k2),'dirty':bool(self.u.mem_read(PATH+0x14,1)[0]),'word18':self.get(PATH+0x18)[0]}
 def event(self,name,**args):self.events.append({'name':name,'collision_word28':self.get(DESCRIPTOR+0x28)[0],**args})
 def trace_cells(self):return self.c['trace_cells'][min(self.trace,len(self.c['trace_cells'])-1)]
 def hook(self,u,a,size,_):
  if 0x400000<=a<0xc00000:self.executed[a]=size
  if a==0x85c680:self.ret(value=self.alloc());return
  if a==0x5581b0:self.ret();return # ray iterator constructor; semantic seam below
  if a==0x4dd030:
   xy=self.args(4);self.trace+=1;self.trace_index=0;self.event('BeginTrace',from_bits=xy[:2],to_bits=xy[2:]);self.ret(16);return
  if a==0x4ddde0:
   cells=self.trace_cells();value=cells[self.trace_index] if self.trace_index<len(cells) else None;self.trace_index+=1
   if value is not None:self.put(self.args(1)[0],*value)
   self.event('NextCell',cell=value);self.ret(4,int(value is not None));return
  if a==STUB:
   index=(self.args(1)[0]-CELLS)//0x30;g=self.c['grid'];cell=[index%g['width'],index//g['width']]
   block=self.c['trace_blocked'][min(self.trace,len(self.c['trace_blocked'])-1)]
   self.event('CellBlocked',cell=cell,result=block);self.ret(4,int(block));return
 def run(self):
  self.put(STACK,STOP,*self.c['end_cell']);self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,STRATEGY)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x53d500,STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==STACK+12
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  state=copy.deepcopy(self.c['state']);state['path_linked']=self.get(STRATEGY+0x1c)[0]!=0;state['collision_linked']=self.get(STRATEGY+4)[0]!=0;state['collision_word28']=self.get(DESCRIPTOR+0x28)[0]
  return {'path':self.read_path(),'state':state,'events':self.events,'abi_preserved':True}
def cases():
 rows=[]
 chains=[('straight',[(1,1),(2,1),(3,1),(4,1),(5,1)]),('corner',[(1,1),(2,1),(3,1),(3,2),(3,3)]),('zigzag',[(1,1),(2,1),(2,2),(3,2),(3,3),(4,3)]),('diagonal',[(1,1),(2,2),(3,3),(4,4)]),('two_turns',[(1,1),(2,1),(3,1),(3,2),(3,3),(4,3),(5,3)])]
 def add(name,chain,blocked,partial=False,cell=4.0,existing=False):
  w=h=8;pred=[0]*(w*h)
  for previous,current in zip(chain,chain[1:]):pred[current[1]*w+current[0]]=DELTAS.index((current[0]-previous[0],current[1]-previous[1]))
  start,end=chain[0],chain[-1];goal=(6,6) if partial else end
  path={'nodes':[],'kind1_count':0,'kind2_count':0,'dirty':False,'word18':123}
  if existing:path={'nodes':[{'words':[bits(28),bits(29),bits(1),0,2,0xdeadbeef,bits(28),bits(29)]}],'kind1_count':0,'kind2_count':1,'dirty':False,'word18':123}
  rows.append({'name':name,'precision':'Double53','end_cell':end,'state':{'start_cell':start,'goal_cell':goal,'start_bits':[bits(start[0]*4+.375),bits(start[1]*4+.625)],'goal_bits':[bits(goal[0]*4+1.125),bits(goal[1]*4+1.75)],'path_linked':True,'collision_linked':True,'collision_word28':0x12345678},'grid':{'width':w,'height':h,'coordinate_shift':2,'cell_size_bits':bits(cell),'predecessors':pred},'path':path,'trace_cells':[[[1,1],[2,1]]],'trace_blocked':blocked})
 for name,chain in chains:
  for label,blocked in [('clear',[False]),('blocked',[True]),('first_blocked',[True,False]),('second_blocked',[False,True])]:
   for partial in [False,True]:add(name+'_'+label+'_'+str(partial),chain,blocked,partial)
 for index,delta in enumerate(DELTAS[1:],1):add('direction_'+str(index),[(3,3),(3+delta[0],3+delta[1])],[False])
 add('existing_region_tail',chains[2][1],[True],existing=True)
 add('fractional_cell',chains[4][1],[True],cell=3.7)
 add('same_cell_distinct',[(2,2)],[False])
 same=copy.deepcopy(rows[-1]);same['name']='same_exact_point';same['state']['goal_bits']=same['state']['start_bits'];rows.append(same)
 tiny=copy.deepcopy(same);tiny['name']='same_tiny_delta';tiny['state']['goal_bits'][0]+=1;rows.append(tiny)
 return [{**copy.deepcopy(row),'name':row['name']+'_'+p,'precision':p} for p in ['Single24','Double53','Extended64'] for row in rows]
def main():
 pe=Pe(ROOT.parent/'AA.exe',ORIGINAL_SHA);v14=Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',ACCEPTED_SHA)
 fixtures=[];executed={}
 for c in cases():
  try:m=Machine(pe,c);output=m.run()
  except Exception as e:raise RuntimeError(c['name']) from e
  fixtures.append({'input':c,'output':output});executed.update(m.executed)
 ranges=[]
 for a,n in sorted(executed.items()):
  if ranges and ranges[-1][1]==a:ranges[-1][1]=a+n
  else:ranges.append([a,a+n])
 pins=[]
 for a,z in ranges:
  raw=pe.at(a,z-a);assert raw==v14.at(a,z-a);pins.append({'start':hex(a),'end_exclusive':hex(z),'sha256':hashlib.sha256(raw).hexdigest()})
 doc={'schema':1,'passed':True,'original_sha256':ORIGINAL_SHA,'v14_sha256':ACCEPTED_SHA,'oracle_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'case_count':len(fixtures),'seams':['native allocator','LOS iterator constructor and begin/next callbacks','strategy cell-blocked callback'],'fixtures':fixtures}
 (OUT/'materialize-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n');(OUT/'materialize-ranges.json').write_text(json.dumps(pins,indent=2)+'\n')
 print(json.dumps({'passed':True,'cases':len(fixtures)}))
if __name__=='__main__':main()
