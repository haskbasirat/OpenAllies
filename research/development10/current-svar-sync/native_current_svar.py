"""Whole76D820 with actual registered231 names/types/order and explicit
laboratory current words. Typed logger arguments are observed at entry,
then their actual disabled bodies and String copy/wrapper/destruction execute.
This proves native traversal/order,
raw Boolean normalization, Point3 field reloads and exact error prefixes,
not observed current values, logger backend parity or a playable battle.
The injected mutation is an explicit storage-intervention probe, not an
application callback discovered at the native scalar load instruction.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/development10/world-ready-7757f0'))
from native_svar_inactive import Machine as Base,inventory,base,SYNC_A
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_ESI,UC_X86_REG_EAX
class Cutoff(Exception):pass
LOGGERS={0x77dc50:('marker',8),0x77da70:('int',12),0x77d930:('float',12),0x77d9d0:('bool',12),0x77db10:('text',12)}
class Machine(Base):
 def hook(self,u,pc,size,user):
  if getattr(self,'normalizing',False):
   if pc==0x77da32:raise Cutoff()
   if 0x77da17<=pc<0x77da32:self.actual[pc]=size;return
  if self.in_dump and pc in LOGGERS:
   if self.cutoff==len(self.events):raise Cutoff()
   kind,_=LOGGERS[pc];label=self.text(self.arg());value=0 if kind=='marker'else self.arg(1)
   if kind=='text':
    slot=self.alloc(4);self.w(slot,value);value=self.rs(slot)
   elif kind=='bool':value=(value&255)!=0
   self.events.append(dict(kind=kind,label=label,value=value))
   if self.mutate and label==self.text(0xa938c8):
    self.w(self.point+8,0x7fc01234)
   #Continue through the actual disabled logger and its CString destructor.
  super().hook(u,pc,size,user)
 def normalize(self,precision,value):
  #Bounded77D9D0 post-allocation prefix; EAX is the explicit logger-entry
  #allocation returned by77D680, and ESP matches the native frame here.
  self.fpu(precision);entry=self.alloc(0x18);self.u.mem_write(entry,b'\xa5'*0x18)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.w(self.stack+0xc,0xa93978);self.w(self.stack+0x10,value)
  self.u.reg_write(UC_X86_REG_EAX,entry);self.u.reg_write(UC_X86_REG_ESI,SYNC_A)
  self.normalizing=True
  try:self.u.emu_start(0x77da17,self.stop,count=100)
  except Cutoff:pass
  finally:self.normalizing=False
  return dict(precision=precision,value=value,kind=self.r(entry+4),normalized=self.r(entry+8))
 def execute(self,entries,precision,variant,stop):
  self.fpu(precision);self.begin(1,list(range(16)),0);self.watch=True
  self.w(0xb3655c,SYNC_A);self.b(SYNC_A,0);self.w(SYNC_A+4,0)
  array=self.alloc(len(entries)*4);self.w(0xb39ab0,array);self.w(0xb39ab4,len(entries))
  self.inputs=[];self.point=None;strings=[]
  for e in entries:
   var=self.alloc(0x2c);self.w(array+4*e['ordinal'],var)
   self.cs(var+0x1c,e['name']);self.w(var+0x20,e['word20']);self.w(var+0x24,e['kind'])
   kind=e['kind']
   if kind==1:value=[0,128,255][(e['ordinal']+variant)%3];self.b(var,value)
   elif kind in(2,3):value=[0x80000000,0x7fc01234,0xffffffff,0x3faaaaab][(e['ordinal']+variant)%4];self.w(var,value)
   elif kind==4:
    value=[65,0,66,0x3bb]if variant else [0x3bb,95,98,105,111,109,101]
    self.cs(var+0x10,''.join(chr(v)for v in value));p=self.r(var+0x10);strings.append((p-4,self.r(p-4)))
   else:
    value=[0x3f800000,0x80000000,0x7f800000]
    for i,v in enumerate(value):self.w(var+4+4*i,v)
    if self.point is None:self.point=var
   self.inputs.append(dict(**e,value=value))
  self.mutate=variant==1;self.cutoff=stop;self.events=[];self.actual.clear();self.in_dump=True
  stopped=False
  try:self.call(0x76d820,[0xb39aa8])
  except Cutoff:stopped=True
  finally:self.in_dump=False
  if not stopped:assert all(self.r(a)==n for a,n in strings),'String temporary did not balance'
  return dict(events=self.events,stopped=stopped,string_temporaries_balanced_on_return=not stopped)
def main():
 entries=inventory();rows=[];pins={};normalization=[]
 for precision,variant in itertools.product(base.PC,(0,1)):
  full=None
  for stop in(None,0,1,2,17,230,462):
   pair=[]
   for image in('original','v14'):
    m=Machine(image);out=m.execute(entries,precision,variant,stop)
    pair.append(out);pins.update(m.actual)
    inputs=m.inputs
   assert pair[0]==pair[1]
   if stop is None:full=pair[0]
   else:assert pair[0]['events']==full['events'][:stop]and pair[0]['stopped']
   rows.append(dict(precision=precision,variant=variant,input=inputs,stop=stop,output=pair[0]))
 for image,precision in itertools.product(('original','v14'),base.PC):
  m=Machine(image);m.actual.clear()
  for value in(0,1,128,255):normalization.append(dict(image=image,**m.normalize(precision,value)))
  pins.update(m.actual)
 doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,
  original_sha256=hashlib.sha256(base.PE0.b).hexdigest(),v14_sha256=hashlib.sha256(base.PE1.b).hexdigest(),
  native_runs=len(rows)*2,normalization_runs=len(normalization),normalization=normalization,executed_windows=base.proof_ranges(pins),logger_backend='actual disabled bodies; entry arguments observed')
 p=HERE/'native-current-svar.json';data=(json.dumps(doc,indent=1)+'\n').encode()
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),runs=len(rows)*2,windows=len(doc['executed_windows']),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
