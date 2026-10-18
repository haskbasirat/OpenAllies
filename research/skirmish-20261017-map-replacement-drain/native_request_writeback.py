"""4FBA50 aliases the queued Params through explicit mutating creation leaves.

This proves parent request ownership and retained write prefixes only. The
laboratory creation leaf writes four chosen words before returning or stopping;
it does not claim real4F9C10/4FA590 mutation values or creation correctness.
Original frozen replacement-body fixture/producer remain unchanged.
"""
from pathlib import Path
import json,sys,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_replacement_drain import Replacement,run,PE0,PE1,PC

WRITES={8:0x41200000,0x14:0x3f800000,0x20:7,0x24:0x20009}
class Writeback(Replacement):
 def hook(self,u,pc,size,user):
  if self.replacement_ready and pc in [0x4f9c10,0x4fa590]:
   assert self.arg()==self.params
   for off,value in WRITES.items():self.w(self.arg()+off,value)
  super().hook(u,pc,size,user)

def main():
 cases=[]
 for actor in [False,True]:
  cases.extend([dict(actor=actor),dict(actor=actor,success=False),dict(actor=actor,cutoff='create_actor'if actor else'create_object'),dict(actor=actor,cutoff='created_log'),dict(actor=actor,success=False,cutoff='failure_log')])
 cases.append(dict(actor=True,cutoff='property'))
 rows=[];pins={}
 for image in ['original','v14']:
  m=Writeback(image)
  for pc in PC:
   for case in cases:
    output,visited=run(m,pc,case)
    output['params_words']=[m.r(m.params+off)for off in WRITES]
    assert output['params_words']==list(WRITES.values())
    pins.update(visited);rows.append(dict(image=image,precision=pc,input=case,output=output))
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),written_offsets=list(WRITES),rows=rows,instructions=instructions)
 p=HERE/'native-request-writeback.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executed_pins=len(instructions),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
