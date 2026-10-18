"""Whole native Solve→endpoint→search→materialization for static collision grids.

Only node allocation/free are replaced. Native tile iteration, collision mask,
cost, heap, predecessors, smoothing, and float arithmetic execute unchanged.
Region pather is disabled in this integration corpus.
"""
from pathlib import Path
import hashlib,json,struct,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase6-pathsearch'))
from native_search import SearchMachine,original,modified,WORLD,GRID,SOLVER,DESC,PATH,TILES,HEAP,REGISTRY,bits
from unicorn.x86_const import *
class SolveMachine(SearchMachine):
 def __init__(self,cw):
  super().__init__(cw);self.executed={};self.node_next=0x4500000;self.trace_events=[]
 def hook(self,u,a,size,user):
  if 0x400000<=a<0xc00000:self.executed[a]=size
  if a in (0x85c680,0x85c2b0):
   sp=self.rsp();value=self.node_next;self.node_next+=0x100
   u.reg_write(UC_X86_REG_EIP,self.r(sp));u.reg_write(UC_X86_REG_ESP,sp+(4 if a==0x85c680 else 8));u.reg_write(UC_X86_REG_EAX,value);return
  if a==0x53d500:self.finished.append([self.signed(self.r(self.rsp()+4)),self.signed(self.r(self.rsp()+8))])
  if a==0x53da99:self.popped.append(list(struct.unpack('<5I',u.mem_read(SOLVER+0x24,20))))
  if a==0x54f370:
   index=(u.reg_read(UC_X86_REG_ECX)-TILES)//0x30;width=self.r(GRID+0x1c);tile=TILES+index*0x30
   self.collision_trace.append([index%width,index//width,self.r(tile+0x28),u.mem_read(tile+0x2d,1)[0]])
 def read_path(self):
  result=[];ptr=self.r(PATH+4);previous=0
  while ptr:
   assert self.r(ptr+0x20)==previous and len(result)<1000
   result.append({'words':list(struct.unpack('<8I',self.u.mem_read(ptr,32)))});previous=ptr;ptr=self.r(ptr+0x24)
  assert self.r(PATH+8)==previous
  return {'nodes':result,'kind1_count':self.signed(self.r(PATH+12)),'kind2_count':self.signed(self.r(PATH+16)),'dirty':bool(self.u.mem_read(PATH+20,1)[0]),'word18':self.r(PATH+24)}
 def run(self,case):
  self.reset(0);self.w(0xb36534,WORLD);self.w(WORLD+0x30,GRID);self.w(0xb364c0,REGISTRY)
  width,height=case['size'];cell=case.get('cell_size',2.);shift=case.get('shift',1)
  self.w(GRID+4,shift);self.w(GRID+8,bits(cell));self.w(GRID+12,bits(width*cell));self.w(GRID+16,bits(height*cell));self.w(GRID+28,width);self.w(GRID+32,height);self.w(GRID+40,TILES);self.w(GRID+72,case.get('initial_epoch',0))
  self.u.mem_write(TILES,bytes(width*height*0x30));self.u.mem_write(SOLVER,bytes(0x200))
  for y in range(height):
   for x in range(width):self.w(TILES+(y*width+x)*0x30+32,1 if x==0 or y==0 or x==width-1 or y==height-1 else (2 if[x,y]in case.get('blocked',[]) else 0))
  strategy=SOLVER+0x44;queue=SOLVER+0x84
  self.w(SOLVER,0xa366a8);self.w(SOLVER+4,GRID);self.w(SOLVER+8,queue);self.w(SOLVER+12,strategy);self.w(strategy,0xa36610)
  self.w(DESC+0x3c,2);self.w(queue,0xa3668c);self.w(queue+4,HEAP);self.w(queue+12,16384);self.w(queue+16,case.get('queue_limit',0))
  cache=0x4004000;self.w(0xb39ab0,cache);self.w(cache+0x5c,cache+0x500);self.w(cache+0x500,0);self.w(0xb27ae0,0x4005000)
  center=lambda p:[v*cell+cell*.5 for v in p]
  start=case.get('start_world',center(case['start']));goal=case.get('goal_world',center(case['goal']))
  args=[PATH,*map(bits,start),*map(bits,goal),DESC,case.get('option_word',0)]
  for i,arg in enumerate(args):self.w(self.STACK+4+i*4,arg)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.reg_write(UC_X86_REG_ECX,SOLVER)
  self.u.emu_start(0x53e1f0,self.STOP,count=20000000)
  assert self.rsp()==self.STACK+32
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  return {'status':self.u.reg_read(UC_X86_REG_EAX),'path':self.read_path(),'strategy_words':list(struct.unpack('<15I',self.u.mem_read(strategy+4,60))),'finished':self.finished,'visit_epoch':self.r(GRID+72),'popped':self.popped,'collision_trace':self.collision_trace,'abi_preserved':True}
def main():
 source=json.loads((ROOT/'research/phase6-pathsearch/native-search-fixtures.json').read_text());fixtures=[];executed={}
 for row in source['cases']:
  if row['input']['name']=='subunit_cells':continue
  p=row['precision'];m=SolveMachine({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[p]);case=row['input']
  try:output=m.run(case)
  except Exception as e:raise RuntimeError(p+' '+case['name']+' at '+hex(m.u.reg_read(UC_X86_REG_EIP))) from e
  fixtures.append({'precision':p,'input':case,'output':output});executed.update(m.executed)
 ranges=[]
 for a,size in sorted(executed.items()):
  if ranges and ranges[-1][1]==a:ranges[-1][1]=a+size
  else:ranges.append([a,a+size])
 pins=[]
 for a,z in ranges:
  raw=original.range(a,z-a);assert raw==modified.range(a,z-a);pins.append({'start':hex(a),'end_exclusive':hex(z),'sha256':hashlib.sha256(raw).hexdigest()})
 doc={'schema':1,'original_sha256':hashlib.sha256(original.b).hexdigest(),'v14_sha256':hashlib.sha256(modified.b).hexdigest(),'oracle_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'case_count':len(fixtures),'seams':['node allocator/free only'],'region_pather_enabled':False,'fixtures':fixtures}
 (OUT/'solve-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n');(OUT/'solve-ranges.json').write_text(json.dumps(pins,indent=2)+'\n');print(json.dumps({'passed':True,'cases':len(fixtures)}))
if __name__=='__main__':main()
