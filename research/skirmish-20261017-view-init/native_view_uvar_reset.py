"""Actual43F156..43F1B2 indexed213-variable reset loop,4B2EB0 indexed
lookup,4B2840 reset and76AC90 real Session+F8 getter. Metadata is the separate
actual constructor capture. Current/default bytes are explicit fixture inputs.
Changing F8 at getter entry is external storage intervention to test lazy reads,
not a claim of an original application callback. Required cases stop at the
getter boundary before its body and preserve earlier reset stores. No whole
VWorld/resource/UI completion claim.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP
ORIGINAL=h.P
METADATA=json.loads((HERE/'native-uvar-registration.json').read_text())['cases'][0]['registrations']
TARGETS=[r['index']for r in METADATA if r['kind']==1 and r['flags']&5==5 or r['index']==44]
class Stop(Exception):pass

def run(image,pc,c):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
 manager=0xb39938;array=m.alloc(213*4);m.w32(manager+8,array)
 cells={};events=[];reads=0;stop=None
 for r in METADATA:
  p=m.alloc(0x2c);m.w32(array+4*r['index'],p);cells[r['index']]=p
  m.w32(p+0x24,r['kind'],r['flags']);m.w8(p,c['before']);m.w8(p+1,c['default'])
 session=m.alloc(0x100);m.w32(0xb36550,session)
 def observer(u,a,n,data):
  nonlocal reads,stop
  if a==0x4b2eb0:events.append(['lookup',m.arg()])
  elif a==0x4b2840:
   index=next(i for i,p in cells.items()if p==m.ecx());events.append(['reset',index])
  elif a==0x76ac90:
   reads+=1;events.append(['phase',reads])
   if c['fail_read']==reads:stop='phase_required';raise Stop()
   value=c['phases'][min(reads-1,len(c['phases'])-1)];m.w32(session+0xf8,value)
  elif a==0x43f1b2:raise Stop()
 m.u.hook_add(UC_HOOK_CODE,observer)
 #call() resets nonvolatiles. Use a fresh input shim to establish EBP=0.
 shim=m.stub_next;m.stub_next+=32
 m.u.mem_write(shim,b'\x31\xed\xb8'+(0x43f156).to_bytes(4,'little')+b'\xff\xe0')
 try:m.call(shim)
 except Stop:pass
 return {'current':[m.r8(cells[i])for i in TARGETS],'default':[m.r8(cells[i]+1)for i in TARGETS],
  'events':events,'phase_reads':reads,'stop':stop}

def cases():
 for before in [0,1,128,255]:
  for phases in [[7],[5,5],[0,5],[0,7],[0,0]]:
   yield {'before':before,'default':0,'phases':phases,'fail_read':None}
 for fail in [1,2]:yield {'before':255,'default':128,'phases':[0,0],'fail_read':fail}
 for default in [1,128,255]:
  for phases in [[7],[0,0]]:yield {'before':128,'default':default,'phases':phases,'fail_read':None}

def main():
 rows=[]
 for pc,c in itertools.product(h.PRECISION,list(cases())):
  a=run('original',pc,c);b=run('v14',pc,c);assert a==b
  rows.append({'precision':pc,'input':c,'output':a})
 #The interval stop is observed before body execution, so exclude it.
 pins=[]
 for a,b in sorted(h.VISITED.items()):
  if a==0x43f1b2:continue
  raw=bytes.fromhex(b);assert raw==ORIGINAL.range(a,len(raw))==h.V.range(a,len(raw))
  pins.append({'va':hex(a),'bytes':b})
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'metadata_sha256':hashlib.sha256((HERE/'native-uvar-registration.json').read_bytes()).hexdigest(),
  'mutable_indices':TARGETS,'cases':rows,'executed_instructions':pins}
 p=HERE/'native-view-uvar-reset.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'executions':2*len(rows),'pins':len(pins),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
