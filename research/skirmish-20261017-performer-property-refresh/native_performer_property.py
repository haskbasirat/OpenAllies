"""Native57F1E0 reached prefix on canonical enabled bytes and exact native
4BE6B0/581280/5032D0 lookup. Actual actions4D1350 and CV57E430 are required
stops, never successful seams. NULL Properties stops at actual required read.
Definitions/list/instance cells are explicit isolated fixture inputs, not a
source-loaded Property allocation claim. Disabled instances satisfy lookup.
"""
from pathlib import Path
import argparse,hashlib,json,itertools,sys
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-performer-value'))
import native_actual_performer as base
X=base.X;h=base.h
STOPS={0x4D1350:'4D1350 actions',0x57E430:'57E430 value',0x5032D6:'581280 NULL Properties read'}
class Machine(base.Actual):
 def _code(self,uc,pc,size,_):
  if pc in STOPS and(pc!=0x5032D6 or self.reg(X.UC_X86_REG_EAX)==0x10):raise RuntimeError('required '+STOPS[pc])
  super()._code(uc,pc,size,_)
def run(image,pc,c):
 m=Machine(image,pc);m.u.mem_unmap(0,0x1000)
 ids=[v for a in c['abilities']for v in(a['required']or[])]+[p['definition']for p in(c['properties']or[])]
 definitions=[m.alloc(0x40)for _ in range(max(3,max(ids,default=0)+1))]
 actor=m.alloc(0x180)
 if c['properties']is not None:
  props=m.alloc(0x80);arr=m.alloc(4*max(1,len(c['properties'])))
  for i,p in enumerate(c['properties']):
   instance=m.alloc(0x20);m.w(instance+0xC,definitions[p['definition']]);m.wb(instance+0x10,p['disabled']);m.w(instance+0x14,p['source']);m.w(arr+4*i,instance)
  m.w(props+0x10,arr);m.w(props+0x14,len(c['properties']));m.w(actor+0x78,props)
 performer=m.alloc(0x80);m.w(performer+4,actor);arr=m.alloc(4*max(1,len(c['abilities'])));abilities=[]
 for i,a in enumerate(c['abilities']):
  ability=m.alloc(0x20);abilities.append(ability);m.wb(ability+0x18,a['enabled']);m.w(arr+4*i,ability)
  if a['required']is not None:
   local=m.alloc(0x70);head=0
   for value in reversed(a['required']):
    node=m.alloc(8);m.w(node,definitions[value]);m.w(node+4,head);head=node
   m.w(local+0x58,head);m.w(ability+8,local)
 m.w(performer+0x14,arr);m.w(performer+0x18,len(abilities));stop=None
 try:m.call(0x57F1E0,ecx=performer)
 except RuntimeError:
  stop=m.reg(X.UC_X86_REG_EIP)
  if stop not in STOPS:raise
 assert stop is not None and not m.seam_hits
 return dict(enabled=[m.rb(a+0x18)for a in abilities],required=STOPS[stop]),m.visited,stop

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--write',action='store_true');args=ap.parse_args()
 properties=[None,[],[dict(definition=0,disabled=False,source=0)],[dict(definition=0,disabled=True,source=0xDEADBEEF)],
  [dict(definition=0,disabled=True,source=0),dict(definition=1,disabled=False,source=0xA0001)],
  [dict(definition=2,disabled=False,source=0),dict(definition=0,disabled=False,source=0)]]
 tables=[[]]
 for required,enabled in itertools.product([None,[],[0],[1],[0,1],[1,0],[0,0]], [False,True]):tables.append([dict(required=required,enabled=enabled)])
 tables.extend([[dict(required=[0],enabled=False),dict(required=[1],enabled=True)],
  [dict(required=[1,0],enabled=True),dict(required=[0],enabled=False),dict(required=None,enabled=True)]])
 rows=[];pins={};boundaries={}
 for pc,props,table in itertools.product(h.PRECISIONS,properties,tables):
  c=dict(properties=props,abilities=table);a,pa,s=run('original',pc,c);b,pb,t=run('v14',pc,c);assert a==b and pa==pb and s==t
  rows.append(dict(precision=pc,input=c,output=a));pins.update(pa)
  assert h.ORIGINAL.at(s,8)==h.ACCEPTED.at(s,8);boundaries[f'{s:08x}']=h.ORIGINAL.at(s,8).hex()
 out=dict(schema=1,scope=__doc__,paired_cases=len(rows),native_executions=2*len(rows),cases=rows,successful_seams=[],executed_instruction_pins={f'{a:08x}':b for a,b in sorted(pins.items())},stopped_boundaries=boundaries,
  source_images=[dict(name=n,sha256=hashlib.sha256(p.data).hexdigest())for n,p in [('original',h.ORIGINAL),('v14',h.ACCEPTED)]])
 path=HERE/'native-performer-property.json'
 if args.write:
  with path.open('x')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_bytes())==out
 print(json.dumps(dict(paired_cases=len(rows),native_executions=2*len(rows),executed_pins=len(pins),required_boundaries=len(boundaries),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
