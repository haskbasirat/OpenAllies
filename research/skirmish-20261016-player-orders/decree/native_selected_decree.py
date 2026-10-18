"""Original61F210 selected-order coordinator with real first Actor B4/+108 virtuals. Native CString/clock/log sinks and4ED360 gameplay body are explicit fixture leaves. Required stops preserve empty/nonempty logging-before-delivery and default-source boundary; no formation or gameplay body supplied."""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase5-orders'));import native_commands as n
import original_orders as base
sys.path.insert(0,str(ROOT/'research/phase8-restoration'));import null_guard
from unicorn.x86_const import *
P=n.PE0;V=n.PE1;H=lambda b:hashlib.sha256(b).hexdigest();CW={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stop(Exception):pass
class Machine(n.Machine):
 def __init__(self):
  self.active=False;super().__init__();null_guard.install(self)
 def hook(self,u,pc,size,user):
  if self.active:
   if pc==0x404f90:
    self.events_native.append('diagnostic')
    if self.stop_at=='diagnostic':raise Stop('diagnostic')
    self.ret();return
   if pc==0x4ed360:
    self.events_native.append('delivery');assert u.reg_read(UC_X86_REG_ECX)==self.view and self.r(self.arg())==self.command_native
    if self.stop_at=='delivery':raise Stop('delivery')
    self.ret(4);return
   if pc==0x4da570:
    self.events_native.append('default');raise Stop('default')
  super().hook(u,pc,size,user)
 def run(self,c,pc):
  self.u.reg_write(UC_X86_REG_FPCW,CW[pc]);player=self.player(17,False,self.client);self.w(self.admin+0x284,player);self.view=self.alloc(0x40);self.w(player+0x50,self.view)
  self.w(self.actor,0xa273f0);self.w(self.actor+0xe8,0)
  if not c['empty']:
   node=self.alloc(8);self.w(node,self.actor);self.w(self.view+4,node);self.w(self.view+8,node)
  self.command_native=self.command(0x5c if c['explicit']else 0x1c);order=self.alloc(8);self.w(order+4,self.command_native);self.events_native=[];self.stop_at=c['stop'];self.active=True
  status='complete'
  try:self.call(0x61f210,this=order);assert (self.u.reg_read(UC_X86_REG_FPSW)>>11)&7==0
  except RuntimeError as e:
   if isinstance(e.__cause__,Stop):status=str(e.__cause__)
   else:raise
  assert not self.null_accesses,self.null_accesses
  return {'status':status,'events':self.events_native}
def main():
 cases=[dict(empty=e,explicit=x,stop=s)for e,x,s in itertools.product([False,True],[False,True],[None,'diagnostic','delivery'])];rows=[];visited={}
 for image,label in [(P,'original'),(V,'v14')]:
  base.PE0=image
  for pc in CW:
   for c in cases:
    m=Machine();rows.append({'image':label,'precision':pc,'input':c,'output':m.run(c,pc)});visited.update(m.visited)
 base.PE0=P;pins=[]
 for va,size in sorted(visited.items()):
  if P.base<=va<P.base+P.image_size:
   raw=P.range(va,size);assert raw==V.range(va,size),hex(va);pins.append({'va':hex(va),'bytes':raw.hex()})
 p=HERE/'native-selected-decree.json';p.open('x').write(json.dumps({'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'rows':rows,'instructions':pins,'instruction_scope':'Pinned intercepted seam entries are not execution of their native bodies.'},indent=1)+'\n');print(json.dumps({'rows':len(rows),'pins':len(pins),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
