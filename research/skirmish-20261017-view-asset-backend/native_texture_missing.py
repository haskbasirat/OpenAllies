"""Real5A77E0 final-lookup missing catch, actual425210/4252A0 throw
construction and catch funclet5A7976. Catalog/cache absence is an explicit
leaf transport, not a real generic depot-load proof. MSVC EH dispatch uses
the inherited disclosed transport (inner frame destructors not executed).
Default Texture5A6E90 entry and uncaught exceptions are required stops.
Current UVar61 and caller words are labelled lab inputs, not source currents.
"""
from pathlib import Path
import sys,json,gc,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE,UC_HOOK_MEM_READ
from unicorn.x86_const import *

def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set();rows=[]
 m.watch_frames(0x5a781b,0xaeecbc)
 current=m.alloc(4);uv=m.alloc(0x400);m.w(uv+0xf4,current);m.w(0xb39940,uv)
 manager=m.alloc(0x100);m.w(0xb36540,manager)
 active=[None];outcome=[None];queries=[];reads=[];wrong=[0]
 def stop(label):outcome[0]=label;m.u.emu_stop()
 def cached(m,u):
  name=m.wstr(m.r(m.r(u.reg_read(UC_X86_REG_ESP)+4)));queries.append(name)
  m.ret(wrong[0] if active[0]=='wrong_candidate' and 'winter' in name else 0,pop=4)
 m.stubs[0x5ac160]=cached
 def catalog(m,u):
  if m.r(u.reg_read(UC_X86_REG_ESP)+8)&255:m.ret(0,pop=8)
 m.stubs[0x5aea90]=catalog
 def throw(m,u):
  esp=u.reg_read(UC_X86_REG_ESP);ti=m.r(esp+8)
  if ti==0:stop('rethrow');return
  if not m._dispatch(u):stop('uncaught_corrupt' if ti==0xad9e00 else 'uncaught_other')
 m.stubs[0x96bef5]=throw
 m.stubs[0x5a6e90]=lambda m,u:stop('default_texture_5a6e90_required')
 def executed(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if outcome[0] is None and u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 def current_read(u,access,address,size,value,_):
  if address<=current<address+size:reads.append(hex(u.reg_read(UC_X86_REG_EIP)))
 m.u.hook_add(UC_HOOK_CODE,executed);m.u.hook_add(UC_HOOK_MEM_READ,current_read,begin=current,end=current+3)
 m.thiscall(0x4013e0,0xb43b40,[])
 def inc(m,u):
  p=m.r(u.reg_read(UC_X86_REG_ESP)+4);v=(m.r(p)+1)&0xffffffff;m.w(p,v);m.ret(v,pop=4)
 m.stubs[0x301f750]=inc;m.w(0x9f01e8,0x301f750)
 name=m.alloc(4);m.w(name,m.cstring('winter/test.tga'));resource=m.alloc(0x100);m.thiscall(0x5a7db0,resource,[name]);slot=m.alloc(4);m.w(slot,resource);wrong[0]=slot
 for kind in ['missing','wrong_candidate']:
  for mode in [0,1,128,255]:
   for soft in [0,1,256,257,0xffffffff]:
    m.eh_frames.clear();m.eh_events.clear();m.messages.clear();m.null_accesses.clear();queries.clear();reads.clear();outcome[0]=None;active[0]=kind
    m.w(current,mode);m.w(manager+0x80,m.cstring('winter'if kind=='wrong_candidate'else''));before=m.r(resource+0x14)
    text=m.alloc(4);m.w(text,m.cstring('test.tga'));oldfs=m.r(0)
    try:
     m.w(m.stack,m.stop);m.w(m.stack+4,text);m.w(m.stack+8,soft);m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_ECX,0);m.u.emu_start(0x5a77e0,m.stop,count=5_000_000);m.check_null()
    except Exception:
     print('FAILED',kind,mode,soft,'pc',hex(m.u.reg_read(UC_X86_REG_EIP)),'eh',m.eh_events,flush=True);raise
    if outcome[0]is None:
     assert m.u.reg_read(UC_X86_REG_EIP)==m.stop and m.u.reg_read(UC_X86_REG_EAX)==0
     outcome[0]='null'
    m.w(0,oldfs)
    expected='uncaught_corrupt'if kind=='wrong_candidate'else'null'if soft&255 else'default_texture_5a6e90_required'if mode else'rethrow'
    assert outcome[0]==expected,(kind,mode,soft,outcome[0],m.eh_events)
    assert reads==([]if kind=='wrong_candidate'else['0x5a7981']),reads
    rows.append(dict(kind=kind,compatibility_raw=mode,caller_word=soft,result=outcome[0],current_reads=reads.copy(),queries=queries.copy(),references_delta=(m.r(resource+0x14)-before)&0xffffffff,catch=[e['handler']for e in m.eh_events]))
 return rows,pins,seams

def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append(dict(precision=pc,native=a));pins.update(pa);seams|=sa|sb
 doc=dict(schema=1,scope=__doc__,rows=rows,executed={hex(a):b for a,b in sorted(pins.items())},intercepted=[hex(a)for a in sorted(seams)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 raw=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-texture-missing.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=sum(len(r['native'])for r in rows),executions=sum(len(r['native'])for r in rows)*2,executed=len(pins),intercepted=len(seams),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
