"""Read actual constructor-owned current UVar67/78 after41F9D0.
The same 213 registration loop executes; registry insertion remains named
transport. No source/profile/console override is claimed by this fixture.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-view-init'))
from native_uvar_registration import Machine as Base,h
class Machine(Base):
 def __init__(self,image,pc):self.message_rows=[];super().__init__(image,pc)
 def register(self):
  index=self.arg(0);p=self.arg(1)
  if index in(67,78):
   kind=self.r(p+0x24)
   self.message_rows.append({'index':index,'kind':kind,'flags':self.r(p+0x28),
       'current':self.r(p)if kind==3 else self.rb(p)})
  return super().register()
def main():
 rows=[];pins={};seams=set()
 for image,pc in itertools.product(('original','v14'),h.PRECISION):
  m=Machine(image,pc);m.run();rows.append({'image':image,'precision':pc,'variables':m.message_rows})
  seams|=m.intercepted
  for a,n in m.visited.items():
   if a in m.intercepted or not(0x401000<=a<0x9ef000):continue
   b=h.P.range(a,n);assert b==h.V.range(a,n);pins[f'{a:08x}']=b.hex()
  assert m.message_rows==[{'index':67,'kind':3,'flags':0,'current':50},{'index':78,'kind':1,'flags':16,'current':1}]
 out={'scope':__doc__,'rows':rows,'executed_pins':pins,'seam_entries_not_executed':[hex(a)for a in sorted(seams)]}
 data=json.dumps(out,indent=2,sort_keys=True)+'\n';path=HERE/'native-message-uvars.json'
 if '--write'in sys.argv:path.write_text(data,encoding='utf-8',newline='\n')
 else:assert path.read_text(encoding='utf-8')==data,'immutable message UVar replay differs'
 print(f'{len(rows)} native constructor runs, {len(pins)} executed pins PASS')
if __name__=='__main__':main()
