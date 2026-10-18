"""Original/v14 CRT B43B40 empty CString producer. Successful StringManager
allocation and atexit registration are named transport seams. No constructor
for a missing Session/World, destructor execution or arbitrary alias writes is
claimed. Fixture generation is explicit; the default independently replays.
"""
from pathlib import Path
import hashlib, importlib.util, itertools, json, sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
MODULE=ROOT/'research/skirmish-20261016-local-bootstrap/implicit-ai/native_default_string.py'
spec=importlib.util.spec_from_file_location('film_crt_transport',MODULE)
base=importlib.util.module_from_spec(spec);spec.loader.exec_module(base)
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_ESP,UC_X86_REG_EIP
class Machine(base.Machine):
 def __init__(self,image):
  self.recording=False;super().__init__(image);self.actual={};self.recording=True
 def hook(self,u,pc,size,user):
  super().hook(u,pc,size,user)
  if self.recording and u.reg_read(UC_X86_REG_EIP)==pc and 0x400000<=pc<0x9f0000:self.actual[pc]=size

def produce():
 rows=[];constructors=[];visited={}
 for image in ('original','v14'):
  m=Machine(image);m.nonorder.discard(0x9e73c0)
  for pc,old in itertools.product(base.PC,(0,0xa5a5a5a5,0xffffffff)):
   m.fpu(pc);m.begin(1,list(range(16)),0);m.w(0xb43b40,old);m.atexit=None;m.actual.clear()
   m.call(0x9e73c0)
   row={'image':image,'precision':pc,'old':old,'value':m.rs(0xb43b40),'atexit':m.atexit}
   assert row['value']==[] and row['atexit']==0x9ef0e0
   rows.append(row);visited.update(m.actual)
   m.w(m.S+0x150,old);m.u.reg_write(UC_X86_REG_ESI,m.S);m.u.reg_write(UC_X86_REG_ESP,m.stack)
   m.actual.clear();m.u.emu_start(0x76f9d8,0x76f9f7,count=10000)
   assert m.u.reg_read(UC_X86_REG_EIP)==0x76f9f7
   value=m.rs(m.S+0x150);assert value==[]
   constructors.append(dict(image=image,precision=pc,old=old,value=value));visited.update(m.actual)
 return {'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(base.PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(base.PE1.b).hexdigest(),
  'cases':rows,'session150_constructor_window':constructors,'executed_windows':base.proof_ranges(visited)}
if __name__=='__main__':
 result=produce();target=HERE/'native-application-film-string.json'
 if '--write' in sys.argv:target.write_text(json.dumps(result,indent=2)+'\n')
 else:assert json.loads(target.read_text())==result
 print('passed',len(result['cases']),'CRT B43B40 rows')
