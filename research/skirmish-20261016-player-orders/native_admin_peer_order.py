"""Actual752E60 traversal and whole751B60 empty histories. List fixtures
identify self first explicitly; the production fresh-host constructor's insertion
provenance is separate. No positive packet/SetPlayer dispatch body is seamed in.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import native_admin_ordering_owner as n
h=n.h;P=n.P;V=n.V
from unicorn.x86_const import *
class Machine(n.Machine):
 def _hook(self,u,pc,size,user):
  if pc==0x751b60:self.events.append({'peer':self.names[self.arg()],'current':self.r32(self.admin+0x284),'previous':self.r32(self.admin+0x208)})
  super()._hook(u,pc,size,user)
 def peers(self,count):
  a=self.admin;self.names={a:'self'};nodes=[a]
  for i in range(count-1):
   x=self.alloc(0x80);nodes.append(x);self.names[x]='remote'+str(i+1)
  if count==0:nodes=[]
  link=0
  for x in reversed(nodes):
   self.w32(x+0xc,0,0);node=self.alloc(8);self.w32(node,x,link);link=node
  self.w32(a+0x2ac,link);self.w32(a+0x284,17);self.w32(a+0x208,99);self.w32(a+0x280,0xffffffff)
  self.call(0x752e60,a,[]);assert self.final_esp==h.STACK+4 and self.fpu_top()==0
  return {'trace':self.events,'current':self.r32(a+0x284),'previous':self.r32(a+0x208),'current_client_280':self.r32(a+0x280)}
def main():
 rows=[]
 for image,label in [(P,'original'),(V,'v14')]:
  h.P=image
  for pc in h.PRECISION:
   for count in range(4):rows.append({'image':label,'precision':pc,'peer_count':count,'output':Machine(pc).peers(count)})
 h.P=P
 for va,raw in h.VISITED.items():assert bytes.fromhex(raw)==P.range(va,len(bytes.fromhex(raw)))==V.range(va,len(bytes.fromhex(raw)))
 out={'scope':__doc__,'producer_sha256':n.H(Path(__file__).read_bytes()),'rows':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-admin-peer-order.json';p.open('x').write(json.dumps(out,indent=1)+'\n');print(json.dumps({'rows':len(rows),'pins':len(h.VISITED),'sha256':n.H(p.read_bytes())}))
if __name__=='__main__':main()
