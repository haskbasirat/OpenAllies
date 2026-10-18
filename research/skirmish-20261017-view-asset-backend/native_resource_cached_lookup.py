"""Real5A77E0/5A8520 cached-name selection, real5AD430/5AEA90 cached
reference increments and actual Texture/Model class getters. The dictionary
leaf5AC160 is explicit cache transport. First uncached5AEA90(load1) and native
wrong-class diagnostic are REQUIRED stopped boundaries, never fake creations.
73A560 and current40F7B0 suffix are explicit laboratory inputs, not GPU claims.
"""
from pathlib import Path
import sys,json,hashlib,gc
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
CASES=[
 ('texture','Art/test.tga','winter','original',False),('texture','Art/test.tga','winter','candidate',False),('texture','Art/test.tga','winter','wrong_candidate',False),('texture','test.tga','','original',False),('texture','missing.tga','winter','none',False),
 ('model','Art/test.nif','winter','original',False),('model','Art/test.nif','winter','candidate',False),('model','Art/test.nif','winter','candidate_variant',True),('model','Art/test.nif','winter','candidate',True),('model','Art/test.nif','winter','original_variant',True),('model','Art/test.nif','winter','original',True),('model','test.nif','','original',False),('model','test.nif','','original_variant',True)]
def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};intercepted=set();entries={};queries=[];policy=[];current=[False];boundary=[None]
 def inc(m,u):
  p=m.r(u.reg_read(UC_X86_REG_ESP)+4);value=(m.r(p)+1)&0xffffffff;m.w(p,value);m.ret(value,pop=4)
 m.stubs[0x301f750]=inc;m.w(0x9f01e8,0x301f750)
 def cached(m,u):
  name=m.wstr(m.r(m.r(u.reg_read(UC_X86_REG_ESP)+4)));key=name.replace('\\','/').lower();queries.append(name)
  m.ret(entries.get(key,(0,0))[0],pop=4)
 m.stubs[0x5ac160]=cached
 def stop(m,u,label):boundary[0]=label;u.emu_stop()
 def uncached(m,u):
  if m.r(u.reg_read(UC_X86_REG_ESP)+8)&255:stop(m,u,'5AEA90 actual source creation')
 m.stubs[0x5aea90]=uncached
 m.stubs[0x4252a0]=lambda m,u:stop(m,u,'4252A0 wrong class')
 m.stubs[0x425210]=lambda m,u:stop(m,u,'425210 missing')
 def hardware(m,u):policy.append(current[0]);m.ret(int(current[0]))
 m.stubs[0x73a560]=hardware
 def suffix(m,u):
  esp=u.reg_read(UC_X86_REG_ESP);out=m.r(esp+4);assert m.r(esp+8)==50;m.w(out,m.cstring('-h'));m.ret(out,pop=8)
 m.stubs[0x40f7b0]=suffix
 def executed(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if boundary[0]is not None:intercepted.add(at)
   elif u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:intercepted.add(at)
 m.u.hook_add(UC_HOOK_CODE,executed);m.thiscall(0x4013e0,0xb43b40,[])
 manager=m.alloc(0x100);m.w(0xb36540,manager)
 def box(text):p=m.alloc(4);m.w(p,m.cstring(text));return p
 def make(name,kind):
  resource=m.alloc(0x100);m.thiscall(0x5a60e0 if kind=='texture' else 0x5a7db0,resource,[box(name)]);slot=m.alloc(4);m.w(slot,resource);entries[name.lower()]=(slot,resource);return resource
 def invoke(at,args):
  oldfs=m.r(0);m.w(m.stack,m.stop)
  for i,v in enumerate(args):m.w(m.stack+4+i*4,v)
  m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_ECX,0);m.u.emu_start(at,m.stop,count=5_000_000);m.check_null();out=m.u.reg_read(UC_X86_REG_EAX);m.w(0,oldfs)
  assert boundary[0]is not None or m.u.reg_read(UC_X86_REG_EIP)==m.stop
  return out
 rows=[]
 for kind,name,folder,setup,hw in CASES:
  for extra in [0,1,0xffffffff]:
   entries.clear();queries.clear();policy.clear();boundary[0]=None;current[0]=hw;m.w(manager+0x80,m.cstring(folder))
   if setup!='none':
    if setup in('candidate','wrong_candidate','candidate_variant'):key='Art/winter/test.'+('tga'if kind=='texture'else'nif')
    else:key=name
    if setup.endswith('variant'):key=key.rsplit('.',1)[0]+'-h.'+key.rsplit('.',1)[1]
    make(key,'model'if setup=='wrong_candidate'else kind)
   out=invoke(0x5a77e0 if kind=='texture'else 0x5a8520,[box(name),extra]);selected=next((k for k,(_,p)in entries.items()if p==out),None)if boundary[0]is None else None
   rows.append(dict(kind=kind,name=name,subfolder=folder,setup=setup,hardware=hw,extra=extra,selected=selected,boundary=boundary[0],queries=queries.copy(),policy_calls=policy.copy(),references={k:m.r(p+0x14)for k,(_,p)in entries.items()}))
   boundary[0]=None
 return rows,pins,intercepted

def main():
 rows=[];pins={};intercepted=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append(dict(precision=pc,native=a));pins.update(pa);intercepted|=sa|sb
 doc=dict(schema=1,scope=__doc__,rows=rows,executed={hex(a):b for a,b in sorted(pins.items())},intercepted=[hex(a)for a in sorted(intercepted)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 p=HERE/'native-resource-cached-lookup.json';raw=(json.dumps(doc,indent=1)+'\n').encode()
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=sum(len(r['native'])for r in rows),machines=6,executions=sum(len(r['native'])for r in rows)*2,executed=len(pins),intercepted=len(intercepted),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()

