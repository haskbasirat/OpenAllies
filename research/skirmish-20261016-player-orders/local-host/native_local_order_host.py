"""Unchanged original75D710 and actual76B8F0/74B900/74B400/
8569A0/74B350/74B3A0, plus766C30/766C40/766C50/766C70 getters.
Raw memory fixtures describe the bounded local non-film roster. Only404F60
warning sink is a seam; no player lookup/client-store/security result is supplied.
"""
from pathlib import Path
import sys,json,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
P=h.P;V=h.V
from unicorn.x86_const import *
class Machine(h.Machine):
 def __init__(self,pc,c):
  super().__init__(pc);self.a=self.alloc(0x400);self.s=self.alloc(0x200);self.p=self.alloc(0x80);self.old=self.alloc(0x80);self.order=self.alloc(0x20);node=self.alloc(8)
  self.w32(0xb36558,self.a);self.w32(0xb36550,self.s);self.w32(self.s+0xbc,node);self.w8(self.s+0x24,0);self.w32(self.s+0xe4,c.get('state',3))
  self.w32(node,self.p,0);self.w32(self.p+4,self.a);self.w32(self.p+0x20,1);self.w8(self.p+0xe,0);self.w32(self.old+0x20,17)
  self.w32(self.a+0x2c4,self.a if c.get('host',True)else 0);self.w32(self.a+0x280,self.a if c.get('peer',True)else 0);self.w32(self.a+0x284,self.p if c.get('current',True)else 0);self.w32(self.a+0x2c,self.old);self.w8(self.a+0x6c,c.get('version',1));self.w32(self.a+0x268,self.alloc(4)if c.get('mirror',False)else 0)
  self.events=[]
  def diag(m):self.events.append('native_warning404F60');m.ret()
  self.seams[0x404f60]=diag
 def normalize(self,p):return 0 if p==0 else self.r32(p+0x20)
 def set_player(self,c):
  self.w32(self.order+4,c['id']);self.call(0x75d710,self.order,[]);assert self.final_esp==h.STACK+4 and self.fpu_top()==0
  return {'current':self.normalize(self.r32(self.a+0x284)),'node2c':self.normalize(self.r32(self.a+0x2c)),'warnings':self.events}
 def getter(self,c):
  entry={'client':0x766c30,'version':0x766c40,'mirror':0x766c50,'film':0x766c70}[c['getter']]
  v=self.call(entry,0,[]);assert self.final_esp==h.STACK+4 and self.fpu_top()==0
  return {'value':bool(v)if c['getter']!='version'else v&255,'warnings':self.events}
def main():
 cases=[('set_player',dict(host=host,peer=peer,id=id))for host,peer,id in itertools.product([False,True],[False,True],[1,999])]
 cases +=[('getter',{'getter':'client','current':v})for v in [False,True]]+[('getter',{'getter':'version','version':v})for v in [0,1,255]]+[('getter',{'getter':'mirror','mirror':v})for v in [False,True]]+[('getter',{'getter':'film','state':v})for v in [0,3,4,5]]
 rows=[]
 for image,label in [(P,'original'),(V,'v14')]:
  h.P=image
  for pc in h.PRECISION:
   for kind,c in cases:rows.append({'image':label,'precision':pc,'kind':kind,'input':c,'output':getattr(Machine(pc,c),kind)(c)})
 h.P=P
 for va,raw in h.VISITED.items():assert bytes.fromhex(raw)==P.range(va,len(bytes.fromhex(raw)))==V.range(va,len(bytes.fromhex(raw)))
 import hashlib
 H=lambda b:hashlib.sha256(b).hexdigest()
 out={'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'rows':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-local-order-host.json';p.open('x').write(json.dumps(out,indent=1)+'\n');print(json.dumps({'rows':len(rows),'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
