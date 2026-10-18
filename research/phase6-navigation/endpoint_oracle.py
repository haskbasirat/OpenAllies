"""Run original endpoint adjustment and clamp; collision leaves are fixtures."""
from pathlib import Path
import copy,hashlib,json,random,struct,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase3-terrain'))
from terrain_uv_oracle import Pe,ORIGINAL_SHA,ACCEPTED_SHA
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
WORLD,GRID,DEST,DESC,STOP,STACK=0x2000000,0x2001000,0x2002000,0x2003000,0x2004000,0x3008000
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
class Machine:
 def __init__(self,pe,c):
  self.c=c;self.u=Uc(UC_ARCH_X86,UC_MODE_32);pe.load(self.u);self.u.mem_map(0x2000000,0x10000);self.u.mem_map(0x3000000,0x10000)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.put(0xb36534,WORLD);self.put(WORLD+0x30,GRID)
  self.put(GRID+8,c['bounds']['cell_size_bits'],*c['bounds']['extent_bits']);self.put(DEST,*c['destination']);self.events=[];self.accept_index=0;self.displace_index=0;self.executed={};self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,a,*v):self.u.mem_write(a,struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v)))
 def get(self,a,n=1):return list(struct.unpack('<'+'I'*n,self.u.mem_read(a,n*4)))
 def args(self,n):return self.get(self.u.reg_read(UC_X86_REG_ESP)+4,n)
 def ret(self,value):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.get(sp)[0]);self.u.reg_write(UC_X86_REG_ESP,sp+4);self.u.reg_write(UC_X86_REG_EAX,int(value))
 def hook(self,u,a,size,_):
  if 0x400000<=a<0xc00000:self.executed[a]=size
  if a==0x53ce40:
   start,candidate,_=self.args(3);result=self.c['accept_results'][min(self.accept_index,len(self.c['accept_results'])-1)];self.accept_index+=1
   self.events.append({'name':'Acceptable','start':self.get(start,2),'candidate':self.get(candidate,2),'result':result});self.ret(result)
  elif a==0x53ca30:
   x,y,ptr,_=self.args(4);response=self.c['displacements'][min(self.displace_index,len(self.c['displacements'])-1)];self.displace_index+=1;before=self.get(ptr,2)
   if response['position'] is not None:self.put(ptr,*response['position'])
   self.events.append({'name':'Displace','fallback':[x,y],'before':before,'after':self.get(ptr,2),'result':response['result']});self.ret(response['result'])
 def run(self):
  self.put(STACK,STOP,*self.c['start'],DEST,*self.c['fallback'],DESC);self.u.reg_write(UC_X86_REG_ESP,STACK)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x53d340,STOP,count=200000)
  assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==STACK+4
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  return {'destination':self.get(DEST,2),'accepted':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'events':self.events,'abi_preserved':True}
def cases():
 base={'name':'base','precision':'Double53','start':list(map(bits,[4,4])),'destination':list(map(bits,[20,25])),'fallback':list(map(bits,[4,4])),'bounds':{'cell_size_bits':bits(4),'extent_bits':list(map(bits,[64,64]))},'accept_results':[True],'displacements':[{'result':False,'position':None}]}
 rows=[]
 def add(name,**parts):c=copy.deepcopy(base);c.update(parts);c['name']=name;rows.append(c)
 add('first_accept');add('always_reject',accept_results=[False]);add('retry_accept',accept_results=[False,False,True]);add('displace_accept',accept_results=[False,True],displacements=[{'result':True,'position':list(map(bits,[21.25,22.5]))}]);add('displace_rejected_then_next',accept_results=[False,False,True],displacements=[{'result':True,'position':list(map(bits,[21.25,22.5]))}])
 for xy in [(0,0),(-10,-10),(100,100),(4,4),(5,4),(4,5),(4.0001,4),(4.0002,4),(4.0003,4),(4.1,4.1),(60,60),(4.125,59.875)]:
  for results in [[True],[False],[False,True]]:add('point_'+str(xy)+'_'+str(results),destination=list(map(bits,xy)),accept_results=results)
 for size in [1.25,2.75,3.7,8,16]:add('cell_'+str(size),bounds={'cell_size_bits':bits(size),'extent_bits':list(map(bits,[64.7,80.25]))},accept_results=[False,False,True])
 rng=random.Random(53340)
 for i in range(40):
  start=[rng.uniform(5,20),rng.uniform(5,20)];dest=[rng.uniform(-10,80),rng.uniform(-10,80)]
  add('fractional_'+str(i),start=list(map(bits,start)),fallback=list(map(bits,start)),destination=list(map(bits,dest)),accept_results=[False]*(i%5)+[True])
 return [{**copy.deepcopy(row),'name':row['name']+'_'+p,'precision':p} for p in ['Single24','Double53','Extended64'] for row in rows]
def main():
 pe=Pe(ROOT.parent/'AA.exe',ORIGINAL_SHA);v14=Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',ACCEPTED_SHA);rows=[];executed={}
 for c in cases():
  try:m=Machine(pe,c);output=m.run()
  except Exception as e:raise RuntimeError(c['name']) from e
  rows.append({'input':c,'output':output});executed.update(m.executed)
 pins=[]
 for a,size in sorted(executed.items()):
  raw=pe.at(a,size);assert raw==v14.at(a,size)
  pins.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'passed':True,'original_sha256':ORIGINAL_SHA,'v14_sha256':ACCEPTED_SHA,'oracle_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'case_count':len(rows),'seams':['53CE40 acceptance predicate','53CA30 obstacle displacement'],'fixtures':rows}
 (OUT/'endpoint-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n');(OUT/'endpoint-ranges.json').write_text(json.dumps(pins,indent=2)+'\n');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
