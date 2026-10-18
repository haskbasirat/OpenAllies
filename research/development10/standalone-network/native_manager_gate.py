"""GameSpy787FE0 publication/BC field prefix and7876F0 current clear.

This is a constructor-field projection only.786C30 lock construction, allocation
and Windows platform calls are declared leaves. No manager worker, account
constructor, services or whole application initialization completion is proved.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));from native_standalone import StandaloneMachine,Stop,PE0,PE1,PC,QPF,QPC
from unicorn.x86_const import UC_X86_REG_ECX,UC_X86_REG_ESP,UC_X86_REG_EIP
GETPROC=QPF+0x20;FLUSH=QPF+0x30
class GateMachine(StandaloneMachine):
 def hook(self,u,pc,size,user):
  if getattr(self,'gate_ready',False):
   if pc==0x786c30:self.trace.append('lock_constructor');self.ret(value=u.reg_read(UC_X86_REG_ECX));return
   if pc==GETPROC:self.trace.append('GetCurrentProcess');self.ret(value=0xffffffff);return
   if pc==FLUSH:
    assert self.arg()==0xffffffff and self.arg(1)==self.manager+0x9c and self.arg(2)==14
    self.trace.append('FlushInstructionCache');self.ret(12,1);return
   if pc in (0x7880f0,0x7876fa):raise Stop()
  super().hook(u,pc,size,user)
def main():
 rows=[];pins={}
 for image in ['original','v14']:
  m=GateMachine(image)
  for address in [QPF,QPC,GETPROC,FLUSH]:m.u.mem_write(address,b'\xc3')
  m.w(0x9f0220,QPF);m.w(0x9f021c,QPC);m.w(0x9f0228,GETPROC);m.w(0x9f0224,FLUSH)
  for pc in PC:
   for initial in [0,1,128,255]:
    m.fpu(pc);m.actual={};m.record_actual=True;m.trace=[];m.ready=True;m.gate_ready=True
    m.allocations={};m.alloc_count=0;m.counter_count=0;m.cutoff=None;m.fail_alloc=None;m.frequency=10_000_000;m.counter=7770000
    m.manager=m.alloc(0xf8);m.u.mem_write(m.manager,b'\xa5'*0xf8);m.b(m.manager+0xbc,initial)
    for va in [0xb3654c,0xb40bd8,0xb40bdc,0xb3d768,0xb3d760,0xb3d764]:m.w(va,0)
    try:m.call(0x787fe0,[0x12345678],this=m.manager,callee_pop=4)
    except Stop:pass
    else:raise AssertionError('constructor prefix cutoff missing')
    assert m.rb(m.manager+0xbc)==0
    out={'present':m.r(0xb3654c)==m.manager,'alias':m.r(0xb40bdc)==m.manager,'global_slot':m.r(0xb40bd8)==0xb3654c,
      'vtable':m.r(m.manager),'bc':m.rb(m.manager+0xbc),'later_c4':m.rb(m.manager+0xc4),'trace':m.trace.copy()}
    rows.append({'image':image,'precision':pc,'kind':'constructor','initial_bc':initial,'output':out});pins.update(m.actual)
    m.fpu(pc);m.actual={};m.b(m.manager+0xbc,initial)
    try:m.call(0x7876f0,this=m.manager)
    except Stop:pass
    else:raise AssertionError('selection prefix cutoff missing')
    assert m.rb(m.manager+0xbc)==0;pins.update(m.actual)
    rows.append({'image':image,'precision':pc,'kind':'select_standalone','initial_bc':initial,'output':{'bc':m.rb(m.manager+0xbc)}})
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append({'va':hex(va),'bytes':raw.hex()})
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'rows':rows,'instructions':instructions}
 path=HERE/'native-manager-gate.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'rows':len(rows),'pins':len(instructions),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
