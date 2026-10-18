"""Original/v14 ServerClock ctor/restart/getters/host-rate, all three x87 PCs.

59ABA0 is an explicit input transport seam returning the supplied u32 result
of the real retained QPC timer; it is not a GetTickCount substitute or proof
of that timer. Cached Node+58 reads execute; the two host-rate reads can receive
different supplied values through a labelled external memory intervention.
No game process, original game, profile or archive is modified.
"""
from pathlib import Path
import hashlib,itertools,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_MEM_READ
ORIGINAL=h.P

class Required(Exception):pass
class Machine(h.Machine):
 def __init__(self,image,pc):
  h.P=ORIGINAL if image=='original' else h.V;super().__init__(pc)
  self.clock=self.alloc(16);self.node=self.alloc(0x80);self.w32(0xb36524,self.node)
  self.ms=0;self.ms_calls=0;self.cached_reads=0;self.cached_values=[];self.fail_time=False
  self.seams[0x59aba0]=self.timer
  self.u.hook_add(UC_HOOK_MEM_READ,self.read)
  self.call(0x75fbb0,self.clock)
  assert self.r32(self.clock)==0xa91ac4 and self.r32(self.clock+4,3)==[0,0,0]
 def timer(self,m):
  assert m.ecx()==self.node+0x50
  self.ms_calls+=1
  if self.fail_time:raise Required('timer_required')
  m.ret(value=self.ms)
 def read(self,u,access,address,size,value,_):
  if address<=self.node+0x58<address+size:
   assert self.cached_reads<len(self.cached_values),'unexpected cached Node+58 read'
   self.w32(self.node+0x58,self.cached_values[self.cached_reads]);self.cached_reads+=1
 def st0(self):
  # Unchanged native return value is stored to an80-bit fixture result slot.
  dst=self.alloc(16);stub=self.stub_next;self.stub_next+=16
  self.u.mem_write(stub,b'\xdb\x3d'+struct.pack('<I',dst))
  self.u.emu_start(stub,stub+6,count=2)
  assert self.fpu_top()==0
  return hex(int.from_bytes(self.u.mem_read(dst,10),'little'))
 def step(self,op):
  self.ms=op.get('ms',0);self.ms_calls=0;self.cached_reads=0
  self.cached_values=op.get('cached',[]);self.fail_time=op.get('fail_time',False)
  kind=op['kind'];result=None;outcome='ok'
  try:
   if kind=='restart':self.call(0x75fbf0,self.clock,[op['period'],op['start']])
   elif kind=='rate':self.call(0x75fd80,self.clock,[op['seconds']])
   else:
    entry={'raw':0x75fc40,'bounded':0x75fcb0,'elapsed':0x75fcf0}[kind]
    self.call(entry,self.clock,[int(bool(self.cached_values))]);result=self.st0()
  except Required as err:outcome=str(err)
  return {'outcome':outcome,'words':self.r32(self.clock+4,3),'extended':result,
          'cached_reads':self.cached_reads,'timer_calls':self.ms_calls}

def scenarios():
 for ms,start in itertools.product((0,1,499,1001,0x7fffffff,0x80000000,0xffffffff),(0,0x80000000,0x3dcccccd,0x447a1234)):
  yield {'name':f'restart_{ms:08x}_{start:08x}','ops':[
    {'kind':'restart','ms':ms,'period':0x3d800000,'start':start},
    {'kind':'raw','cached':[0x3f800001]}, {'kind':'raw','ms':ms},
    {'kind':'bounded','cached':[0x3f800001]}, {'kind':'bounded','ms':ms},
    {'kind':'elapsed','cached':[0x3f800001]}, {'kind':'elapsed','ms':ms},
    {'kind':'rate','cached':[0x3f800000,0x3f800001],'seconds':0x3dcccccd},
    {'kind':'bounded','cached':[0x3f800001]},
    {'kind':'restart','ms':ms,'period':0x3e000000,'start':0,'fail_time':True}]}
 for cached in (0,0x80000000,0x3dcccccd,0x3dccccce,0x3e4ccccd,0x3f800000,0x49742400):
  yield {'name':f'rate_{cached:08x}','ops':[
   {'kind':'restart','ms':0,'period':0x3d800000,'start':0},
   {'kind':'bounded','cached':[cached]},
   {'kind':'rate','cached':[cached,cached],'seconds':0x3dcccccd},
   {'kind':'raw','cached':[cached]}, {'kind':'bounded','cached':[cached]},
   {'kind':'elapsed','cached':[cached]}]}

def main():
 rows=[]
 for pc,image,s in itertools.product(h.PRECISION,('original','v14'),scenarios()):
  m=Machine(image,pc);steps=[{'input':op,'output':m.step(op)} for op in s['ops']]
  rows.append({'precision':pc,'image':image,'name':s['name'],'steps':steps})
 executed=[{'va':hex(a),'bytes':b} for a,b in sorted(h.VISITED.items()) if a!=0x59aba0]
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'seam_entry_not_executed':hex(0x59aba0),'executed_instructions':executed,'cases':rows}
 path=HERE/'native-server-clock.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'cases':len(rows),'executions':len(rows),'steps':sum(len(r['steps']) for r in rows),'executed_pins':len(executed),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
