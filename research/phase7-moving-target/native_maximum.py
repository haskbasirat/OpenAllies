"""Complete original57EC80 enabled-ability maximum, actual4C8270 and modifiers."""
from pathlib import Path
import copy,hashlib,json,sys,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-ability'))
import native_admission as base
from unicorn.x86_const import *
ARRAY,ENTRIES=0x4030000,0x4031000
class Machine(base.Machine):
 def __init__(self,c):
  super().__init__(c);self.w(base.PERF+4,base.ACTOR);self.w(base.PERF+0x14,ARRAY);self.w(base.PERF+0x18,len(c['abilities']));self.w(base.PERF+0x1c,len(c['abilities']))
  for i,a in enumerate(c['abilities']):
   p=ENTRIES+i*0x100;self.w(ARRAY+i*4,p);self.u.mem_write(p,bytes(self.u.mem_read(base.ABILITY,0x2c)));self.u.mem_write(p+0x18,bytes([a['enabled']]));self.w(p+0x1c,a['fuzzy_range'])
 def maximum(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,base.PERF);self.u.emu_start(0x57ec80,self.STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  self.u.mem_write(self.STOP,b'\xd9\x1d'+struct.pack('<I',self.STATE));self.u.emu_start(self.STOP,self.STOP+6,count=1);assert not self.events;return self.r(self.STATE)
def cases():
 for pc in ['Single24','Double53','Extended64']:
  for count in [0,1,4]:
   for enabled in ['all','none','mixed']:
    for modifier in [False,True]:
     for magnitude in [15.,-1.]:
      c=base.base(f'{pc}_{count}_{enabled}_{modifier}_{magnitude}');c['precision']=pc;c['definition']['range']=base.bits(magnitude);c['definition']['projectile']=True;c['actors']['1']['has_properties']=modifier;c['projectile_delta']=base.bits(.7777777);c['local']['fuzzy_range']=base.bits(1.3333333)
      c['abilities']=[{**copy.deepcopy(c['ability']),'enabled':enabled=='all'or(enabled=='mixed'and i%2==1),'fuzzy_range':base.bits([.717,-7.,1.9999999,3.3333333][i])}for i in range(count)]
      yield c
def main():
 rows=[];visited={}
 for c in cases():m=Machine(c);rows.append({'input':c,'output':m.maximum()});visited.update(m.executed)
 for a,h in visited.items():
  b=bytes.fromhex(h);assert b==base.original.range(a,len(b))==base.modified.range(a,len(b)),hex(a)
 sha=lambda b:hashlib.sha256(b).hexdigest()
 ranges={}
 for label,a,end in [('maximum',0x57ec80,0x57ed15),('iterator_start',0x517f60,0x517fd0)]:
  b=base.original.range(a,end-a);assert b==base.modified.range(a,end-a);ranges[label]={'va':hex(a),'length':end-a,'sha256':sha(b)}
 doc={'schema':1,'scope':__doc__,'producer_sha256':sha(Path(__file__).read_bytes()),'original_sha256':sha(base.original.b),'accepted_sha256':sha(base.modified.b),'cases':rows,'ranges':ranges,'visited_windows':[{'va':hex(a),'bytes':h}for a,h in sorted(visited.items())]}
 (HERE/'native-maximum-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
