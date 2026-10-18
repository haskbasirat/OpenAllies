"""Bounded704350 process renderer child+54 constructor through its+24=0
store, and exact7040E0 begin byte write. The parent ctor70EF50 call/store and
global singleton writes are pinned separately, not claimed as whole renderer
construction. One explicit engine-new transport provides first128-byte array;
the constructor stops before its next allocation/material child. This proves
only the retained scalar owner mapping in the modern SceneRenderer, not Ni
allocation/resource readiness or ApplicationLoading/AD.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_CODE
ORIGINAL=h.P
class Cutoff(Exception):pass
def run_ctor(image,pc):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);p=m.alloc(0x28)
 m.u.mem_write(p,b'\xa5'*0x28);calls=[]
 def allocate(x):
  assert x.arg()==0x80;calls.append(128);x.ret(value=x.alloc(128))
 m.seams[0x6f7810]=allocate
 def observe(u,a,n,data):
  if a==0x7043c6:raise Cutoff()
 m.u.hook_add(UC_HOOK_CODE,observe)
 try:m.call(0x704350,p)
 except Cutoff:pass
 assert m.r8(p+0x24)==0 and m.r32(0xb400dc)==p and calls==[128]
 return {'active':m.r8(p+0x24),'singleton_is_same':m.r32(0xb400dc)==p,'allocated':[128]}
def run_begin(image,pc,before):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);p=m.alloc(0x28)
 m.u.mem_write(p,b'\xa5'*0x28);m.w8(p+0x24,before)
 m.call(0x7040e0,p);assert m.r8(p+0x24)==1
 assert m.u.mem_read(p+0x23,1)==b'\xa5'and m.u.mem_read(p+0x25,1)==b'\xa5'
 return {'active':1,'neighbor_bytes':[165,165]}
def main():
 rows=[]
 for image,pc in itertools.product(('original','v14'),h.PRECISION):
  rows.append({'image':image,'precision':pc,'operation':'ctor_prefix','output':run_ctor(image,pc)})
  for before in (0,1,128,255):rows.append({'image':image,'precision':pc,'operation':'begin','before':before,'output':run_begin(image,pc,before)})
 seams={0x6f7810,0x7043c6};pins=[]
 for a,b in sorted(h.VISITED.items()):
  if a in seams:continue
  assert bytes.fromhex(b)==ORIGINAL.range(a,len(bytes.fromhex(b)))==h.V.range(a,len(bytes.fromhex(b)))
  pins.append({'va':hex(a),'bytes':b})
 metadata=[]
 for a,n in [(0x70f32a,0x70f35c-0x70f32a)]:
  raw=ORIGINAL.range(a,n);assert raw==h.V.range(a,n);metadata.append({'va':hex(a),'bytes':raw.hex(),'executed':False})
 doc={'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
      'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'cases':rows,'executed_pins':pins,'metadata_only_parent_pins':metadata,
      'allocator_transport_not_executed':'0x6f7810','required_cutoff_before_body':'0x7043c6'}
 path=HERE/'native-view-screen.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8',newline='\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'rows':len(rows),'genuine_executed_pins':len(pins)}))
if __name__=='__main__':main()
