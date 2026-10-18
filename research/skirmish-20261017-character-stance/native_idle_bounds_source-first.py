"""Actual49E93F..49E965 two required View integer getter/store prefix.
Actual5B4ED0/integer virtual10 execute. Typed source-node lookup is an
explicit input service; source inheritance/parser and whole49DC40 are not
claimed. The enclosing View lookup/ESI ownership is pinned separately.
"""
from pathlib import Path
import sys,json,hashlib,struct,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn.x86_const import *
P,V=h.P,h.V
class Machine(h.Machine):
 def __init__(self,c,pc):
  super().__init__(b'',pc);self.dest=self.alloc(0x200,0xa5);self.node=self.alloc(0x58);self.events=[]
  fields={}
  for key,value in [('idle_loops_min',c['min']),('idle_loops_max',c['max'])]:
   field=self.alloc(0x14);self.w(field,0xa487c8);self.w(field+0x10,value);fields[key]=field
  def lookup(m):
   assert m.u.reg_read(UC_X86_REG_ECX)==m.node;name=m.read_wide(m.arg(0));m.events.append(name)
   assert name in fields;m.ret(4,0 if c['missing']==name else fields[name])
  def stop(m):self.outcome='complete';raise h.Throw()
  def missing(m):self.outcome='required integer field missing';raise h.Throw()
  self.seam(0x5b4750,lookup);self.probes[0x49e965]=stop;self.probes[0x5b4f03]=missing
 def run(self):
  self.u.reg_write(UC_X86_REG_ESP,h.STACK);self.u.reg_write(UC_X86_REG_EBP,self.dest);self.u.reg_write(UC_X86_REG_ESI,self.node)
  try:self.u.emu_start(0x49e93f,h.STOP,count=100000)
  except h.Throw:pass
  return dict(outcome=self.outcome,min_word=self.r(self.dest+0x12c),max_word=self.r(self.dest+0x130),lookups=self.events)
def main():
 rows=[];pins={};cases=[dict(min=a,max=b,missing=None)for a,b in [(1,3),(-3,7),(2147483647,-2147483648)]]+[dict(min=1,max=3,missing=k)for k in ['idle_loops_min','idle_loops_max']]
 for pc,c in itertools.product(h.CW,cases):
  outputs=[]
  for image in [P,V]:
   h.P=image;m=Machine(c,pc);out=m.run();outputs.append(out)
   for a,b in h.VISITED.items():
    if a not in [0x5b4750,0x49e965,0x5b4f03]and P.base<=a<P.base+P.image_size:pins[a]=bytes.fromhex(b)
  assert outputs[0]==outputs[1];rows.append(dict(precision=pc,input=c,output=outputs[0]))
 h.P=P
 for a,b in pins.items():assert P.range(a,len(b))==V.range(a,len(b))==b
 #Actual enclosing getter obtains child View and retains its pointer in ESI.
 #The authoritative decompile and raw preceding block accompany this leaf;
 #no transfer of an actor-root source pointer is authorized by the fixture.
 doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,executions=len(rows)*2,instructions=[dict(va=hex(a),bytes=b.hex())for a,b in sorted(pins.items())],typed_integer_vtable=dict(va='0xa487c8',bytes=P.range(0xa487c8,0x14).hex()),source_path='View.idle_loops_min / View.idle_loops_max',observed_cutoffs=['0x49e965','0x5b4f03'],seam='0x5b4750 typed node lookup')
 raw=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-idle-bounds-source.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
