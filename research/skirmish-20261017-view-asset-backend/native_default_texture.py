"""Actual5A6E90 default Texture allocation, real59E060/5ABE50 selected
writable depot,5AAAE0/5AA660/cache transfer and59CB00/59C360 class creation.
Bool/current Int lookup and CString class-format/opaque registryID17 are
explicit fixture transports; real typed getters/ctor/cache/weight bodies run.
Small valid name/class hash tables and zero fixture eviction budgets do not
claim observed manager startup budgets. No image decode/GPU/worker is run.
"""
from pathlib import Path
import sys,json,hashlib,gc
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *

def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set();rows=[];lookups=[];logs=[];weight=[25];warning=[0]
 def inc(m,u):
  p=m.r(u.reg_read(UC_X86_REG_ESP)+4);v=(m.r(p)+1)&0xffffffff;m.w(p,v);m.ret(v,pop=4)
 m.stubs[0x301f750]=inc;m.w(0x9f01e8,0x301f750)
 lockvt=m.alloc(12);m.w(lockvt+4,m.stub(lambda m,u:m.ret()));m.w(lockvt+8,m.stub(lambda m,u:m.ret()))
 boolword=m.alloc(4);boolptr=m.alloc(4);m.w(boolptr,boolword)
 intword=m.alloc(4);intptr=m.alloc(4);m.w(intptr,intword)
 def lookup(m,u):
  esp=u.reg_read(UC_X86_REG_ESP);name=m.raw_wstr(m.r(m.r(esp+4)));lookups.append(name)
  if name=='DataMissingResourceWarning':m.w(boolword,warning[0]);m.ret(boolptr,pop=4)
  else:
   assert name=='ResourceCacheTextureRAMWeight',name;m.w(intword,weight[0]or 0);m.ret(intptr if weight[0]is not None else 0,pop=4)
 m.stubs[0x405370]=lookup
 def fmt(m,u):
  esp=u.reg_read(UC_X86_REG_ESP);out=m.r(esp+4);pattern=m.raw_wstr(m.r(esp+8));arg=m.wstr(m.r(esp+12))
  assert pattern=='ResourceCache%sRAMWeight'and arg=='Texture',(pattern,arg)
  m.w(out,m.cstring('ResourceCacheTextureRAMWeight'));m.ret(out)
 m.stubs[0x405d60]=fmt
 def diagnostic(m,u):
  esp=u.reg_read(UC_X86_REG_ESP);logs.append(m.r(esp+8));m.ret()
 m.stubs[0x404fc0]=diagnostic
 def executed(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,executed);m.thiscall(0x4013e0,0xb43b40,[])
 #Actual9D06B0 ASCII class literal to counted CString transport;17 is opaque class identity
 #transport, not a claim about the original process registration ordinal.
 assert bytes(image.range(0xa46574,20)).split(b'\0',1)[0]==b'KKC_TextureResource'
 m.w(0xb3d980,m.cstring('KKC_TextureResource'));m.w(0xb3d97c,17)
 assert m.wstr(m.r(0xb3d980))=='KKC_TextureResource'
 def box(text):p=m.alloc(4);m.w(p,m.cstring(text));return p
 def manager():
  depot=m.alloc(0x24);m.thiscall(0x59e060,depot,[box('Userdata/'),box('Userdata/'),box(''),1,0]);mgr=m.alloc(0x180);m.w(mgr+0x20,lockvt)
  depots=m.alloc(4);m.w(depots,depot);m.w(mgr+0x84,depots);m.w(mgr+0x88,1);m.thiscall(0x5abe50,mgr,[]);assert m.r(mgr+0x90)==depot
  for off,cap in[(0x60,11),(0x70,7)]:m.w(mgr+off,m.alloc(cap*4));m.w(mgr+off+4,cap)
  cache=mgr+0x9c;m.w(cache+0x20,m.alloc(23*4));m.w(cache+0x24,23);m.w(cache+0x2c,1)
  m.w(0xb36540,mgr);m.w(0xb43b70,0);return mgr,depot
 for mode in [0,1,2,128,255]:
  for ram in [None,7,25]:
   warning[0]=mode;weight[0]=ram;lookups.clear();logs.clear();mgr,depot=manager();name=m.cstring('EnvironmentMap.tga')
   try:resource=m.cdecl(0x5a6e90,[name])
   except Exception:
    print('FAIL',mode,ram,hex(m.u.reg_read(UC_X86_REG_EIP)),lookups,flush=True);raise
   assert m.r(resource+8)==0x200040 and m.r(resource+0x14)==1 and m.r(resource+0x28)==1
   assert m.r(m.r(resource+0x24))==depot and m.r(mgr+0x68)==1 and m.r(mgr+0x78)==0
   cache=mgr+0x9c;classes=[]
   for i in range(m.r(cache+0x24)):
    node=m.r(m.r(cache+0x20)+i*4)
    while node:classes.append(m.r(node+4));node=m.r(node+8)
   assert len(classes)==1,classes;c=classes[0]
   assert logs==[1 if mode else 10]and m.r(c+0x14)==(10 if ram is None else ram)and m.r(c+0x24)==10
   rows.append(dict(warning_raw=mode,ram_current=ram,diagnostic_levels=logs.copy(),lookups=lookups.copy(),flags8=m.r(resource+8),references14=m.r(resource+0x14),requests4=m.r(resource+4),locations=m.r(resource+0x28),work_is_selected=True,active=m.r(mgr+0x68),inactive=m.r(mgr+0x78),physical_source=False,class_name=m.wstr(m.r(c)),ram_weight=m.r(c+0x14),vram_weight=m.r(c+0x24),ram_total=m.r(cache+0x38),vram_total=m.r(cache+0x44),task_refs=m.r(m.r(resource+0x20)+4),source3c=m.r(resource+0x3c),child40=m.r(resource+0x40),width54=m.r(resource+0x54),height58=m.r(resource+0x58)))
 return rows,pins,seams

def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append(dict(precision=pc,native=a));pins.update(pa);seams|=sa|sb
 doc=dict(schema=1,scope=__doc__,rows=rows,executed={hex(a):b for a,b in sorted(pins.items())},intercepted=[hex(a)for a in sorted(seams)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 raw=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-default-texture.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=sum(len(r['native'])for r in rows),executions=sum(len(r['native'])for r in rows)*2,executed=len(pins),intercepted=len(seams),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
