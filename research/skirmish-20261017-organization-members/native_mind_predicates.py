"""Actual Mind64/7C used by57C800 over current embedded status, not stack guesses.
Base64 actual purecall stops at its real entry; no fabricated false result.
"""
from pathlib import Path
import sys,json,hashlib,itertools,struct
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import native_mind_callbacks as m
h=m.h;H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(m.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc,image,c)
  offset={('regiment',0x64):0x68,('regiment',0x7c):0x280,('element',0x64):0x7c,('element',0x7c):0x188,('aircraft',0x64):0x34}.get((c['kind'],c['slot']))
  self.status=None if offset is None else self.mind+offset+0xc
  if self.status is not None:self.w32(self.status,c['status'])
 def _hook(self,u,pc,n,d):
  if pc==0x96cac2:self.trace.append(['purecall']);self.stop='purecall';h.VISITED[pc]=bytes(u.mem_read(pc,n)).hex();assert h.P.range(pc,n)==h.V.range(pc,n);u.emu_stop();return
  super()._hook(u,pc,n,d)
 def run(self):
  entry=struct.unpack('<I',h.P.range(m.TABLE[self.c['kind']]+self.c['slot'],4))[0];value=None
  try:value=self.call(entry,self.mind,[])
  except AssertionError:
   if not self.stop:raise
  return {'trace':self.trace,'stop':self.stop,'value':None if value is None else bool(value&255)}
def main():
 rows=[]
 for kind,slot,status,pc in itertools.product(m.TABLE,[0x64,0x7c],[0,1,2,3,0x80000000,0xffffffff],h.PRECISION):
  c={'kind':kind,'slot':slot,'status':status,'operation':'predicate','word50':1,'wordb8':2,'old':0,'new':0}
  results=[Machine(pc,image,c).run()for image in(h.P,h.V)];assert results[0]==results[1];rows.append({'precision':pc,'input':c,'native':results[0]})
 doc={'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'constructor_helper_sha256':H(Path(m.__file__).read_bytes()),'rows':rows,'original_v14_executions':2*len(rows),'pins':[{'va':hex(a),'bytes':b,'kind':'required_purecall_entry'if a==0x96cac2 else'executed'}for a,b in sorted(h.VISITED.items())]}
 path=HERE/'native-mind-predicates.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'rows':len(rows),'executions':doc['original_v14_executions'],'pins':len(doc['pins']),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
