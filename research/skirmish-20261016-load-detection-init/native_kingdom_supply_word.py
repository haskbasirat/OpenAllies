"""5595D0 actual WORD-domain read at Kingdom+22C, including poisoned neighbors.
No callbacks or successful seams. This validates storage width, not a full zone.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(h.Machine):
 def __init__(self,pc,image):
  super().__init__(pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   self.u.mem_write(image.base,image.b[:0x1000])
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
 def run(self,low,adjacent,supply,nonmajor):
  k=self.alloc(0x240);w=self.alloc(4);self.w32(k+0x190,nonmajor);self.w32(k+0x22c,(adjacent<<16)|low);self.w32(w,supply)
  reads=[]
  self.u.hook_add(UC_HOOK_MEM_READ,lambda u,a,p,n,v,d:reads.append([p-k,n]),begin=k+0x22c,end=k+0x22f)
  result=self.call(0x5595d0,w,[k])&0xff
  assert result==int(not nonmajor and bool(low&supply))
  assert reads==([] if nonmajor else [[0x22c,2]])
  return {'result':result,'supply_reads':reads}
def main():
 rows=[]
 for pc,(low,supply),adjacent,nonmajor in itertools.product(h.PRECISION,[(0,0xffff),(0x9234,0x8000),(0x9234,0x4000),(0x0100,0x0100)],(0,0xffff),(0,1)):
  results=[Machine(pc,image).run(low,adjacent,supply,nonmajor) for image in(h.P,h.V)];assert results[0]==results[1]
  rows.append({'precision':pc,'low_word':low,'adjacent_word':adjacent,'supply':supply,'nonmajor':nonmajor,'native':results[0]})
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'rows':rows,'native_machines':len(rows)*2,'executed_pins':[{'va':hex(a),'bytes':b} for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-kingdom-supply-word.json'
 if '--write' in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'native_machines':len(rows)*2,'executed_pins':len(h.VISITED),'fixture_sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
