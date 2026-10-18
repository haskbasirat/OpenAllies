"""Actual CRT9E7400 default CString producer with successful StringManager
empty-allocation transport inherited from existing BootstrapMachine. Atexit
registration96CAB0 is a named successful seam; no destructor execution or
arbitrary alias mutation/perpetual original-process constant is certified.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges
class Machine(BootstrapMachine):
 def hook(self,u,pc,size,user):
  if self.bootstrap_ready and pc==0x96cab0:
   self.atexit=self.arg();self.ret(0,0);return
  super().hook(u,pc,size,user)
def main():
 rows=[];visited={}
 for image in ('original','v14'):
  m=Machine(image)
  #Inherited order-registry bootstrap skips unrelated CRT initializers. This
  #producer must execute this exact initializer rather than that lab seam.
  m.nonorder.discard(0x9e7400)
  for pc,old in itertools.product(PC,(0,0xa5a5a5a5,0xffffffff)):
   m.fpu(pc);m.begin(1,list(range(16)),0);m.w(0xb43b48,old);m.atexit=None
   m.call(0x9e7400);value=m.rs(0xb43b48)
   assert value==[]and m.atexit==0x9ef100
   rows.append({'image':image,'precision':pc,'old':old,'value':value,'atexit':m.atexit});visited.update(m.visited)
 out={'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),
  'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-default-string.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'CRT default rows')
if __name__=='__main__':main()
