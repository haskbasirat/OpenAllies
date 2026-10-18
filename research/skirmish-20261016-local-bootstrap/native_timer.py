"""Native59ABA0 QPC integer kernel, original/v14 allPC; OS counter seam only.
No application startup or wall-clock-frequency constructor claim is implied.
"""
from pathlib import Path
import json,struct,hashlib
from native_bootstrap import BootstrapMachine,LobbyMachine,PE0,PE1,PC,STUB,proof_ranges
from unicorn.x86_const import UC_X86_REG_EIP
HERE=Path(__file__).resolve().parent
class TimerMachine(BootstrapMachine):
 def __init__(self,image):
  super().__init__(image);self.w(0x9f021c,STUB+0x20);self.counter=0
 def hook(self,u,pc,size,user):
  if getattr(self,'bootstrap_ready',False)and pc==0x59aba0:
   # Run this body; inherited bootstrap only used it as diagnostic log time.
   LobbyMachine.hook(self,u,pc,size,user);return
  if getattr(self,'bootstrap_ready',False)and pc==STUB+0x20:
   u.mem_write(self.arg(),struct.pack('<q',self.counter));self.ret(4,1);return
  super().hook(u,pc,size,user)
def main():
 cases=[];visited={}
 pairs=[(0,0),(1,0),(-1,0),(1234567890123,1234567000000),
  (2**63-1,-1),(-2**63,1),(2**31+7,0),(0,2**63-1)]
 for image in ('original','v14'):
  m=TimerMachine(image);timer=m.alloc(8)
  for precision in PC:
   m.fpu(precision)
   for frequency in (1,3,1000,10_000_000,3_579_545,2**32+1,-3):
    m.u.mem_write(0xb3d760,struct.pack('<q',frequency))
    for counter,anchor in pairs:
     m.counter=counter;m.u.mem_write(timer,struct.pack('<q',anchor))
     milliseconds=m.call(0x59aba0,this=timer)
     delta=((counter-anchor+2**63)%2**64)-2**63
     product=((delta*1000+2**63)%2**64)-2**63
     expected=(abs(product)//abs(frequency))*(-1 if (product<0)!=(frequency<0) else 1)
     assert milliseconds==expected&0xffffffff,(counter,anchor,frequency,hex(milliseconds),expected)
     cases.append({'image':image,'precision':precision,'counter':counter,'anchor':anchor,'frequency':frequency,'milliseconds':milliseconds})
  visited.update(m.visited)
 original=[{k:v for k,v in r.items()if k!='image'}for r in cases if r['image']=='original']
 v14=[{k:v for k,v in r.items()if k!='image'}for r in cases if r['image']=='v14'];assert original==v14
 report={'passed':True,'scope':'Native59ABA0 integer body with injected real-API-shaped counter/frequency inputs, not native startup',
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),
  'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':cases,'executed_windows':proof_ranges(visited)}
 (HERE/'native-timer.json').write_text(json.dumps(report,indent=2)+'\n');print('passed',len(cases),'timer rows')
if __name__=='__main__':main()
