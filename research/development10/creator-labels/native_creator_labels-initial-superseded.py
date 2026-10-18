"""Actual50DC00 first-match lookup and49AE00 same-pair/table writes.

CString storage/allocation and4BAA20's existing-entry lookup are explicit
transport seams. This does not claim missing-entry creation, full parameter
loading, native allocation/exception parity or773E30 activation. All executed
nonseam bytes are pinned in original/v14 atPC24/53/64.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-local-bootstrap'))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ECX

class LookupStop(Exception):pass
class Machine(BootstrapMachine):
 def hook(self,u,pc,size,user):
  if getattr(self,'labels_ready',False)and pc==0x4baa20:
   assert self.arg(1)==self.pair and self.arg(2)==1
   self.trace.append({'lookup':'4BAA20','pair_before':self.rs(self.pair+4),'key':self.rs(self.pair)})
   if self.fail:raise LookupStop()
   self.w(self.arg(),self.entry);self.w(self.entry+4,self.r(self.entry+4)+1)
   self.ret(16,self.arg());return
  super().hook(u,pc,size,user)

def run(image,pc,ids,query,key,supplied,fail):
 m=Machine(image);m.fpu(pc);m.visited={};m.trace=[]
 parameters=m.alloc(0x20);array=m.alloc(max(1,len(ids))*0x6c)
 m.w(parameters+0x10,array);m.w(parameters+0x14,len(ids))
 for i,name in enumerate(ids):
  m.cs(array+i*0x6c,name);m.cs(array+i*0x6c+4,key);m.cs(array+i*0x6c+8,'old-pair');m.cs(array+i*0x6c+0xc,'old-glyph')
 arg=m.alloc(4);m.cs(arg,query)
 found=m.call(0x50dc00,[arg],this=parameters,callee_pop=4)
 index=None if found==0 else (found-array)//0x6c
 assert found==0 or found==array+index*0x6c
 out={'found_index':index,'pair':None,'global':None,'error':None,'trace':[]}
 if found:
  m.pair=found+4;m.entry=m.alloc(0x14);m.w(m.entry,0);m.w(m.entry+4,100);m.cs(m.entry+0x10,'old-global')
  m.fail=fail;m.labels_ready=True;value=m.alloc(4);m.cs(value,supplied)
  try:m.call(0x49ae00,[value],this=m.pair,callee_pop=4)
  except LookupStop:out['error']='required existing-entry lookup cutoff'
  out.update(pair=m.rs(found+8),global_text=m.rs(m.entry+0x10),trace=m.trace)
 return out,{a:size for a,size in m.visited.items()if a!=0x4baa20}

def main():
 cases=[(['K\0first','K','k'],'K\0query','MapKey','A\0B',False),
        (['K','k'],'k','MapKey','\u03a9',False),(['K'],'missing','MapKey','unused',False),
        ([], 'K','MapKey','unused',False),([''],'','MapKey','Empty-ID',False),
        (['K'],'K','','empty-key',False),(['K'],'K','MapKey','prefix-survives',True),
        (['K'],'K','MapKey','',False),(['\u03a9'],'\u03a9','MapKey','\ud800\udfff',False)]
 rows=[];pins={}
 for image in ['original','v14']:
  for pc in PC:
   for ids,query,key,supplied,fail in cases:
    output,visited=run(image,pc,ids,query,key,supplied,fail);pins.update(visited)
    rows.append({'image':image,'precision':pc,'input':{'ids':ids,'query':query,'key':key,'supplied':supplied,'fail_lookup':fail},'output':output})
 a=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='original']
 b=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size)
   instructions.append({'va':hex(va),'bytes':raw.hex()})
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'rows':rows,'instructions':instructions}
 out=HERE/'native-creator-labels.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps({'rows':len(rows),'pins':len(instructions),'sha256':hashlib.sha256(out.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
