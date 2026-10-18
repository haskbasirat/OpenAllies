"""Bounded7757F0 successful-allocation caller. Native CString copies and
control instructions execute; preparePlayers/film-depot/open/preamble/SAI/
playback and two proven CString path helpers are named laboratory leaves.
Required-leaf stops are not C++ exceptions and do NOT enter native SEH retry.
No Host task, real map startup, film IO or activation is certified here.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-local-bootstrap'))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ECX
class RequiredStop(Exception):pass
CLOSE=0x7700a00
def units(s):return list(s.encode('utf-16-le')[i]+(s.encode('utf-16-le')[i+1]<<8)for i in range(0,len(s.encode('utf-16-le')),2))
def joined(a,b):
 sep=lambda c:c in(47,92)
 a=list(a)
 if b and sep(b[0]):
  if a and sep(a[-1]):a.pop()
 elif a and not sep(a[-1]):a.append(47)
 return a+list(b)
class Machine(BootstrapMachine):
 def __init__(self,image):
  self.ready=False;super().__init__(image);self.u.mem_write(CLOSE,b'\xc3')
  self.ready=True;self.actual={};self.events=[]
 def event(self,kind,**args):
  if self.case.get('stop')==kind:raise RequiredStop(kind)
  self.events.append(dict(kind=kind,**args))
 def hook(self,u,pc,size,user):
  if self.ready:
   if pc==0x74b310:self.event('detach');self.ret();return
   if pc==CLOSE:self.event('close');self.ret();return
   if pc==0x773e30:
    self.event('prepare')
    if 'prepare_state'in self.case:self.w(self.S+0xe4,self.case['prepare_state'])
    if 'prepare_phase'in self.case:self.w(self.S+0xf8,self.case['prepare_phase'])
    self.ret();return
   if pc==0x405370:
    assert self.text(self.r(self.arg()))=='FilmRotationQueueSize'
    self.event('rotation',value=self.r(self.rot))
    self.ret(4,self.rotptr);return
   if pc==0x76fe60:
    assert self.arg()==1
    self.event('filename')
    self.cs(self.S+0x150,'chosen_name')
    if 'filename_rotation'in self.case:self.w(self.rot,self.case['filename_rotation'])
    self.ret(4,self.S+0x150);return
   if pc==0x404f60 and self.text(self.arg(1)).endswith('Session.cpp'):
    self.event('opening',name=units(self.text(self.arg(5))))
    self.ret();return
   if pc==0x85ab30:
    name=self.rs(self.arg());extension=self.rs(self.arg(1))
    self.event('extension',extension=extension)
    self.cs(self.arg(),name+([46]+extension if extension else[]));self.ret(value=self.arg());return
   if pc==0x410cb0:
    assert self.text(self.arg(1))=='PathFilms'
    self.event('path');self.cs(self.arg(),'films/');self.ret(8,self.arg());return
   if pc==0x85abb0:
    value=joined(self.rs(self.arg(1)),self.rs(self.arg(2)))
    self.cs(self.arg(),value);self.event('join',value=value);self.ret(value=self.arg());return
   if pc==0x5a3250:
    self.event('construct',ids=self.rs(self.arg()),creator=self.arg(1)==self.creator,rules=self.arg(2)==self.header+0x38)
    self.ret(value=self.film);return
   if pc==0x5a00e0:self.event('open',published=self.r(self.S+0x1c)==self.film);self.ret();return
   if pc==0x76c310:self.event('record');self.ret();return
   if pc==0x7a0d40:
    self.event('sai')
    if 'sai_state'in self.case:self.w(self.S+0xe4,self.case['sai_state'])
    self.ret();return
   if pc==0x76e1d0:self.event('playback');self.ret();return
  super().hook(u,pc,size,user)
  if self.ready and u.reg_read(UC_X86_REG_EIP)==pc and 0x400000<=pc<0x9f0000:self.actual[pc]=size
 def run(self,case,precision):
  self.case=case;self.fpu(precision);self.begin(1,list(range(16)),0)
  self.w(self.S+0xe4,case['state']);self.w(self.S+0xf8,case['phase']);self.b(self.S+0x168,0xa5)
  self.cs(self.S+0x150,'old_name');self.cs(0xb43b40,'')
  self.film=self.alloc(0xb4);vt=self.alloc(0x30);self.w(vt+0x28,CLOSE);self.w(self.film+0x3c,vt)
  self.w(self.S+0x1c,self.film if case['old'] else 0)
  self.creator=self.alloc(0x10);self.header=self.alloc(0x80);settings=self.alloc(0xa0);world=self.alloc(0x30)
  self.w(settings+0x88,self.creator);self.w(settings+0x90,self.header);self.w(world+0x28,settings);self.w(0xb36534,world)
  launch=self.alloc(0x40);self.cs(launch+0x3c,'playback' if case.get('playback_name',True)else'');self.w(self.S+0x68,launch)
  self.rot=self.alloc(4);self.w(self.rot,case['rotation']&0xffffffff);self.rotptr=self.alloc(4);self.w(self.rotptr,self.rot)
  self.events=[];stop=None
  try:self.call(0x7757f0,this=self.S)
  except RequiredStop as e:stop=str(e)
  except Exception as e:raise AssertionError((case,precision,self.events,str(e)))from e
  return dict(input=case,precision=precision,events=self.events,stop=stop,name=self.rs(self.S+0x150),
   state=self.r(self.S+0xe4),phase=self.r(self.S+0xf8),film=self.r(self.S+0x1c)!=0,flag168=self.rb(self.S+0x168))
def produce():
 cases=[dict(state=s,phase=p,rotation=r,old=o)for s,p,r,o in itertools.product((0,2,4,5),(0,7,14),(-1,0,2),(False,True))]
 for extra in ({'prepare_state':4},{'prepare_state':0},{'prepare_phase':7},{'filename_rotation':0},{'sai_state':4},{'playback_name':False}):
  cases.append(dict(state=2,phase=0,rotation=2,old=True,**extra))
 for stop in ('prepare','rotation','filename','construct','open','record','sai','playback'):
  cases.append(dict(state=4 if stop=='playback' else 2,phase=0,rotation=2,old=True,stop=stop))
 rows=[];pins={}
 for image in ('original','v14'):
  m=Machine(image)
  for pc,case in itertools.product(PC,cases):rows.append(dict(image=image,**m.run(case,pc)))
  pins.update(m.actual)
 for i in range(len(rows)//2):assert {k:v for k,v in rows[i].items()if k!='image'}=={k:v for k,v in rows[i+len(rows)//2].items()if k!='image'}
 return dict(passed=True,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  original_sha256=hashlib.sha256(PE0.b).hexdigest(),v14_sha256=hashlib.sha256(PE1.b).hexdigest(),cases=rows,executed_windows=proof_ranges(pins))
if __name__=='__main__':
 result=produce();path=HERE/'native-world-ready.json'
 if '--write'in sys.argv:path.write_text(json.dumps(result,indent=2)+'\n')
 else:assert result==json.loads(path.read_text())
 print('passed',len(result['cases']),'native7757F0 caller rows')
