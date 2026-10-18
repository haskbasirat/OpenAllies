"""Actual49AD70 key-only localized-pair current-value producer.

Serialized CString reads, existing unique table answers and missing-lookup
cutoffs are named seams. Stream virtual48=true and59E520=false select the
key-only translation branch. Empty keys execute the real unconditional empty
CString assignment. No missing-entry fallback/diagnostic success is invented.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_EIP
STREAM_MODE=0x7700f00
class Missing(Exception):pass
class PairMachine(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'pair_load',False):
   if pc==0x41d290:
    assert self.arg()==self.stream
    self.cs(self.arg(1),self.key);self.events.append('read_key');self.ret(value=self.stream);return
   if pc==STREAM_MODE:self.events.append('stream48_true');self.ret(value=1);return
   if pc==0x59e520:self.events.append('59E520_false');self.ret(value=0);return
   if pc==0x4b9f40:
    assert self.arg()==self.pair and self.arg(1)==0xb43b48
    self.events.append('lookup_unique'if self.value is not None else'missing_lookup_cutoff')
    if self.value is None:raise Missing()
    self.ret(8,self.answer);return
  super().hook(u,pc,size,user)
def main():
 rows=[];pins={}
 cases=[('',None),('Key','translated'),('Key','\u03a9'),('Key','A\0B'),('Key',''),('missing',None)]
 for image in ['original','v14']:
  m=PairMachine(image)
  for pc in PC:
   for key,value in cases:
    m.fpu(pc);m.actual={};m.record_actual=True;m.labels_ready=False;m.pair_load=True;m.events=[];m.key=key;m.value=value
    m.stream=m.alloc(0x80);vt=m.alloc(0x50);m.w(m.stream,vt);m.w(vt+0x48,STREAM_MODE)
    m.u.mem_write(STREAM_MODE,b'\xc3') # bounded transport stub, never a native pin
    m.pair=m.alloc(8);m.cs(m.pair,'old-key');m.cs(m.pair+4,'old-text');m.answer=m.alloc(4);m.cs(m.answer,value or'')
    error=None
    try:m.call(0x49ad70,[m.stream,m.pair])
    except Missing:error='required missing-key fallback boundary'
    except Exception:
     print('failed',image,pc,key,repr(value),hex(m.u.reg_read(UC_X86_REG_EIP)),m.events,flush=True)
     raise
    rows.append({'image':image,'precision':pc,'key':key,'unique_text':value,'output':{'key':m.rs(m.pair),'text':m.rs(m.pair+4),'error':error,'events':m.events.copy()}})
    pins.update(m.actual)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append({'va':hex(va),'bytes':raw.hex()})
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'rows':rows,'instructions':instructions}
 out=HERE/'native-key-only-pair.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps({'rows':len(rows),'pins':len(instructions),'sha256':hashlib.sha256(out.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
