"""Actual4ECA80 allocation -> actual4EC4A0 -> reread Kingdom index -> slot publication.
Eligibility is an explicit true test seam (existing separate predicate proof).
Heap/CString allocator transport is explicit. Active observer stops at79E7B0;
no observer callback is replaced by success. A labelled exit probe changes the
Kingdom index to prove the later native reread, not a real game callback claim.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));from native_memory_refresh import Machine,h,H
from unicorn.x86_const import UC_X86_REG_EIP
class Publication(Machine):
 def __init__(self,pc,image,c):
  base={'class':9,'time':0x40500000,'flags':0x20000000,'position':[0x3fc00000,0xc0200000],'variant':17,'owner':0,'display':'Pine','key':'','text':'','kingdom':'','area':'','camera':'','stop':'none','data_variations':False}
  super().__init__(pc,image,base);self.case=c;self.allocs=[];self.created=None
  self.kingdom=self.alloc(0x300);self.w(self.kingdom+0x18c,0);self.w(self.kingdom+0x190,0);self.w(self.world+0x158,3)
  self.w(self.actor+0x58,0);self.w(self.actor+0x5c,0)
  self.observer=self.alloc(0x30)if c['observer']else 0;self.w(0xb36528,self.observer)
  if self.observer:self.wb(self.observer+0x20,int(c['active']))
  def malloc(m):
   size=m.arg(0);p=m.alloc(size,0xa5);self.allocs.append([size,p]);m.ret(0,p)
  self.seam(0x6f7810,malloc);self.seam(0x4eb490,lambda m:m.ret(0,1))
  def record(m):self.created=m.arg(0)
  self.probes[0x4ec4a0]=record
  def change(m):
   if c['rewrite_index']:m.w(self.kingdom+0x18c,1)
  self.probes[0x4ec588]=change
  def observer(m):self.required='79E7B0';m.u.emu_stop()
  self.seam(0x79e7b0,observer)
 def run(self):
  try:self.call(0x4eca80,self.actor,[self.kingdom])
  except AssertionError:
   if not self.required:raise
  assert self.u.reg_read(UC_X86_REG_EIP)==h.STOP or self.required
  r=self.created;assert r is not None
  return {'outcome':self.required or'complete','count':self.r(self.actor+0x58),'slots':[self.r(self.r(self.actor+0x5c)+4*i)==r for i in range(3)],'time':self.r(r),'name':self.cstring_text(self.r(r+4)),'position':[self.r(r+8),self.r(r+0xc)],'definition_matches':self.r(r+0x10)==self.definition,'owner':self.r(r+0x14),'aspect':self.r(r+0x18),'stats':[self.r(r+off)for off in(0x1c,0x20,0x24,0x28,0x2c)],'flags':self.r(r+0x34),'allocations':[s for s,p in self.allocs]}
def main():
 rows=[]
 for present,active in[(False,False),(True,False),(True,True)]:
  for rewrite in[False,True]:
   c={'observer':present,'active':active,'rewrite_index':rewrite}
   for pc in h.CW:
    vals=[Publication(pc,image,c).run()for image in(h.P,h.V)];assert vals[0]==vals[1];rows.append({'pc':pc,'input':c,'native':vals[0]})
 seams={0x6f7810:'heap_allocator_transport',0x856370:'CString_allocator_transport',0x4eb490:'explicit_eligibility_test_answer',0x79e7b0:'required_observer_entry'}
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(h.P.b),'v14_sha256':H(h.V.b),'rows':rows,'machines':len(rows)*2,'pins':[{'va':hex(a),'bytes':b,'kind':seams.get(a,'executed')}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-memory-publication.json'
 if'--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':doc['machines'],'pins':len(doc['pins']),'fixture_sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
