"""Original/v14 actual enum constructors498420/498500/4985E0/4986C0/
4987B0/4989B0; selected464BD0 source-pair intervals including real42BF10,
42B8E0 empty lists/context allocation;4BE420 linkedDefinition five-list
loop (42B8B0 is a named callback transport for exact per-list failures).
 Actor4624B0 constructor runs with49C760 Object-base transport explicit.
 Source intervals start with explicit enclosing Actor registers/stack, not
whole ActorDataLoadData. Positive FX loaders are required cutoffs. No
AbilityDefinition non-FX load, shared registration or global phase claim.
"""
from pathlib import Path
import sys,json,hashlib,gc
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
class Required(Exception):pass

def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set()
 def capture(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,capture);m.thiscall(0x4013e0,0xb43b40,[])
 #The actual Actor constructor writes these fields. Its Object49C760
 #base is a named completed-base transport, never counted as executed.
 name=m.alloc(4);m.w(name,m.cstring('actor'));category=m.alloc(32);actor=m.alloc(0x700);m.u.mem_write(actor,b'\xa5'*0x700)
 m.stubs[0x49c760]=lambda mm,u:mm.ret(u.reg_read(UC_X86_REG_ECX),pop=8)
 m.thiscall(0x4624b0,actor,[name,category]);del m.stubs[0x49c760]
 offsets=[0x194,0x198,0x19c,0x1a0,0x1f0,0x220,0x24c,0x250,0x254,0x258,0x294,0x298,0x29c,0x2a0,0x2a4,0x534,0x598,0x59c,0x5a0,0x5a4]
 constructor={hex(o):m.r(actor+o)for o in offsets}
 names={}
 for key,ctor,owner,count in [('mover',0x498420,0xb3aee8,2),('body',0x498500,0xb3af0c,2),('root',0x4985e0,0xb3af30,2),('animation',0x4986c0,0xb3af54,3),('ability',0x4987b0,0xb3af78,5),('portal',0x4989b0,0xb3afc0,2)]:
  m.thiscall(ctor,owner,[1]);table=m.r(owner+4)
  names[key]=[m.wstr(m.r(m.r(table+4*i)+4))for i in range(count)]
 requests=[]
 for delta in [0,1,0x80000000,0xffffffff]:
  for mode in [0,1,0xffffffff]:
   for stop in [-1,0,2,4]:
    data=m.alloc(0x80);definition=m.alloc(0xa0);m.w(data,definition);calls=[]
    def callback(mm,u):
     s=u.reg_read(UC_X86_REG_ESP);index=(u.reg_read(UC_X86_REG_ECX)-definition-0x70)//8
     calls.append(dict(list=index,delta=mm.r(s+4),mode=mm.r(s+8)))
     if index==stop:raise Required('FXlist')
     mm.ret(pop=8)
    m.stubs[0x4288b0]=callback;status='ok'
    try:m.thiscall(0x4be420,data,[delta,mode])
    except Required:status='required:FXlist'
    requests.append(dict(input=dict(delta=delta,mode=mode,stop=stop),calls=calls,status=status))
 del m.stubs[0x4288b0]
 #Actual context allocation/setup is executed. Only the enclosing owner's
 #already initialized fields/registers and typed source getter are inputs.
 pairs=[];source=m.alloc(4)
 for group,start,end,base,source_at in [('root',0x464f66,0x465007,0x194,0xb0),('mover',0x4660cf,0x466148,0x24c,0x18),('body',0x4664b4,0x466551,0x294,None),('portal',0x468369,0x4683b5,0x598,None),('ability',0x4bf804,0x4bf8bf,0x70,None)]:
  for stop in [None,*names[group]]:
   actor=m.alloc(0x700);trace=[];status='ok';contexts=[]
   def count(mm,u):
    s=u.reg_read(UC_X86_REG_ESP);name=mm.raw_wstr(mm.r(s+4));trace.append('children:'+name)
    mm.ret(1 if name==stop else 0,pop=4)
   def iterator(mm,u):mm.ret(source)
   def positive(mm,u):trace.append('positiveFX');raise Required('positiveFX')
   def synth_observer(u,at,size,_):
    if at==0x42bf10:
     s=u.reg_read(UC_X86_REG_ESP);contexts.append(m.r(s+12))
    if at==0x42b8e0:
     s=u.reg_read(UC_X86_REG_ESP);trace.append(dict(synthesize=[m.r(s+4)-actor,m.r(s+8)-actor],mode=m.r(s+12)))
   m.stubs[0x5b4220]=count;m.stubs[0x42be10]=iterator;m.stubs[0x42b990]=positive
   def interval_end(mm,u):
    #The selected root/Mover end boundary precedes the caller's cdecl
    #42B8E0 addesp12. Transport that cleanup only; no body is replaced.
    if group in ['root','mover']:u.reg_write(UC_X86_REG_ESP,u.reg_read(UC_X86_REG_ESP)+12)
    mm.ret()
   m.stubs[end]=interval_end
   hook=m.u.hook_add(UC_HOOK_CODE,synth_observer)
   #thiscall sets a real return sentinel. The interval uses its enclosing
   #stackframe, whose typed owner/source/register inputs are explicit.
   setup=start
   def interval(mm,u):
    s=u.reg_read(UC_X86_REG_ESP);u.reg_write(UC_X86_REG_EBP,source if group=='ability'else actor);u.reg_write(UC_X86_REG_EBX,source)
    if group=='ability':u.reg_write(UC_X86_REG_ESI,actor)
    if source_at is not None:mm.w(s+source_at,source)
    del mm.stubs[setup];u.reg_write(UC_X86_REG_EIP,setup)
   m.stubs[setup]=interval
   try:m.thiscall(start,actor,[])
   except Required:status='required:positiveFX'
   m.u.hook_del(hook);m.stubs.pop(start,None);del m.stubs[end]
   pairs.append(dict(group=group,stop=stop,status=status,trace=trace,same_context=len(set(contexts))<=1,heads=[m.r(actor+base),m.r(actor+base+8)]))
 health=[]
 for trigger in [0,1,2,3,4,0x7fffffff,0x80000000,0xffffffff]:
  actor=m.alloc(12);m.w(actor+8,m.cstring('actor'));fx=m.alloc(0x40);m.w(fx+0x30,trigger);link=m.alloc(8);m.w(link,fx);m.w(link+4,0)
  def health_start(mm,u):
   u.reg_write(UC_X86_REG_ESI,link);u.reg_write(UC_X86_REG_EBP,actor);del mm.stubs[0x466528];u.reg_write(UC_X86_REG_EIP,0x466528)
  m.stubs[0x466528]=health_start;m.stubs[0x466551]=lambda mm,u:mm.ret()
  def diagnostic(mm,u):raise Required('negativeHealth')
  m.stubs[0x410c40]=diagnostic;status='ok'
  try:m.thiscall(0x466528,actor,[])
  except Required:status='required:negativeHealth'
  m.stubs.pop(0x466528,None);del m.stubs[0x466551]
  health.append(dict(trigger=trigger,status=status))
 inline=[]
 for present in [False,True]:
  scope=m.alloc(4);child=m.alloc(4);reads=[];status=[]
  def node_getter(mm,u):
   s=u.reg_read(UC_X86_REG_ESP);reads.append(dict(name=mm.raw_wstr(mm.r(s+4)),optional=mm.r(s+8)))
   mm.ret(child if present else 0,pop=8)
  def branch_start(mm,u):u.reg_write(UC_X86_REG_EBP,scope);del mm.stubs[0x4bfdd7];u.reg_write(UC_X86_REG_EIP,0x4bfdd7)
  m.stubs[0x5b4410]=node_getter;m.stubs[0x4bfdd7]=branch_start
  def inline_end(mm,u):status.append('inline');mm.ret()
  def shared_end(mm,u):status.append('shared');mm.ret()
  m.stubs[0x4bfdef]=inline_end;m.stubs[0x4bff4e]=shared_end
  m.thiscall(0x4bfdd7,scope,[]);del m.stubs[0x4bfdef];del m.stubs[0x4bff4e];m.stubs.pop(0x4bfdd7,None)
  inline.append(dict(child_present=present,reads=reads,branch=status))
 return dict(constructor=constructor,names=names,requests=requests,pairs=pairs,health=health,inline_selection=inline),pins,seams
def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append(dict(precision=pc,output=a));pins.update(pa);seams|=sa|sb
 for a,b in pins.items():assert bytes.fromhex(b)==h.P.range(a,len(bytes.fromhex(b)))==h.V.range(a,len(bytes.fromhex(b)))
 doc=dict(scope=__doc__,images={'original':hashlib.sha256(h.P.path.read_bytes()).hexdigest(),'v14':hashlib.sha256(h.V.path.read_bytes()).hexdigest()},rows=rows,executed_pins={hex(a):b for a,b in sorted(pins.items())},intercepted_not_executed=[hex(a)for a in sorted(seams)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-actor-request.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n',encoding='utf-8',newline='\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),request_cases=sum(len(r['output']['requests'])for r in rows),pair_cases=sum(len(r['output']['pairs'])for r in rows),executed_pins=len(pins))))
if __name__=='__main__':main()
