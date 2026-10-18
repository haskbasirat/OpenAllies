"""Actual43FEE5..43FF36 clear/54BDE0/540A60/546840/542A00/request loop.
427BD0 canonical lookup is a declared immutable typed response seam. VFX
vtable48 and14 bodies execute (real constructor vtable identities separately
proved by native-vfx-lookup). Lab resources40 are null, not source-null proof.
The AD-store entry is an observed cutoff and does not execute.
"""
from pathlib import Path
import sys,json,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn.x86_const import *
ORIGINAL,V14=h.P,h.V
VT={0:0xa20770,5:0xa20800,12:0xa20ad0,10:0xa10e10}
class Machine(h.Machine):
 def __init__(self,pc,c):
  super().__init__(b'',pc);self.c=c;self.events=[];self.names={0:None};self.outcome='running'
  self.view=self.alloc(0x300);self.data=self.alloc(0x100);self.w(0xb36530,self.data)
  self.things=self.alloc(16);self.w(self.data+0xc,self.things)
  self.objects={}
  for name,kind in[('one',5),('two',12),('not_vfx',0),('model',10)]:
   p=self.alloc(0x100);self.objects[name]=p;self.names[p]=name;self.w(p,VT[kind]);self.w(p+0x20,0xfffffffe)
  manager=self.alloc(0x100);self.w(0xb36520,manager)
  head=0
  for actions in reversed(c['triggers']):
   trigger=self.alloc(0x60);self.w(trigger+0x24,self.action_nodes(actions))
   n=self.alloc(8);self.w(n,trigger);self.w(n+4,head);head=n
  self.w(manager+4,head)
  old=0
  for i in range(c['old_nodes']):n=self.alloc(8);self.w(n,0xf000+i);self.w(n+4,old);old=n
  self.w(self.view+0xb0,old);self.w(self.view+0xb4,old)
  self.seam(0x427bd0,self.lookup)
  self.probes[0x43ff36]=lambda m:m.stop('complete')
  self.probes[0x494380]=lambda m:m.events.append('request:'+m.names[m.u.reg_read(UC_X86_REG_ECX)])
  self.probes[0x494480]=lambda m:m.events.append('request:'+m.names[m.u.reg_read(UC_X86_REG_ECX)])
 def action_nodes(self,actions):
  head=0
  for action in reversed(actions):
   element=self.alloc(0x30);descriptor=self.alloc(0x30);self.w(element,descriptor)
   opcode=action['opcode'];self.w(descriptor+0x20,opcode);self.w(descriptor+0x24,2 if 82<=opcode<=84 else 1)
   if 82<=opcode<=84:
    parameter=self.alloc(0x40);self.w(parameter+0x1c,self.cstring(action['ids']));self.w(element+8,parameter)
   else:self.w(element+8,0) #Prove parameter loads are genuinely skipped.
   n=self.alloc(8);self.w(n,element);self.w(n+4,head);head=n
  return head
 def stop(self,name):self.outcome=name;raise h.Throw()
 def lookup(self,m):
  assert m.u.reg_read(UC_X86_REG_ECX)==self.things
  query=m.cstring_text(m.arg(0));ordinal=sum(e.startswith('lookup:')for e in m.events)
  if ordinal==self.c['stop_lookup']:self.stop('error:lookup')
  self.events.append('lookup:'+query);m.ret(4,self.objects.get(query,0))
 def run(self):
  self.u.reg_write(UC_X86_REG_ESI,self.view);self.u.reg_write(UC_X86_REG_EBX,0);self.call(0x43fee5,0,[])
  assert not self.null_accesses,self.null_accesses
  values=[];head=self.r(self.view+0xb0)
  while head:values.append(self.names[self.r(head)]);head=self.r(head+4)
  return dict(outcome=self.outcome,events=self.events,list=values,counts={n:self.r(p+0x20)for n,p in self.objects.items()},ad=self.b(self.view+0xad))
def main():
 one=lambda op,ids:dict(opcode=op,ids=ids)
 graphs=[[],[[]],[[one(0,'ignored')]],[[one(82,'one'),one(83,'missing'),one(84,'not_vfx'),one(82,'one')],[one(84,'two'),one(1,'ignored')]],[[one(82,'model'),one(83,'one')]]]
 cases=[dict(triggers=g,old_nodes=old,stop_lookup=None)for g,old in itertools.product(graphs,[0,3])]
 base=dict(triggers=graphs[3],old_nodes=3)
 for i in range(5):cases.append(dict(base,stop_lookup=i))
 h.VISITED.clear();rows=[]
 for pc,c in itertools.product(h.CW,cases):
  h.P=ORIGINAL;a=Machine(pc,c).run();h.P=V14;b=Machine(pc,c).run();assert a==b
  rows.append(dict(precision=pc,input=c,output=a))
 h.P=ORIGINAL;excluded={0x6f7810,0x6f7430,0x427bd0,0x43ff36,0x494380,0x494480}
 pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in excluded}
 doc=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),rows=rows,pins=pins,
  seams=['427BD0 immutable canonical lookup','allocation/free transports'],observed_cutoffs=['43FF36 ADstore notexecuted'],observed_entries=['494380/494480 execute after entry observation except noerrorshere'])
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-trigger-vfx.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=h.H(data))))
if __name__=='__main__':main()
