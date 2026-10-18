"""Exact74D7CB..74D7E5 lazy raw-byte gate and bounded constructor stores.
Entry is after the descriptor decode; handler/cleanup entries are cutoffs.
Constructor slices supply EBX=0 from their declared enclosing ctor prefix;
they prove these stores only, not complete Session/Node construction.
"""
from pathlib import Path
import json,sys,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-local-bootstrap'))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges,NODE
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EIP,UC_X86_REG_EDI,UC_X86_REG_ESI,UC_X86_REG_EBX
class End(Exception):pass
class Machine(BootstrapMachine):
 def __init__(self,image):
  self.proving=False;super().__init__(image);self.proving=True;self.actual={};self.reads=[]
  self.u.hook_add(UC_HOOK_MEM_READ,self.read)
 def read(self,u,kind,address,size,value,user):
  if self.proving and address in(self.S+0x24,NODE+0x2e5):self.reads.append('session'if address==self.S+0x24 else'admin')
 def hook(self,u,pc,size,user):
  if self.proving:
   if pc in(0x74d7e5,0x74d840,0x76faea,0x75d1c0):self.destination=pc;raise End()
   if 0x74d7cb<=pc<0x74d7e5 or pc in(0x76fae7,0x75d1ba):
    self.actual[pc]=size;return
  super().hook(u,pc,size,user)
 def slice(self,start):
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_EDI,NODE)
  try:self.u.emu_start(start,self.stop,count=100)
  except End:pass
 def run(self,precision,session,admin):
  self.fpu(precision);self.S=self.alloc(0x178);self.w(0xb36550,self.S)
  self.b(self.S+0x24,session);self.b(NODE+0x2e5,admin);self.reads=[]
  self.slice(0x74d7cb)
  return dict(precision=precision,session=session,admin=admin,reads=list(self.reads),stopped=self.destination==0x74d840)
 def constructor_stores(self):
  self.u.reg_write(UC_X86_REG_EBX,0);self.u.reg_write(UC_X86_REG_ESI,self.S)
  self.b(self.S+0x24,0xa5);self.slice(0x76fae7);session=self.rb(self.S+0x24)
  self.u.reg_write(UC_X86_REG_ESI,NODE);self.b(NODE+0x2e5,0xa5);self.slice(0x75d1ba)
  return dict(session=session,admin=self.rb(NODE+0x2e5),ebx_input=0)
def main():
 rows=[];ctors=[];pins={}
 for image in('original','v14'):
  m=Machine(image)
  for pc,a,b in itertools.product(PC,(0,1,128,255),(0,1,128,255)):
   rows.append(dict(image=image,**m.run(pc,a,b)));ctors.append(dict(image=image,precision=pc,**m.constructor_stores()));pins.update(m.actual)
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),
  'executed_windows':proof_ranges(pins),'constructors':ctors,'cases':rows}
 p=HERE/'native-order-gate.json'
 if'--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'runs':len(rows),'constructors':len(ctors),'windows':len(doc['executed_windows']),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
