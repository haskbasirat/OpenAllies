"""Actual5A19C0 Header pair producer callsite5A1A3F..5A1A5A,
including both actual49AD70 bodies in key-only mode. Frame is explicit:
ESI=stream, EDI=Header, initialized writable stack; no fullHeader claim.
Unique lookup/CString byte reader transports are named. Missing-key fallback
stops at the actual lookup boundary, preserving earlier key/text effects.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_key_only_pair import PairMachine,Missing,STREAM_MODE,PE0,PE1,PC
from unicorn.x86_const import *
class Machine(PairMachine):
 def hook(self,u,pc,size,user):
  if getattr(self,'pair_load',False)and pc==0x41d290:
   self.pair=self.arg(1);index=(self.pair-self.header-0x20)//8
   assert index in [0,1];self.key,self.value=self.values[index]
   self.answer=self.alloc(4);self.cs(self.answer,self.value or'')
  super().hook(u,pc,size,user)
def run(image,pc,values):
 m=Machine(image);m.fpu(pc);m.actual={};m.record_actual=True;m.labels_ready=False;m.pair_load=True;m.events=[]
 m.values=values;m.header=m.alloc(0x68);m.stream=m.alloc(0x80);vt=m.alloc(0x50);m.w(m.stream,vt);m.w(vt+0x48,STREAM_MODE);m.u.mem_write(STREAM_MODE,b'\xc3')
 for off in [0x20,0x28]:m.cs(m.header+off,'old-key');m.cs(m.header+off+4,'old-text')
 m.u.reg_write(UC_X86_REG_ESI,m.stream);m.u.reg_write(UC_X86_REG_EDI,m.header);m.u.reg_write(UC_X86_REG_ESP,m.stack);m.w(m.stack+0x20,7)
 error=None
 try:m.u.emu_start(0x5a1a3f,0x5a1a5a,count=10000)
 except Missing:error='required missing-key fallback boundary'
 if error is None:assert m.u.reg_read(UC_X86_REG_EIP)==0x5a1a5a
 return dict(pairs=[dict(key=m.rs(m.header+o),text=m.rs(m.header+o+4))for o in [0x20,0x28]],error=error,events=m.events),m.actual

def main():
 values=[[('',None),('',None)],[('Name','A\0B'),('Desc','\u03a9')],[('Name',''),('Desc','Text')],[('missing',None),('Desc','Unused')],[('Name','Kept'),('missing',None)]]
 rows=[];pins={}
 for pc,case in itertools.product(PC,values):
  a,pa=run('original',pc,case);b,pb=run('v14',pc,case);assert a==b;rows.append(dict(precision=pc,input=case,output=a));pins.update(pa);pins.update(pb)
 instructions=[]
 for va,n in sorted(pins.items()):
  if not PE0.base<=va<PE0.base+PE0.image_size:continue
  raw=PE0.range(va,n);assert raw==PE1.range(va,n);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 data=(json.dumps(dict(scope=__doc__,rows=rows,executions=len(rows)*2,instructions=instructions),indent=1)+'\n').encode();path=HERE/'native-header-pairs.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(instructions),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
