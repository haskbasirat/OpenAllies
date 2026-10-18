"""75C600 queue constructor and whole75C9A0 constructor-empty fast path.
Platform critical-section/event constructors are named backend seams. The
positive queue stops at the first diagnostic before7516E0 or dequeue: this
does not certify any command payload/handler or a successful positive drain.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-local-bootstrap'))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges,NODE
from unicorn.x86_const import UC_X86_REG_EIP
class Stop(Exception):pass
class Machine(BootstrapMachine):
 def __init__(self,image):
  self.proving=False;super().__init__(image);self.proving=True;self.actual={};self.platform_seams=set()
 def hook(self,u,pc,size,user):
  if self.proving:
   if pc in(0x857d60,0x857840):
    self.platform_seams.add(pc);self.events.append({'kind':'critical_ctor'if pc==0x857d60 else'event_ctor','argument':None if pc==0x857d60 else self.arg()})
    self.ret(0 if pc==0x857d60 else 4);return
   if pc==0x404f60:
    self.platform_seams.add(pc);self.events.append({'kind':'positive_diagnostic'});raise Stop()
  super().hook(u,pc,size,user)
  if self.proving and u.reg_read(UC_X86_REG_EIP)==pc and 0x401000<=pc<0x9ef000:self.actual[pc]=size
 def run(self,precision,positive):
  self.fpu(precision);self.events=[];self.u.mem_write(NODE+0x118,b'\xa5'*0x40)
  self.call(0x75c600,this=NODE+0x118);ctor={hex(o):self.r(NODE+0x118+o)for o in(0x28,0x2c,0x30,0x34)}
  constructor_events=list(self.events);self.events=[]
  if positive:self.w(NODE+0x14c,positive)
  stop=None
  try:self.call(0x75c9a0,this=NODE)
  except Stop:stop='positive_diagnostic'
  return dict(precision=precision,positive_count=positive,constructor=ctor,constructor_events=constructor_events,
    stop=stop,events=self.events,count_after=self.r(NODE+0x14c))
def main():
 rows=[];pins={};seams=set()
 for image in('original','v14'):
  m=Machine(image)
  for pc,count in itertools.product(PC,(0,1,7)):
   rows.append(dict(image=image,**m.run(pc,count)));pins.update(m.actual);seams|=m.platform_seams
 for p in seams:pins.pop(p,None)
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),
  'seams':[hex(s)for s in sorted(seams)],'executed_windows':proof_ranges(pins),'cases':rows}
 p=HERE/'native-command-queue.json'
 if'--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'runs':len(rows),'windows':len(doc['executed_windows']),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
