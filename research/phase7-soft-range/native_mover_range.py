"""Execute the original577530..57757D fragment within Mover5774A0.
The enclosing caller supplies a valid target GTID and range. The fragment
captures soft virtual+10 source first, target second, then calls46F100 with
the SOURCE descriptor as receiver. Full movement/earlier admission is outside
this fragment. Ordinary getters and all geometry execute original code;
derived mutable getters are explicit fixture effects inherited from the
52FE60 producer. No geometry helper is replaced.
"""
from pathlib import Path
import copy,hashlib,json
from native_soft_range import Machine as ArrivalMachine,cases,A,B,bits,original,modified
from unicorn.x86_const import *
HERE=Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
class Machine(ArrivalMachine):
 def run(self):
  self.reset(0)
  self.put(self.STATE+0x44,0x10002,self.c.get('range_bits',0x3dcccccd))
  self.u.reg_write(UC_X86_REG_ECX,A)
  self.u.reg_write(UC_X86_REG_ESI,self.STATE)
  self.u.emu_start(0x577530,0x57757d,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==0x57757d and self.u.reg_read(UC_X86_REG_ESP)==self.STACK
  return {'result':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'events':self.events}
def mover_cases():
 yield from cases()
 base=next(cases())
 for precision in ['Single24','Double53','Extended64']:
  for ordinary in [True,False]:
   c=copy.deepcopy(base);c.update(name='receiver_witness_'+precision+'_'+str(ordinary),precision=precision,ordinary=ordinary)
   c['actors']['1'].update(position=list(map(bits,[0,0])),facing=list(map(bits,[0,-1])))
   c['actors']['2'].update(position=list(map(bits,[4,0])),facing=list(map(bits,[0,-1])))
   c['shapes']['1']={'kind':3,'radius_bits':bits(4.25**.5),'bounds':list(map(bits,[-2,-.5,2,.5]))}
   c['shapes']['2']={'kind':3,'radius_bits':bits(4.25**.5),'bounds':list(map(bits,[-.5,-2,.5,2]))}
   yield c
def main():
 rows=[];visited={};differences=0
 for c in mover_cases():
  m=Machine(c);output=m.run();rows.append({'input':c,'output':output});visited.update(m.visited)
  differences+=output['result']!=ArrivalMachine(c).run()['result']
 assert differences>=6,'missing receiver-order branch witnesses'
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b))
 ranges={}
 for label,a,end in [('mover_source_receiver_fragment',0x577530,0x57757d),('soft_descriptor',0x4ea0f0,0x4ea143),('ordinary_soft_getter',0x4ea020,0x4ea027)]:
  b=original.range(a,end-a);assert b==modified.range(a,end-a);ranges[label]={'va':hex(a),'length':end-a,'sha256':sha(b)}
 doc={'schema':1,'method':__doc__,'original_sha256':sha(original.b),'accepted_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'parent_producer_sha256':sha((HERE/'native_soft_range.py').read_bytes()),'ranges':ranges,'cases':rows,'receiver_differences_against_arrival':differences,'visited_windows':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())]}
 (HERE/'native-mover-range.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows),'receiver_differences_against_arrival':differences}))
if __name__=='__main__':main()
