"""Full5A3250 caller with explicit manager lock/find/remove,5A31D0 constructor,
depot query/add, diagnostics and publication services. Actual native branches,
current depot-tail read, final405B80 name copy and unlock call execute. The
fixture supplies child-produced list contents; it does not certify their
producer or replace the actual backend location/registry proof.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
#Install only one hook at each logger address. Loader's default logger seam
#would otherwise execute a second ret before this fixture's checked callback.
L.LOGGERS={a:n for a,n in L.LOGGERS.items()if a not in (0x404ea0,0x404ed0)}
class Stopped(Exception):pass
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc]);events=[]
 manager=m.alloc(0x98);lockvt=m.alloc(12);m.w(manager+0x20,lockvt);m.w(0xb36540,manager)
 work=m.alloc(4);foreign=m.alloc(4);m.w(manager+0x90,work)
 old=m.alloc(0xb4)if case['existing']else 0
 ids=m.alloc(4);m.w(ids,m.cstring('Films/LastFilm_0.RFM'))
 creator=m.alloc(4);name=m.alloc(4);m.w(name,m.cstring('current-map-name'))
 resource=None;header=None
 def event(s):
  if case.get('stop')==s:raise Stopped(s)
  events.append(s)
 def locked():event('lock');assert m.u.reg_read(UC_X86_REG_ECX)==manager+0x20;m.ret(0)
 def unlocked():event('unlock');assert m.u.reg_read(UC_X86_REG_ECX)==manager+0x20;m.ret(0)
 for off,fn,stub in [(4,locked,L.STUBS+0xc00),(8,unlocked,L.STUBS+0xc20)]:m.seam(stub,fn);m.w(lockvt+off,stub)
 def found():event('find');assert m.arg(0)==ids;m.ret(0,old)
 def only_work():event('only_work');assert m.u.reg_read(UC_X86_REG_ECX)==old;m.ret(0,int(case['only_work']))
 def removed():event('remove');assert m.u.reg_read(UC_X86_REG_ECX)==manager and m.arg(0)==old;m.ret(4)
 def warning(which):event(which);m.ret(0)
 def list_fields(items):
  a=m.alloc(max(4,4*len(items)))
  for i,item in enumerate(items):m.w(a+4*i,item)
  m.w(resource+0x24,a);m.w(resource+0x28,len(items))
 def constructed():
  nonlocal resource,header
  event('construct');assert m.arg(0)==ids and m.arg(1)==creator
  resource=m.u.reg_read(UC_X86_REG_ECX);header=m.alloc(0x68)
  m.w(resource+0x90,header);m.w(header+0x38,m.cstring('old-header-name'))
  list_fields(([work]if case['topmost']else[work,foreign])if case['has_work']else[])
  m.ret(8,resource)
 def has_work():event('has_work');assert m.u.reg_read(UC_X86_REG_ECX)==resource and m.arg(0)==work;m.ret(4,int(case['has_work']))
 def add_work():
  event('add_work');assert m.u.reg_read(UC_X86_REG_ECX)==resource and m.arg(0)==work
  list_fields([work]if case['topmost']else[work,foreign]);m.ret(4)
 def published():
  event('publish');assert m.u.reg_read(UC_X86_REG_ECX)==manager and m.arg(0)==resource
  if case.get('mutate_name'):m.w(name,m.cstring('changed-during-publication'))
  m.ret(4)
 m.seam(0x59f720,found);m.seam(0x5aa3c0,only_work);m.seam(0x5ac510,removed)
 m.seam(0x404ea0,lambda:warning('warn_existing'));m.seam(0x404ed0,lambda:warning('warn_topmost'))
 m.seam(0x5a31d0,constructed);m.seam(0x5aa390,has_work);m.seam(0x5aaae0,add_work);m.seam(0x5acec0,published)
 def before(u,at,size,_):
  if at==0x5a335c:event('topmost')
  elif at==0x5a33b3:event('name')
 m.u.hook_add(UC_HOOK_CODE,before)
 stopped=None
 try:
  assert m.call(0x5a3250,0,[ids,creator,name])==resource
  assert all(m.u.reg_read(r)==0x5a5a0000 for r in [UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])
 except Stopped as e:stopped=str(e)
 assert (m.u.reg_read(UC_X86_REG_FPSW)>>11)&7==0
 return dict(events=events,stopped=stopped,constructed=resource is not None,
             recorded_name=m.text(m.r(header+0x38))if header else None),m.windows()
def cases():
 for existing,only in [(False,False),(True,False),(True,True)]:
  for has in [False,True]:
   for top in [False,True]:
    c=dict(existing=existing,only_work=only,has_work=has,topmost=top,mutate_name=not top)
    yield c
    for stop in ['find','only_work','warn_existing','remove','construct','has_work','add_work','topmost','warn_topmost','publish','name']:
     yield dict(c,stop=stop)
def main():
 rows=[];blocks={}
 for c in cases():
  for pc in PC:
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
   rows.append(dict(input=c,pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 raw=(json.dumps(dict(schema=1,scope=__doc__,cases=rows,blocks=[blocks[k]for k in sorted(blocks)]),indent=1)+'\n').encode()
 path=HERE/'native-film-create.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
