"""581F70/5827B0 eligibility, real4E65E0/hash queries/Single callbacks.

Inputs are controlled native field laboratories. Requirements4C5250 is the
declared child transport; it writes the caller's real result byte. All other
decision routines, owner getters, recruit membership and Single consume/restore
bodies execute unchanged in each original/v14 machine at all three x87 PCs.
No diagnostic destination, current gameplay capture or activation is claimed.
"""
from pathlib import Path
import sys,json,hashlib,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE,UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_ECX,UC_X86_REG_EBX
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stop(Exception):pass

def run(c,image,pc,stop=None):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc]);events=[]
 a=m.alloc(0x200);own=m.alloc(0x600);r=m.alloc(0x40);k=m.alloc(0x400)
 m.w(a,0xa273f0);m.w(a+4,own);m.w(a+0xe8,k if c['owner']else 0);m.w(a+0x98,a+0x180);m.w(r+4,a)
 defs=[m.alloc(0x600)for _ in c['definitions']];reqs=[m.alloc(0x20)for _ in defs];layouts=[]
 for item in c['layouts']:
  layout=m.alloc(0x20);data=m.alloc(0x30);params=m.alloc(4*len(item['members']));entries=m.alloc(16*len(item['members']))
  m.w(layout+8,data);m.w(layout+0xc,params);m.w(data+0x18,item['primary']);m.w(data+0x1c,entries);m.w(data+0x20,len(item['members']))
  for i,(member,number)in enumerate(zip(item['members'],item['numbers'])):
   m.w(params+4*i,defs[member]if member is not None else 0);m.w(entries+16*i+8,number)
  layouts.append(layout)
 for i,d in enumerate(c['definitions']):
  m.w(defs[i]+0x174,d['mask']);m.w(defs[i]+0x178,d['class']);m.w(defs[i]+0x2c8,reqs[i]);m.w(defs[i]+0x2f0,layouts[d['layout']]if d['layout']is not None else 0)
  m.u.mem_write(defs[i]+0x1b3,bytes([int(d['strict'])]))
 #Real recruit list cells; native573AC0 follows these exact nodes.
 head=0
 for i in reversed(c['own_list']):
  node=m.alloc(8);m.w(node,defs[i]);m.w(node+4,head);head=node
 m.w(own+0x4ec,head)
 table=k+0x1fc;buckets=m.alloc(20);m.w(table+8,buckets);m.w(table+0xc,5);items=[]
 for i,item in enumerate(c['items']):
  p=m.alloc(0x24);m.w(p,0xa29694);m.w(p+4,defs[item['definition']]);m.u.mem_write(p+8,bytes([int(item['available'])]));items.append(p)
  node=m.alloc(12);b=defs[item['definition']]%5;m.w(node,defs[item['definition']]);m.w(node+4,p);m.w(node+8,m.r(buckets+4*b));m.w(buckets+4*b,node)
 def event(kind,value):
  if stop==len(events):raise Stop(str(stop))
  events.append([kind,value])
 def requirements():
  i=reqs.index(m.u.reg_read(UC_X86_REG_ECX));event('requirements',i)
  assert m.arg(0)==(k if c['owner']else 0)and m.arg(1)==a and m.arg(2)==a and m.arg(6)==0
  m.u.mem_write(m.arg(3),bytes([int(c['definitions'][i]['properties_met'])]));m.ret(28,int(c['definitions'][i]['met']))
 m.seam(0x4c5250,requirements)
 def before(u,at,size,_):
  if at==0x4e0ea0:event('single_consume',[items.index(u.reg_read(UC_X86_REG_ECX)),m.arg(0)])
  elif at==0x4e0ec0:event('single_restore',[items.index(u.reg_read(UC_X86_REG_ECX)),m.arg(0)])
  elif at==0x581fcf:event('own_list',bool(head))
  elif at==0x573ac0:event('membership',defs.index(m.arg(0)))
 m.u.hook_add(UC_HOOK_CODE,before)
 result=None;stopped=None
 try:
  target=defs[c['target']]if c['target']is not None else 0;layout=layouts[c['explicit_layout']]if c['explicit_layout']is not None else 0
  result=m.call(0x5827b0 if c['wrapper']else 0x581f70,r,[target,layout,c['check'],0]if c['wrapper']else[target,layout,c['check'],m.alloc(8),m.alloc(8)])
 except Stop as e:stopped=str(e)
 return dict(result=result,stopped=stopped,events=events,available=[m.u.mem_read(p+8,1)[0]for p in items]),m.windows()

def cases():
 d=lambda **k:dict(dict(mask=0,**{'class':1},layout=None,strict=False,met=True,properties_met=True),**k)
 base=dict(definitions=[d(),d(),d()],layouts=[],target=0,explicit_layout=None,check=1,owner=False,own_list=[],items=[],wrapper=True)
 yield dict(base,target=None)
 for wrapper in(False,True):
  for owner in(False,True):
   for cls in(0,1,2,3,0xffffffff):
    for available in(None,False,True):
     yield dict(base,wrapper=wrapper,owner=owner,definitions=[d(**{'class':cls}),d(),d()],items=[]if available is None else[dict(definition=0,available=available)])
 for met,props,strict in((True,False,True),(False,False,False),(False,False,True),(False,True,True)):
  yield dict(base,definitions=[d(met=met,properties_met=props,strict=strict),d(),d()])
 for check in(0,1,128):
  for members in([], [0],[1]):yield dict(base,check=check,own_list=members)
 for primary in(-1,0):
  for wanted in(-2,0,1,2):
   for available in(False,True):
    org=d(mask=128,**{'class':2},layout=0)
    yield dict(base,owner=True,definitions=[org,d(),d()],layouts=[dict(primary=primary,members=[None,1],numbers=[7,wanted])],items=[dict(definition=1,available=available)])
 #Recursive last-nonzero result, including 2 then1 (must not short circuit).
 for members in([None],[],[1,2],[2,1]):
  for explicit in(False,True):
   defs=[d(mask=128 if not explicit else 0,layout=0),d(met=False,properties_met=False,strict=True),d(met=False,properties_met=True)]
   yield dict(base,definitions=defs,explicit_layout=0 if explicit else None,layouts=[dict(primary=0,members=members,numbers=[1]*len(members))])

def main():
 rows=[];pins={}
 for i,c in enumerate(cases()):
  for pc in PC:
   original,w1=run(c,'original',pc);expanded,w2=run(c,'v14',pc);assert original==expanded,(i,pc,original,expanded)
   for p in w1+w2:pins[p['address']]=p['sha256']
   rows.append(dict(case=i,pc=pc,input=c,output=original))
   #Stop before each declared observable child, compare exact prior writes.
   if i in(0,11,32,78,90):
    for cut in range(len(original['events'])):
     a,p1=run(c,'original',pc,cut);b,p2=run(c,'v14',pc,cut);assert a==b
     for p in p1+p2:pins[p['address']]=p['sha256']
     rows.append(dict(case=i,pc=pc,input=c,stop=cut,output=a))
 payload=dict(scope=__doc__,rows=rows,basic_block_windows=[dict(address=a,sha256=h)for a,h in sorted(pins.items())])
 path=HERE/'native-eligibility.json'
 if '--write'in sys.argv:path.write_text(json.dumps(payload,indent=2)+'\n')
 else:assert json.loads(path.read_text())==payload,'immutable fixture differs'
 print('PASS',len(rows),'rows',len(rows)*2,'separate executions',len(pins),'basic-block byte windows',hashlib.sha256(path.read_bytes()).hexdigest())
if __name__=='__main__':main()
