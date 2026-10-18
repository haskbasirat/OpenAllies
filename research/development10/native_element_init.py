"""Whole5704D0 and actual4CF110/529C30/45D5B0/5014E0 over explicit current fields.
Positive50AC60 stops at entry after the actual rate store. No queue success seam.
"""
from pathlib import Path
import sys,json,hashlib,random
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'phase8-ai'))
import native_machine as nm
from unicorn import UC_HOOK_MEM_READ
ORIGINAL=nm.original
class Required(Exception):pass

def inputs():
 out=[]
 for i,(base,mul) in enumerate([(0,0x3f800000),(0x3f800000,0x3f800000),(0x3eaaaaab,0x3faaaaab),(0x41200000,0x3e800000),(0x00800000,0x3f000001),(0x800000,0x3f000000),(0xbf800000,0x3fc00000),(0x3f800001,0x3f7fffff)]):
  for context in [False,True]:
   for version,pending in [(21,False),(21,True),(22,False),(27,False)]:
    out.append(dict(name=f'rate{i}_{context}_{version}_{pending}',base=base,multiplier=mul,context=context,version=version,pending=pending,prior=0xdeadbeef))
 return out

def run(c,image,pc):
 nm.original=ORIGINAL if image=='original' else nm.modified
 m=nm.Machine(pc);actor=m.alloc(0x200);definition=m.alloc(0x600);element=m.alloc(0x80);world=m.alloc(0x300)
 m.w(actor+4,definition);m.w(element+4,actor);m.w(element+0xc,0x77777770 if c['pending'] else 0);m.w(element+0x20,c['prior']);m.w(definition+0x2cc,c['base']);m.w(0xb36534,world);m.w(world+0x2c,c['version'])
 if c['context']:
  props=m.alloc(0xe0);m.w(actor+0x78,props);m.w(props+0x1c+0x3c,c['multiplier'])
 headreads=[];calls=[]
 def read(u,access,address,size,value,arg):
  if address==element+0xc:headreads.append(size)
 m.u.hook_add(UC_HOOK_MEM_READ,read)
 def required(z):calls.append(dict(this_offset=z.ecx()-element,data=z.arg(1)));raise Required()
 m.seams[0x50ac60]=required
 try:
  value=m.call(0x5704d0,element);assert value&255==1;outcome='complete'
 except Required:outcome='required50AC60'
 result=dict(rate=m.r(element+0x20),outcome=outcome,pending_reads=headreads,queue_calls=calls)
 #Actual no-op AfterInit is proved independently even when the init prefix stops.
 m.call(0x4b2830,element)
 return result,m.visited

def main():
 rows=[];pins={}
 for c in inputs():
  for pc in nm.PRECISIONS:
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
   for pp in (pa,pb):
    for va,raw in pp.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw;pins[va]=raw
   rows.append(dict(input=c,precision=pc,native=a))
 doc=dict(schema=1,scope=__doc__,rows=rows,native_runs=len(rows)*2,instructions=[dict(va=hex(v),bytes=b.hex()) for v,b in sorted(pins.items())])
 path=HERE/'native-element-init.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8')
 else:assert json.loads(path.read_text(encoding='utf-8'))==doc
 print(json.dumps(dict(rows=len(rows),native_runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
