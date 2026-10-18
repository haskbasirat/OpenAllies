"""Native Body property refresh using real derived getters and one562200 node."""
from pathlib import Path
import copy,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import Machine as Base,IMAGE,ACCEPTED,pe,initial,environment,bits
from unicorn.x86_const import UC_X86_REG_EIP
MOD,MINIMUM=0x20b0000,0x20b1000
class Machine(Base):
 def __init__(self,c):
  self.c=c;self.visited={};super().__init__(c['body'],c['environment'],{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']],[])
  self.w(self.DATA+0x284,c['base']);self.w(self.DATA+0x288,c['base']);self.w(MOD+0x50,c['factor']);self.w(MOD+0x10,c['delta']);self.w(self.CACHE+0x1a4,MINIMUM);self.w(MINIMUM,c['minimum'])
 def hook(self,u,a,n,user):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  super().hook(u,a,n,user)
 def run(self):
  if self.c['method']=='maximum':self.call(0x561d40,[MOD if self.c['modifier'] else 0,0,int(self.c['preserve'])],12)
  elif self.c['method']=='defense':self.call(0x561790,[MOD if self.c['modifier'] else 0],4)
  else:self.call(0x562200,[self.c['health'],self.c['base']],8)
  result={'body':self.state(),'environment':self.env,'events':self.events}
  if self.c['method']=='pair':result['pair']={'health':self.r(self.STACK+4),'maximum':self.r(self.STACK+8)}
  return result
def cases():
 rows=[]
 base={'body':initial(),'environment':environment(),'base':bits(100),'factor':bits(1.05),'delta':bits(2),'minimum':bits(1),'modifier':True,'preserve':True,'health':bits(25)}
 def add(name,method,fn=None):
  c=copy.deepcopy(base);c.update(name=name,method=method)
  if fn:fn(c)
  rows.append(c)
 for health,maximum,new,factor in [(75,100,100,1.05),(1,3,7,1.1),(2,7,99,.3),(0,100,100,1.2),(.1,1,.25,1.1),(99,100,70,.3333333),(10000001,10000003,10000019,.10000001)]:
  for preserve in [False,True]:add(f'max_{health}_{maximum}_{new}_{factor}_{preserve}','maximum',lambda c,h=health,o=maximum,n=new,f=factor,p=preserve:c.update(body=initial(h,o),base=bits(n),factor=bits(f),preserve=p))
 add('maximum_no_context','maximum',lambda c:c.update(modifier=False))
 for value,delta,minimum in [(5,2,1),(5,-10,1),(0,-1,2),(-1,2,0),(.1,.2,.05),(100,1.05,2)]:add(f'defense_{value}_{delta}_{minimum}','defense',lambda c,v=value,d=delta,m=minimum:c.update(base=bits(v),delta=bits(d),minimum=bits(m)))
 add('defense_no_context','defense',lambda c:c.update(modifier=False,base=bits(-1)))
 for health,maximum in [(25,50),(100,50),(-1,50),(0,0),(1,.00001),(25,200)]:add(f'pair_{health}_{maximum}','pair',lambda c,h=health,m=maximum:c.update(health=bits(h),base=bits(m)))
 add('pair_health_effects','pair',lambda c:c.update(environment=environment(health_rules=[{'kind':0,'threshold':bits(.5)}]),health=bits(10),base=bits(50)))
 for p in ['Single24','Double53','Extended64']:
  for row in rows:yield {**copy.deepcopy(row),'name':row['name']+'_'+p,'precision':p}
def main():
 rows=[];visited={}
 for c in cases():
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=IMAGE.at(a,n);assert raw==ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'visited_windows':windows,'seams':['Body health setter actor/effect callbacks from original_body.py; maximum/defense derived getters execute unchanged'],'scope':'No linked graph is supplied; each pair case is the actual complete one-node562200 path.'}
 path=HERE/'native-body-updates.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
