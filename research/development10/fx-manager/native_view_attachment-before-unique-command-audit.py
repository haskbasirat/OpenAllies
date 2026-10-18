"""Actual50CAB0 attachment/drain including50C510 and45F240 list kernels.

Heap allocation/free are transport seams.43B870 delivery and428350 command
destruction are named child seams, or BEFORE-entry required cutoffs. Commands
are opaque laboratory allocation identities, never source counts or compiled
FX substitutes. Error rows prove node removal before the required child.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ECX
class RequiredChild(Exception):pass
class Attachment(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'attachment_ready',False):
   if pc in (0x43b870,0x428350):
    label='deliver'if pc==0x43b870 else'destroy'
    value=self.arg()if label=='deliver'else self.u.reg_read(UC_X86_REG_ECX)
    self.calls.append([label,self.labels.get(value,0)])
    assert label!='deliver'or self.u.reg_read(UC_X86_REG_ECX)==self.view
    self.children+=1
    if self.fail_at==self.children:raise RequiredChild()
    self.ret(4 if label=='deliver'else 0);return
   if pc==0x6f7430:
    pointer=self.arg();self.freed.append(self.nodes.get(pointer,self.labels.get(pointer,'unknown')))
    assert pointer in self.nodes or pointer in self.labels,(hex(pointer),self.nodes,self.labels,self.calls)
    self.ret(0);return
  super().hook(u,pc,size,user)
def pending(m):
 values=[];node=m.r(m.manager+8);seen=set();tail=0
 while node:
  assert node not in seen;seen.add(node);values.append(m.labels.get(m.r(node),0));tail=node;node=m.r(node+4)
 assert tail==m.r(m.manager+12)
 return values
def run(m,pc,c):
 m.fpu(pc);m.actual={};m.record_actual=True;m.attachment_ready=False
 m.manager=m.alloc(0x20);m.view=m.alloc(4);m.w(m.manager+4,0x11111111)
 m.u.mem_write(m.manager,bytes([int(c['muted'])]));m.labels={};m.nodes={};m.calls=[];m.freed=[];m.children=0;m.fail_at=c['fail_at']
 values={v:m.alloc(4)for v in c['pending']if v};m.labels={p:v for v,p in values.items()};prior=0
 for index,value in enumerate(c['pending']):
  node=m.alloc(8);m.w(node,values.get(value,0));m.w(node+4,0);m.nodes[node]='node'+str(index)
  if prior:m.w(prior+4,node)
  else:m.w(m.manager+8,node)
  prior=node
 m.w(m.manager+12,prior);m.attachment_ready=True;error=None
 try:m.call(0x50cab0,[m.view if c['view']else 0],this=m.manager,callee_pop=4)
 except RequiredChild:error='required '+m.calls[-1][0]
 output=dict(attached=bool(m.r(m.manager+4)),pending=pending(m),calls=m.calls,freed=m.freed,error=error)
 return output,m.actual
def main():
 rows=[];pins={};cases=[]
 for muted,view,order in itertools.product([False,True],[False,True],[[],[11],[11,0,12],[0,0],[11,12,11]]):
  #Native unmuted/null/nonempty continually revisits reappended nodes. No
  #successful completion or artificial iteration-bound parity is claimed.
  if not muted and not view and order:continue
  cases.append(dict(muted=muted,view=view,pending=order,fail_at=None))
 for muted,view,order,fail in [(False,True,[11,0,12],1),(False,True,[11,0,12],2),(True,True,[11,0,12],1),(True,False,[11,0,12],2)]:
  cases.append(dict(muted=muted,view=view,pending=order,fail_at=fail))
 for image in ['original','v14']:
  m=Attachment(image)
  for pc in PC:
   for c in cases:
    out,actual=run(m,pc,c);rows.append(dict(image=image,precision=pc,input=c,output=out));pins.update(actual)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']
 b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 out=HERE/'native-view-attachment.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
