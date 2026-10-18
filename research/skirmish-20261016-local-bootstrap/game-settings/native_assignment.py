"""Whole76B500 dispatch around named successful5A46D0/5144A0 copy seams.
The component algorithms are separately exercised by native-game-settings and
the Creator author oracle. No callback reentry/allocation failure is asserted.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges
from unicorn.x86_const import UC_X86_REG_ECX
class Machine(BootstrapMachine):
 def __init__(self,image):
  self.active=False;super().__init__(image)
 def hook(self,u,pc,size,user):
  if self.active and pc in (0x5a46d0,0x5144a0):
   target=u.reg_read(UC_X86_REG_ECX);assert self.arg()==self.source
   if pc==0x5a46d0:
    assert target==self.game;self.events.append('game_settings')
   else:
    slot=next(offset for offset,ptr in self.creators.items()if ptr==target)
    self.events.append(f'creator_{slot:02x}')
   self.ret(4,target);return
  super().hook(u,pc,size,user)
def main():
 rows=[];visited={}
 for image in ('original','v14'):
  m=Machine(image);m.begin(1,list(range(16)),0)
  for pc in PC:
   m.fpu(pc)
   for mode in (0,1,2,3,4,5,6,0xffffffff):
    launch=m.alloc(0x30);m.game=m.alloc(0x68);m.source=m.alloc(0x68)
    m.w(launch+4,mode);m.w(launch+0x14,m.game)
    m.creators={off:m.alloc(0x54)for off in (0x18,0x1c,0x20,0x28)}
    for off,ptr in m.creators.items():m.w(launch+off,ptr)
    m.active=True;m.events=[];m.call(0x76b500,[m.source],this=launch,callee_pop=4);m.active=False
    selected={0:0x20,2:0x1c,5:0x28}.get(mode,0x18)
    assert m.events==['game_settings',f'creator_{selected:02x}']
    rows.append({'image':image,'precision':pc,'mode':mode,'creator_slot':selected,'events':m.events})
  visited.update(m.visited)
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 out={'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-assignment.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'76B500 copy/dispatch rows')
if __name__=='__main__':main()
