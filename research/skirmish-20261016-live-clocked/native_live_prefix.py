"""Native4F9970 fog/phase/auxiliary prefix over original/v14 at all x87 PCs.

Actual77D9D0 executes. The profiling41C0E0 instrumentation call is a declared
RET4 seam. FirstTick4F96D0 and the Windows current-thread API are required stops,
never successful bodies. Enabled positive sync logging is separately proved by
phase8-net/native_sync.py and reused through the existing Rust Synchronizer.
No game process or game/profile/archive file is changed.
"""
from pathlib import Path
import hashlib, itertools, json, sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_MEM_READ
ORIGINAL=h.P

class Required(Exception): pass
class Machine(h.Machine):
 def __init__(self, image, pc, row):
  h.P=ORIGINAL if image=='original' else h.V
  super().__init__(pc)
  self.row=row;self.fog=None;self.world_reads=0
  self.world=self.alloc(0x300);self.sync=self.alloc(0x40)
  self.w32(0xb3655c,self.sync)
  self.w8(self.sync,row['log_mode']!='disabled')
  self.w32(self.sync+4,1 if row['log_mode']=='suspended' else 0)
  self.w32(self.sync+8,0xffffffff)
  self.w32(self.world+0xd8,0);self.w32(self.world+0xdc,0)
  self.w32(self.world+0xe0,0,0,0x3d800000)
  self.aux=None
  if row['counter'] is not None:
   self.aux=self.alloc(4);self.w32(self.aux,row['counter']);self.w32(self.world+4,self.aux)
  svar=self.alloc(0x200);value=self.alloc(4)
  self.w32(0xb39ab0,svar);self.w32(svar+0x1bc,value);self.w8(value,row['svar_fog'])
  self.w8(self.world+0x275,row['world_fog'])
  self.seams[0x41c0e0]=lambda m:m.ret(4)
  self.seams[0x4f96d0]=lambda m:(_ for _ in ()).throw(Required('first_tick_required'))
  thread_api=h.STUBS+0x800
  self.w32(0x9f01e4,thread_api)
  self.seams[thread_api]=lambda m:(_ for _ in ()).throw(Required('sync_gate_required'))
  self.u.hook_add(UC_HOOK_MEM_READ,self.read)
 def read(self,u,access,address,size,value,_):
  if address<=self.world+0x275<address+size:self.world_reads+=1
 def _hook(self,u,pc,size,_):
  if pc==0x77d9d0:self.fog=self.arg(1)&255
  super()._hook(u,pc,size,_)
 def run(self):
  try:self.call(0x4f9970,self.world);raise AssertionError('must stop at a required boundary')
  except Required as err:outcome=str(err)
  return {'outcome':outcome,'fog':self.fog,'phase':self.r32(self.world+0xd8),
    'clock':self.r32(self.world+0xdc,4),'counter':self.r32(self.aux) if self.aux else None,
    'sync_hash':self.r32(self.sync+8),'world_fog_reads':self.world_reads}

def main():
 rows=[]
 for pc,image,svar,world,counter,log in itertools.product(h.PRECISION,('original','v14'),(False,True),(False,True),(None,8,0xffffffff),('disabled','suspended','required_gate')):
  row={'svar_fog':svar,'world_fog':world,'counter':counter,'log_mode':log}
  m=Machine(image,pc,row);out=m.run()
  assert out['world_fog_reads']==int(not svar)
  assert out['phase']==(0 if log=='required_gate' else 1)
  rows.append({'image':image,'precision':pc,'input':row,'output':out})
 stops={0x41c0e0,0x4f96d0}
 instructions=[{'va':hex(a),'bytes':b} for a,b in sorted(h.VISITED.items()) if a not in stops]
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
      'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
      'profile_seam':'41C0E0 RET4 instrumentation only','required_stops':['4F96D0','GetCurrentThreadId IAT9F01E4'],
      'executed_instructions':instructions,'cases':rows}
 path=HERE/'native-live-prefix.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'cases':len(rows),'executions':len(rows),'executed_pins':len(instructions),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))

if __name__=='__main__':
 main()
