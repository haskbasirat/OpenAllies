"""Actual73A560 ordered model-variant predicate and739D10 field publication.

Predicate runs have no successful seams. Constructor field-only rows explicitly
intercept base NiShader8A5130 and name8A68E0; they do not certify shader/device
initialization or the later73B020 shader pair. Both images/all3PC.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn import UC_HOOK_MEM_READ
class Policy(Machine):
 def __init__(self,image):
  self.armed=False;self.constructor=False;super().__init__(image);self.u.hook_add(UC_HOOK_MEM_READ,self.read)
 def read(self,u,access,address,size,value,user):
  if self.armed:
   for at,name in self.observed.items():
    if address<=at<address+size:self.reads.append(name)
 def hook(self,u,pc,size,user):
  if self.constructor and pc in [0x8a5130,0x8a68e0]:
   self.actual.pop(pc,None);self.ret(4 if pc==0x8a68e0 else 0,self.ecx());return
  super().hook(u,pc,size,user)
def main():
 rows=[];constructors=[];pins={}
 for image in ['original','v14']:
  m=Policy(image)
  for precision in PC:
   for skinned in [0,1,128]:
    m.armed=False;m.constructor=True;m.fpu(precision);receiver=m.alloc(0x1a0);m.w(0xb40690,0);m.w(0xb40694,0)
    m.actual={};m.record_actual=True;m.call(0x739d10,[skinned],this=receiver,callee_pop=4);pins.update(m.actual)
    constructors.append(dict(image=image,precision=precision,skinned=skinned,receiver191=m.u.mem_read(receiver+0x191,1)[0],lit128=m.r(receiver+0x128),unlit12c=m.r(receiver+0x12c),published_skinned=m.r(0xb40690)==receiver,published_unskinned=m.r(0xb40694)==receiver))
   m.constructor=False
   for skin,compat,uvar,lit,unlit in itertools.product([0,1,128],[0,1,128],[0,1,128],[0,1],[0,1]):
    m.armed=False;m.fpu(precision);receiver=m.alloc(0x1a0);manager=m.alloc(0x288);table=m.alloc(0x200);variable=m.alloc(4)
    m.b(receiver+0x191,skin);m.b(manager+0x275,compat);m.b(variable,uvar);m.w(receiver+0x128,0x7700000 if lit else 0);m.w(receiver+0x12c,0x7700100 if unlit else 0)
    m.w(0xb36510,manager);m.w(0xb39940,table);m.w(table+0x1c4,variable)
    m.observed={receiver+0x191:'receiver191',manager+0x275:'compat275',variable:'uvar113',receiver+0x128:'lit128',receiver+0x12c:'unlit12c'};m.reads=[]
    m.actual={};m.record_actual=True;m.armed=True;result=m.call(0x73a560,this=receiver);pins.update(m.actual)
    rows.append(dict(image=image,precision=precision,input=dict(receiver191=skin,compat275=compat,uvar113=uvar,lit128=lit,unlit12c=unlit),output=dict(result=result,reads=m.reads)))
 for collection in [rows,constructors]:assert [{k:v for k,v in r.items()if k!='image'}for r in collection if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in collection if r['image']=='v14']
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 #Actual5A6FD0 always chooses the current skinned-family receiver.
 call=PE0.range(0x5a6fef,11);assert call==PE1.range(0x5a6fef,11)==bytes.fromhex('8b0d9006b400e866351900')
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,constructors=constructors,instructions=instructions,current_receiver_call=dict(va='0x5a6fef',bytes=call.hex()))
 out=HERE/'native-model-policy.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),constructors=len(constructors),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
