"""Original53DC00 list/waypoint prepass with required topology/world leaves."""
from pathlib import Path
import copy,hashlib,json,struct,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase6-navigation'))
from materialize_oracle import Machine as PathMachine,PATH,STACK,Pe,ORIGINAL_SHA,ACCEPTED_SHA,bits
from unicorn.x86_const import *
WORLD,GRID,DESC,MANAGER,NODE_ARRAY,NODES,STOP=0x2003000,0x2004000,0x200A000,0x200B000,0x200C000,0x2030000,0x200D000
class Machine(PathMachine):
 def __init__(self,pe,c):
  native={'precision':c['precision'],'state':{'start_cell':[0,0],'goal_cell':[0,0],'start_bits':[0,0],'goal_bits':[0,0],'collision_word28':0},'grid':{'width':1,'height':1,'coordinate_shift':1,'cell_size_bits':bits(2),'predecessors':[0]},'path':c['path']}
  super().__init__(pe,native);self.u.mem_map(0,0x1000);self.case=c;self.u.mem_map(NODES,0x10000);self.put(0xb36534,WORLD);self.put(WORLD+0x30,GRID);self.put(0xb3652c,MANAGER);self.put(MANAGER+0x14,NODE_ARRAY)
  for i,xy in enumerate(c['centers']):self.put(NODE_ARRAY+i*4,NODES+i*0x100);self.put(NODES+i*0x100,i);self.put(NODES+i*0x100+0x54,*xy)
  self.lookup_index=0;self.block_index=0;self.line_index=0;self.events=[];self.region_allocations={};self.region_frees=[]
 def event(self,name,**payload):self.events.append({'name':name,**payload})
 def hook(self,u,a,size,_):
  if 0x400000<=a<0xc00000:self.executed[a]=size
  if a==0x7aa3d0:
   result=self.case['lookup_results'][min(self.lookup_index,len(self.case['lookup_results'])-1)];self.lookup_index+=1
   self.event('RegionAt',point=self.args(2),result=result);self.ret(8,NODES+result*0x100);return
  if a==0x53f320:
   out,start,goal,cached=self.args(4);assert cached==0;self.event('PlanRegions',start=start,goal=goal,result=self.case['plan_result'])
   ids=self.case['route'];ptrs=[self.alloc() for _ in ids];self.put(out,ptrs[0] if ptrs else 0,ptrs[-1] if ptrs else 0)
   for i,(ptr,value) in enumerate(zip(ptrs,ids)):
    self.put(ptr,value,ptrs[i-1] if i else 0,ptrs[i+1] if i+1<len(ptrs) else 0);self.region_allocations[ptr]=value
   self.ret(16,self.case['plan_result']);return
  if a==0x6f7430:
   ptr=self.args(1)[0];self.region_frees.append(self.region_allocations[ptr]);self.ret();return
  if a in (0x77dc50,0x77da70):self.ret(8 if a==0x77dc50 else 12);return
  if a==0x5586d0:self.event('BeginProbe');self.ret(4,u.reg_read(UC_X86_REG_ECX));return
  if a==0x4f3c40:
   actor,context=self.args(2);assert context==DESC
   result=self.case['blocked'][min(self.block_index,len(self.case['blocked'])-1)];self.block_index+=1
   self.event('BlockedHere',position=self.get(DESC+0x4c,2),result=result);self.ret(8,result);return
  if a==0x522dd0:
   startx,starty,goalx,goaly=self.args(4);result=self.case['line_clear'][min(self.line_index,len(self.case['line_clear'])-1)];self.line_index+=1
   self.event('LineClear',start=[startx,starty],goal=[goalx,goaly],result=result);self.ret(16,int(result));return
  if a==0x53d340:
   x,y,point,fx,fy,context=self.args(6);assert context==DESC;before=self.get(point,2)
   if self.case['adjusted'] is not None:self.put(point,*self.case['adjusted'])
   self.event('Adjust',start=[x,y],fallback=[fx,fy],before=before,after=self.get(point,2),result=self.case['adjust_success']);self.ret(value=int(self.case['adjust_success']));return
  super().hook(u,a,size,_)
 def run(self):
  self.put(STACK,STOP,PATH,*self.case['start'],*self.case['goal'],DESC);self.u.reg_write(UC_X86_REG_ESP,STACK)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x53dc00,STOP,count=200000)
  assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==STACK+28
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  assert sorted(self.region_frees)==sorted(self.case['route'])
  return {'path':self.read_path(),'result':self.u.reg_read(UC_X86_REG_EAX),'events':self.events,'region_frees':self.region_frees,'context_position':self.get(DESC+0x4c,2),'abi_preserved':True}
def cases():
 seed={'name':'base','precision':'Double53','start':list(map(bits,[1.125,1.75])),'goal':list(map(bits,[33.25,40.5])),'centers':[list(map(bits,[i*5.1+3.2,i*7.3+4.125])) for i in range(8)],'lookup_results':[0,7,2],'route':list(range(8)),'plan_result':1,'blocked':[0],'line_clear':[False],'adjust_success':True,'adjusted':None,'path':{'nodes':[],'kind1_count':0,'kind2_count':0,'dirty':False,'word18':123}}
 rows=[]
 def add(name,**parts):c=copy.deepcopy(seed);c.update(parts);c['name']=name;rows.append(c)
 for n in range(9):
  for blocked,clear,label in [([0],[False],'immediate'),([1],[False],'blocked'),([0],[True],'connected'),([1,0],[False],'one_block'),([0],[True,False],'one_clear')]:add('length_'+str(n)+'_'+label,route=list(range(n)),blocked=blocked,line_clear=clear)
 add('graph_failure',plan_result=0);add('graph_partial',plan_result=2);add('adjust_fail',adjust_success=False);add('adjust_shift',adjusted=list(map(bits,[13.75,14.875])));add('adjust_start',adjusted=seed['start'])
 for i in range(4):add('coincident_'+str(i),centers=[list(map(bits,[3.2,4.125])) if k>=i else v for k,v in enumerate(seed['centers'])])
 existing={'nodes':[{'words':[bits(50),bits(60),bits(1),0,1,0,bits(50),bits(60)]}],'kind1_count':1,'kind2_count':0,'dirty':False,'word18':123}
 add('existing_tail',path=existing);add('existing_tail_adjust',path=existing,adjusted=list(map(bits,[13.75,14.875])))
 return [{**copy.deepcopy(c),'name':c['name']+'_'+p,'precision':p} for p in ['Single24','Double53','Extended64'] for c in rows]
def main():
 pe=Pe(ROOT.parent/'AA.exe',ORIGINAL_SHA);v14=Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',ACCEPTED_SHA);rows=[];executed={}
 for c in cases():
  try:m=Machine(pe,c);out=m.run()
  except Exception as e:raise RuntimeError(c['name']) from e
  rows.append({'input':c,'output':out});executed.update(m.executed)
 for a,n in executed.items():assert pe.at(a,n)==v14.at(a,n)
 doc={'schema':1,'passed':True,'case_count':len(rows),'original_sha256':ORIGINAL_SHA,'v14_sha256':ACCEPTED_SHA,'oracle_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'seams':['region ID lookup','region graph result list','collision context probe and placement test','line connectivity','endpoint adjustment independently ported','node allocation/free','debug output'],'fixtures':rows}
 (OUT/'prepass-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n');(OUT/'prepass-ranges.json').write_text(json.dumps([{'address':hex(a),'bytes':pe.at(a,n).hex()} for a,n in sorted(executed.items())],indent=2)+'\n');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
