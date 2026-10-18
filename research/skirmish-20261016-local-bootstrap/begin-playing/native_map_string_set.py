"""Actual49AE00 pair/registered-entry writes;4BAA20 is the named existing-entry
lookup seam, including the captured currentpair+4 at that boundary. No native
missing-entry creation/full text-manager behavior is claimed.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges
from unicorn.x86_const import *
class TextMachine(BootstrapMachine):
 def hook(self,u,pc,size,user):
  if getattr(self,'pair_ready',False)and pc==0x4baa20:
   assert self.arg(1)==self.pair and self.arg(2)==1
   self.events.append({'call':'4BAA20','create':1,'pair_text_before':self.rs(self.pair+4),'supplied':self.rs(self.arg(3))})
   self.w(self.arg(),self.entry);self.w(self.entry+4,self.r(self.entry+4)+1)
   self.ret(16,self.arg());return
  super().hook(u,pc,size,user)
def main():
 rows=[];visited={}
 for image in ('original','v14'):
  m=TextMachine(image);m.pair_ready=True;m.pair=m.alloc(8);m.entry=m.alloc(0x14);m.w(m.entry,0);m.w(m.entry+4,100)
  for p in PC:
   for key in ('','MapKey','MAPKEY'):
    for text in ('','Host','A\x00B','\u03a9'):
     m.fpu(p);m.cs(m.pair,key);m.cs(m.pair+4,'old-pair');m.cs(m.entry+0x10,'old-global');m.events=[]
     v=m.alloc(4);m.cs(v,text);m.call(0x49ae00,[v],this=m.pair,callee_pop=4)
     assert m.rs(m.pair+4)==[ord(c)for c in text]
     assert m.rs(m.entry+0x10)==[ord(c)for c in ('old-global'if not key else text)]
     rows.append({'image':image,'precision':p,'key_units':[ord(c)for c in key],'key':key,'supplied_units':[ord(c)for c in text],'pair_units':m.rs(m.pair+4),'global_units':m.rs(m.entry+0x10),'events':m.events.copy()})
  visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 out={'passed':True,'scope':'Actual49AE00 pair assignment/existing registered-entry+10 write; named4BAA20 lookup seam, not full manager or missing creation','producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-map-string-set.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'mapstring rows')
if __name__=='__main__':main()
