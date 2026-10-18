"""Actual9D0680+9D06E0/409750 descriptor registration,9D0900 NIF table,
Model5A7EC0/5A7ED0 and59CB00/59C360/59C040 class creation/cached lookup.
Valid empty fixture type/class tables are not observed application registries.
ASCII->counted CString,CRT%s,current Int lookup,atexit and inherited heap
are explicit transports. Zero fixture budgets avoid eviction, not native
startup budget observations. Product policy deliberately removes RAM caps.
No NIF decode,Task loaded/readiness or model rendering is executed.
"""
from pathlib import Path
import sys,json,hashlib,gc
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()

def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set();rows=[]
 def ascii_name(m,u):
  p=m.r(u.reg_read(UC_X86_REG_ESP)+4);raw=bytearray()
  while m.u.mem_read(p,1)!=b'\0':raw.extend(m.u.mem_read(p,1));p+=1
  assert raw in [b'KKC_NiResource',b'KKC_NiAVObjResource'];out=u.reg_read(UC_X86_REG_ECX);m.w(out,m.cstring(raw.decode('ascii')));m.ret(out,pop=4)
 m.stubs[0x4094f0]=ascii_name;m.stubs[0x96cab0]=lambda m,u:m.ret()
 def capture(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,capture)
 #One valid empty TypeManager, actual registration assigns its ordinal.
 m.w(0xb38e08,0);m.w(0xb38e0c,0);m.w(0xb38e10,2)
 m.w(0xb38e14,m.alloc(31*4));m.w(0xb38e18,31);m.w(0xb38e1c,0)
 m.cdecl(0x9d0680,[]);m.cdecl(0x9d06e0,[])
 descriptor=dict(class_id=m.r(0xb3d990),name=m.wstr(m.r(0xb3d994)),base=hex(m.r(0xb3d998)),factory=hex(m.r(0xb3d9a0)),manager_count=m.r(0xb38e0c))
 assert descriptor==dict(class_id=1,name='KKC_NiAVObjResource',base='0xb3d968',factory='0x5a84c0',manager_count=2)
 m.cdecl(0x9d0900,[]);extensions=[]
 for i in range(21):
  name=m.raw_wstr(m.r(0xb282f8+8*i))
  if name in ['NIF']:extensions.append(dict(extension=name,descriptor=hex(m.r(0xb282fc+8*i))))
 assert extensions==[dict(extension='NIF',descriptor='0xb3d990')]
 weight=[None];lookups=[];intword=m.alloc(4);intptr=m.alloc(4);m.w(intptr,intword)
 def fmt(m,u):
  sp=u.reg_read(UC_X86_REG_ESP);out=m.r(sp+4);pattern=m.raw_wstr(m.r(sp+8));arg=m.wstr(m.r(sp+12))
  assert pattern=='ResourceCache%sRAMWeight'and arg=='NiAVObj';m.w(out,m.cstring('ResourceCacheNiAVObjRAMWeight'));m.ret(out)
 def lookup(m,u):
  name=m.raw_wstr(m.r(m.r(u.reg_read(UC_X86_REG_ESP)+4)));assert name=='ResourceCacheNiAVObjRAMWeight';lookups.append(name)
  m.w(intword,weight[0]if weight[0]is not None else 0);m.ret(intptr if weight[0]is not None else 0,pop=4)
 m.stubs[0x405d60]=fmt;m.stubs[0x405370]=lookup
 for current in[None,1,3,10,25,30,2147483647]:
  cache=m.alloc(0x48);m.w(cache+0x20,m.alloc(23*4));m.w(cache+0x24,23);m.u.mem_write(cache+0x2c,b'\1')
  resource=m.alloc(0x90);m.u.mem_write(resource,b'\xa5'*0x90);m.w(resource,0xa45c98);before=bytes(m.u.mem_read(resource,0x90))
  weight[0]=current;lookups.clear();klass=m.thiscall(0x59cb00,cache,[resource])['eax'];first=len(lookups)
  expected=10 if current is None else current
  assert m.r(klass+0x14)==expected and m.r(klass+0x24)==10 and m.r(cache+0x28)==1
  words=[m.r(klass+4*i)for i in range(13)];weight[0]=999
  again=m.thiscall(0x59cb00,cache,[resource])['eax'];assert again==klass and len(lookups)==first and[m.r(klass+4*i)for i in range(13)]==words
  assert before==bytes(m.u.mem_read(resource,0x90))
  rows.append(dict(precision=pc,ram_current=current,class_name=m.wstr(m.r(klass)),class_id=m.r(klass+4),ram_weight=m.r(klass+0x14),vram_weight=m.r(klass+0x24),ram_total=m.r(cache+0x38),vram_total=m.r(cache+0x44),class_count=m.r(cache+0x28),lookup_count=first,cached_lookup_count=len(lookups)-first,cached_same_allocation=again==klass,resource_bytes_unchanged=True))
 m.check_null();return dict(descriptor=descriptor,extensions=extensions,rows=rows),pins,seams

def main():
 rows=[];registrations=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.extend(a['rows']);registrations.append(dict(precision=pc,descriptor=a['descriptor'],extensions=a['extensions']));pins.update(pa);seams|=sa|sb
 for a,b in pins.items():assert h.P.range(a,len(bytes.fromhex(b)))==h.V.range(a,len(bytes.fromhex(b)))==bytes.fromhex(b)
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__)),rows=rows,registrations=registrations,executed={hex(a):b for a,b in sorted(pins.items())},intercepted_not_executed=[hex(a)for a in sorted(seams)])
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-model-class.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(paired_class_rows=len(rows),class_calls=4*len(rows),descriptor_and_table_runs=6,executed=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
