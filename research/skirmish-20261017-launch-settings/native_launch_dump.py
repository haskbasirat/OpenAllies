"""Actual76D6C0 traversal with every constructor-nonnull child and actual
59F010/5128F0/5118A0/50E820/505EA0/784560 descendants. Raw explicit
fixture storage, definitions and CString transports; typed logger leaf calls
are observations, not executed logger bodies. Native instructions/order and
all argument labels/values are pinned independently of enabled SyncA state.
"""
from pathlib import Path
import sys,json,hashlib,itertools,random,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-mission-start'))
from native_creator_constructor import Machine,h,rich_launch
class Stop(Exception):pass
class DumpMachine(Machine):
 def __init__(self,image,pc,stop=None):
  super().__init__(image,pc,dict(width=192,height=192,independent=0,resource_count=3));self.events=[];self.cutoff=stop
  for a,kind,n in [(0x77da70,'int',12),(0x77d930,'float',12),(0x77d9d0,'bool',12),(0x77db10,'text',12),(0x77dc50,'marker',8),(0x40f8d0,'text',0)]:
   self.seams[a]=lambda a=a,kind=kind,n=n:self.logger(kind,n)
 def logger(self,kind,pop):
  if self.cutoff==len(self.events):raise Stop()
  label=self.text(self.arg(0));v=self.arg(1) if kind!='marker'else 0
  if kind=='text':
   at=self.alloc(4);self.w(at,v);v=self.rs(at)
  elif kind=='bool':v=(v&255)!=0
  self.events.append(dict(kind=kind,label=label,value=v));self.ret(pop)
 def run(self,case):
  p=self.alloc(0x34);self.build_launch(p,case)
  #build_launch supplies full positive child pointers, but its bonus words
  #belong to the network projection; set their explicit current raw storage.
  for child,params in [(self.r(p+0x14),case['game'])]+[(self.r(p+o),case[k])for o,k in [(0x18,'map_default'),(0x1c,'map_scenario'),(0x20,'map_random'),(0x24,'map_24'),(0x28,'map_metagame')]]:
   header=self.r(child+8) if params is case['game']else child+4
   for i,row in enumerate(params['header']['kingdoms']):
    at=self.r(header+0x10)+i*0x6c;values=row.get('bonus',[]);ptr=self.alloc(max(1,len(values))*4)
    self.w(at+0x34,ptr);self.w(at+0x38,len(values));self.w(at+0x3c,len(values))
    for j,v in enumerate(values):self.w(ptr+j*4,v)
  label=self.string('supplied_but_not_a_marker');stopped=False
  try:self.call(0x76d6c0,[label,p])
  except Stop:stopped=True
  except Exception:
   print('FAULT',hex(self.u.reg_read(__import__('unicorn.x86_const',fromlist=['UC_X86_REG_EIP']).UC_X86_REG_EIP)),self.events[-10:],flush=True);raise
  return dict(events=self.events,stopped=stopped)

def run(case,image,pc,stop):
 m=DumpMachine(image,pc,stop);out=m.run(case);return out,m.visited,m.intercepted

def main():
 cases=[]
 for i in range(4):
  c=rich_launch(random.Random(0x76d6c0+i));c['game']['version']=[0,1,2,7][i]
  c['game']['pair_20']['second']=[65,0,66];c['game']['pair_28']['second']=[0x03a9]
  for k in ['game','map_default','map_scenario','map_random','map_24','map_metagame']:
   for row in c[k]['header']['kingdoms']:row['bonus']=[0x3f800000,0x80000000,0x7fc01234]
  cases.append(c)
 rows=[];visited={};seams=set()
 for pc,i in itertools.product(h.PRECISION,range(len(cases))):
  full,_,_=run(cases[i],'original',pc,None)
  for stop in [None,0,1,3,10,len(full['events'])//2,len(full['events'])-1]:
   a,pa,sa=run(cases[i],'original',pc,stop);b,pb,sb=run(cases[i],'v14',pc,stop);assert a==b
   if stop is not None:assert a['events']==full['events'][:stop]and a['stopped']
   visited.update(pa);visited.update(pb);seams|=sa|sb;rows.append(dict(precision=pc,input=cases[i],stop=stop,output=a))
 pins=[]
 for va,n in sorted(visited.items()):
  if va in seams or not 0x401000<=va<0x9ef000:continue
  raw=h.P.range(va,n);assert raw==h.V.range(va,n);pins.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(schema=1,scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,executions=len(rows)*2,instructions=pins,seams=[hex(a)for a in sorted(seams)])
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-launch-dump.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()



