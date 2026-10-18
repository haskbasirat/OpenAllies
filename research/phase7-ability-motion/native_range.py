"""Complete original4C8270/4BE910 and concrete modifier helpers501D50/501DB0.
No gameplay hook is reached. Compare three successive binary32 residual stores
to retain the extended result beyond its first binary32 rounding.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-ability'))
import native_admission as admission
from unicorn.x86_const import *
class Machine(admission.Machine):
 def run_range(self):
  self.reset(0);self.w(self.STACK+4,admission.PROPS+0x1c if self.c['range_modifier'] else 0);self.u.reg_write(UC_X86_REG_ECX,admission.ABILITY)
  self.u.emu_start(0x4c8270,self.STOP,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  code=b''
  for i in range(2):code+=b'\xd9\x15'+struct.pack('<I',self.STATE+4*i)+b'\xd8\x25'+struct.pack('<I',self.STATE+4*i)
  code+=b'\xd9\x1d'+struct.pack('<I',self.STATE+8)
  self.u.mem_write(self.STOP,code);self.u.emu_start(self.STOP,self.STOP+len(code),count=5)
  assert not self.events
  return [self.r(self.STATE+i*4)for i in range(3)]
def cases():
 for pc in ['Single24','Double53','Extended64']:
  for projectile in [False,True]:
   for bombard in [False,True]:
    for modifier in [False,True]:
     for local in [False,True]:
      for index,(base,localf,instf,delta1,delta2)in enumerate([(15,.3333333,.717,-2.317,.819),(1,-1.33333,2.17,-9.,-2.),(.000719,-.0000771,871.337,3.1717,-2.7878)]):
       c=admission.base(f'{pc}_{projectile}_{bombard}_{modifier}_{local}_{index}');c.update(precision=pc,range_modifier=modifier,projectile_delta=admission.bits(delta1),bombard_delta=admission.bits(delta2));c['definition'].update(range=admission.bits(base),projectile=projectile,flags=8 if bombard else 0);c['local']['fuzzy_range']=admission.bits(localf);c['ability']['fuzzy_range']=admission.bits(instf)
       if not local:c['local']=None;c['ability']['local_definition']=None
       yield c
def main():
 rows=[];visited={}
 for c in cases():m=Machine(c);rows.append({'input':c,'output':m.run_range()});visited.update(m.executed)
 sha=lambda b:hashlib.sha256(b).hexdigest()
 doc={'schema':1,'method':__doc__,'original_sha256':sha(admission.original.b),'accepted_sha256':sha(admission.modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'harness_sha256':sha(Path(admission.__file__).read_bytes()),'cases':rows,'visited_windows':[{'va':hex(a),'bytes':b}for a,b in sorted(visited.items())]}
 (HERE/'native-range-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
