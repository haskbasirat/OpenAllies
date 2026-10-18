"""Real5A5410/5AA6F0/5AA290/96CC4C/59D240/405030 and59CD10.
Heap/CString/allocator-pool/critical-section/atexit transports are explicit;
the cache list starts via real59CD10, not an observed empty-BSS inference.
Model/Sequence root pointer words are explicit transport inputs, not decoded
NIF/KF objects. Fixed pool capacity is replaced by growable host storage.
"""
from pathlib import Path
import sys,json,hashlib,gc
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set();events=[]
 def capture(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,capture)
 guardvt=m.alloc(12)
 m.w(guardvt+4,0x6a71d0);m.w(guardvt+8,0x6a71d0)
 def guard(m,u):m.w(u.reg_read(UC_X86_REG_ECX),guardvt);m.ret(u.reg_read(UC_X86_REG_ECX))
 m.stubs[0x857d60]=guard
 def pool(m,u):events.append('pool');p=m.alloc(0x3c);m.u.mem_write(p,b'\xa5'*0x3c);m.ret(p)
 m.stubs[0x85c680]=pool
 manager=m.alloc(0x28);m.u.mem_write(manager,b'\xa5'*0x28);m.thiscall(0x59cd10,manager,[])
 assert m.r(0xb3d7a0)==manager and m.r(manager+0x20)==0 and m.r(manager+0x24)==0
 rows=[];prior=0
 for i,(size,flags,name,sequence)in enumerate([(0,0,'a/Name.NIF',False),(1,0,'ABC.nif',False),(5,0,'aBc.nif',True),(16777217,0x400,'art\\GERCorps.nif',False),(0x7fffffff,0,'x.nif',False),(0x80000001,0,'z.nif',True),(0xffffffff,0,'日本.nif',False),(123,0x100,'skip.nif',False)]):
  resource=m.alloc(0x90);m.w(resource,0xa45290 if sequence else 0xa45c98);m.w(resource+8,flags);m.w(resource+0xc,size);m.w(resource+0x10,size)
  root=m.alloc(4);m.w(resource+0x40,root);m.w(resource+0x38,m.cstring(name));before=len(events)
  m.thiscall(0x5a5410,resource,[]);entry=m.r(resource+0x3c)
  if flags&0x100:
   assert entry==0 and len(events)==before
   rows.append(dict(pc=pc,bytes=size,flags_before=flags,flags_after=m.r(resource+8),name=name,sequence=sequence,entry=False));continue
  assert entry and m.r(entry)==root and m.r(entry+8)==1 and m.r(entry+0x18)==prior and m.r(entry+0x1c)==0
  if prior:assert m.r(prior+0x1c)==entry
  else:assert m.r(manager+0x20)==entry
  assert m.r(manager+0x24)==entry and m.r(entry+0x30)==43 and m.r(entry+0x34)==0 and m.u.mem_read(entry+0x38,1)==b'\1'
  assert bytes(m.u.mem_read(m.r(entry+0x2c),172))==bytes(172)
  rows.append(dict(pc=pc,bytes=size,flags_before=flags,flags_after=m.r(resource+8),name=name,sequence=sequence,entry=True,cache_size=m.r(entry+4),filename=m.wstr(m.r(entry+0xc)),hash=m.r(entry+0x10),tail_is_same=True,previous_is_prior=True,zero_buckets=43,state_byte=1))
  prior=entry
 m.check_null();return rows,pins,seams
def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.extend(a);pins.update(pa);seams|=sa|sb
 out=dict(scope=__doc__,rows=rows,executed={hex(a):b for a,b in sorted(pins.items())},intercepted=[hex(a)for a in sorted(seams)])
 data=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'native-ni-cache.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(paired_rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
