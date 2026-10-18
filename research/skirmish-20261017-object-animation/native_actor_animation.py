"""Actual463B50->463440->4632D0->461F70 Actor Animation source prefix,
45F3E0 set constructor,499FF0 type registry and45EA60 request child order.
Node getters, iterator and Sequence factory are named typed transports.
Real497020 prefix/alias lookup, format405D60 and CString/vector bodies run.
Fixed empty FX lists run real42BF10/428A30; unavailable positive FX nodes
and native diagnostics are required cutoffs. This does not certify the
whole ActorData loader, all source phases or decoded KF/controller state.
"""
from pathlib import Path
import sys,json,hashlib,gc,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_FPCW
class Required(Exception):pass

def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set()
 def capture(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,capture);m.thiscall(0x4013e0,0xb43b40,[])
 m.thiscall(0x499ff0,0xb3ae88,[])
 names=[m.wstr(m.r(m.r(0xb3ae88)+8*i))for i in range(7)]
 aliases=[dict(name=m.wstr(m.r(m.r(0xb3ae94)+8*i)),kind=m.r(m.r(0xb3ae94)+8*i+4))for i in range(m.r(0xb3ae98))]
 print('registry',names,flush=True)if'--debug'in sys.argv else None
 #Exact literal call names from461F70's three registrations, with the
 #same layout read by actual code; registry construction remains separate.
 registry=m.alloc(12)
 for i,name in enumerate(['WhenStart','WhenDo','WhenEnd']):
  entry=m.alloc(8);m.w(entry+4,m.cstring(name));m.w(registry+4*i,entry)
 m.w(0xb3af58,registry)
 inputs={};trace=[];node=m.alloc(4);resource=m.alloc(0x40)
 def text(mm,p):return mm.raw_wstr(p)
 def typed_string(mm,u):
  key=text(mm,mm.r(u.reg_read(UC_X86_REG_ESP)+4));key=key.lower();trace.append('read:'+key)
  if inputs.get('stop')=='read:'+key:raise Required('read:'+key)
  slot=mm.alloc(4);mm.w(slot,mm.cstring(inputs[key]));mm.ret(slot,pop=4)
 m.stubs[0x5b54d0]=typed_string
 def fixed(mm,u):
  key=text(mm,mm.r(u.reg_read(UC_X86_REG_ESP)+4));key=key.lower();trace.append('read:'+key)
  if inputs.get('stop')=='read:'+key:raise Required('read:'+key)
  value=mm.alloc(4);mm.w(value,inputs['times'][0]if key=='time'else inputs[key]);stub=mm.alloc(16);mm.u.mem_write(stub,b'\xd9\x05'+struct.pack('<I',value)+b'\xc2\x04\x00');u.reg_write(UC_X86_REG_EIP,stub)
 m.stubs[0x5b5130]=fixed
 def integer(mm,u):
  key=text(mm,mm.r(u.reg_read(UC_X86_REG_ESP)+4));key=key.lower();trace.append('read:'+key)
  if inputs.get('stop')=='read:'+key:raise Required('read:'+key)
  mm.ret(inputs[key],pop=4)
 m.stubs[0x5b4ed0]=integer;m.stubs[0x5b5380]=integer
 def count(mm,u):
  key=text(mm,mm.r(u.reg_read(UC_X86_REG_ESP)+4));trace.append('children:'+key)
  if inputs.get('stop')==key:raise Required(key)
  mm.ret(1 if key=='Animation'else len(inputs.get('times',[]))if key=='WhenDoAtTime'else 0,pop=4)
 m.stubs[0x5b4220]=count
 m.stubs[0x42be10]=lambda mm,u:mm.ret(node)
 def relative(mm,u):
  arg=mm.r(u.reg_read(UC_X86_REG_ESP)+4);value=mm.wstr(mm.r(arg));value=value.lstrip('/')if value.startswith('/')else inputs['folder']+'/'+value
  trace.append('relative:'+value);mm.w(arg,mm.cstring(value));mm.ret(arg,pop=4)
 m.stubs[0x4763c0]=relative
 def sequence(mm,u):
  value=mm.wstr(mm.r(mm.r(u.reg_read(UC_X86_REG_ESP)+4)));trace.append('sequence:'+value)
  if inputs.get('stop')=='sequence':raise Required('sequence')
  mm.ret(resource if inputs['resource']else 0)
 m.stubs[0x5a80d0]=sequence
 def diagnostic(mm,u):trace.append('diagnostic');raise Required('diagnostic')
 m.stubs[0x410c40]=diagnostic
 def timed_fx(mm,u):trace.append('timedFX');raise Required('timedFX')
 m.stubs[0x42b990]=timed_fx
 def cases():
  base=dict(ids='idle0',type='idle',kf='%s%s.kf',prefix='unit_',folder='Art/Units',length=0x3f800000,key_ratio=0x3f000000,walk_distance=0x3f800000,animation_blend=0,sister_anim_type='',weight=0x3f800000,damage_threshold=0,damage_filter=0xffffffff,resource=True,stop=None)
  for name in names:yield dict(base,type=name,ids=name+'0')
  for kind in ['IDLE_extra','move_fast','die_water']:yield dict(base,type=kind,ids=kind)
  for alias in aliases:yield dict(base,type=alias['name'].upper(),ids=alias['name'])
  yield dict(base,sister_anim_type='MOVE_other',animation_blend=1)
  yield dict(base,resource=False)
  for key,value in [('damage_threshold',0x3f800000),('damage_filter',7),('key_ratio',0xbf800000),('key_ratio',0x40000000)]:yield dict(base,**{key:value})
  for stop in ['sequence','read:type','read:length','read:key_ratio','read:walk_distance','read:animation_blend','read:sister_anim_type','read:weight','read:damage_threshold','read:damage_filter','WhenStart','WhenDo','WhenEnd','WhenDoAtTime']:yield dict(base,stop=stop)
  for time in [0,0x80000000,0x3f800000,0xbf800000]:yield dict(base,times=[time])
  for inherit,ids in [(True,'idle0'),(True,'IDLE0'),(False,'idle0'),(True,'new')]:yield dict(base,seed=dict(base),inherit=inherit,ids=ids)
 loads=[]
 for inputs in cases():
  trace=[];actor=m.alloc(0x700);m.w(actor+0xb0,m.cstring(inputs['prefix']));m.w(actor+8,m.cstring('actor'))
  owner=m.alloc(0x6c);m.u.mem_write(owner,b'\xa5'*0x6c);m.thiscall(0x45f3e0,owner,[])
  constructor=dict(count=m.r(owner+4),capacity=m.r(owner+8),typed_counts=[m.r(owner+0x10+12*i)for i in range(7)],weights=[m.r(m.r(owner+0x60)+i*4)for i in range(7)])
  name=m.alloc(4);m.w(name,m.cstring('Animation'));status='ok'
  if 'seed'in inputs:
   target_input=inputs;inputs=inputs['seed'];m.thiscall(0x463b50,actor,[node,name,0,owner,0]);inputs=target_input;trace=[]
  try:m.thiscall(0x463b50,actor,[node,name,0,owner,int(inputs.get('inherit',False))])
  except Required as e:status='required:'+str(e)
  count_=m.r(owner+4);a=m.r(m.r(owner)+4*(count_-1))if count_ else None
  fields=None if a is None else {hex(o):m.r(a+o)for o in [4,8,0x10,0x14,0x1c,0x20,0x24,0x28,0x2c,0x30]}
  if a:
   timed=[];cue=m.r(a+0x50)
   while cue:timed.append(dict(time=m.r(cue),fx_present=m.r(cue+4)!=0));cue=m.r(cue+12)
   fields.update(ids=m.wstr(m.r(a+12)),blend=bytes(m.u.mem_read(a+24,1))[0],same_resource=m.r(a+0x34)==resource,resource_present=m.r(a+0x34)!=0,fx_counts=[m.r(a+i+4)for i in [0x38,0x40,0x48]],timed_present=m.r(a+0x50)!=0,timed=timed)
  loads.append(dict(input=inputs,status=status,trace=trace,constructor=constructor,count=count_,fields=fields,typed_counts=[m.r(owner+0x10+12*i)for i in range(7)],weights=[m.r(m.r(owner+0x60)+i*4)for i in range(7)]))
 #45EA60 resource priority is literal ZERO, unlike Object49B7E0.
 requests=[]
 for present in [False,True]:
  for delta in [0,1,0xffffffff]:
   for timed_count in [0,2]:
    a=m.alloc(0x58);m.w(a+0x34,resource if present else 0);calls=[];vt=m.alloc(16);m.w(resource,vt);lists={a+0x38:0,a+0x40:1,a+0x48:2};head=0
    for i in reversed(range(timed_count)):
     cue=m.alloc(16);m.w(cue+12,head);head=cue;lists[cue+4]=3+i
    m.w(a+0x50,head)
    def request(mm,u):s=u.reg_read(UC_X86_REG_ESP);calls.append(dict(kind='sequence',delta=mm.r(s+4),priority=mm.r(s+8)));mm.ret(pop=8)
    def fx_observer(u,at,size,_):
     if at==0x4288b0:
      s=u.reg_read(UC_X86_REG_ESP);calls.append(dict(kind='fx',list=lists[u.reg_read(UC_X86_REG_ECX)],delta=m.r(s+4),priority=m.r(s+8)))
    hook=m.u.hook_add(UC_HOOK_CODE,fx_observer);m.w(vt+12,m.stub(request));m.thiscall(0x45ea60,a,[delta]);m.u.hook_del(hook);requests.append(dict(present=present,delta=delta,timed_count=timed_count,calls=calls))
 finalizers=[]
 for kinds in [[],[0],[0,1],[0,1,3],[0,1,3,2],[0,1,3,2,2],[0,1,3,2,2,2]]:
  for threshold in [0,0x3f800000,0xbf800000]:
   owner=m.alloc(0x6c);m.thiscall(0x45f3e0,owner,[]);pointers=[];trace=[]
   for i,kind in enumerate(kinds):
    ids=m.cstring('animation'+str(i));a=m.thiscall(0x463440,owner,[kind,ids,0])['eax'];pointers.append(a)
    m.w(a+0x2c,[0x3f800000,0x40400000,0x3f000000][i%3]);m.w(a+0x24,threshold if kind==3 else 0)
   actor=m.alloc(12);m.w(actor+8,m.cstring('actor'));status='ok'
   try:m.thiscall(0x461190,actor,[owner])
   except Required:status='required:diagnostic'
   finalizers.append(dict(kinds=kinds,walks=[m.r(a+0x2c)for a in pointers],death_threshold=threshold,status=status,move_thresholds=[m.r(a+0x30)for a in pointers]))
 return dict(names=names,aliases=aliases,loads=loads,requests=requests,finalizers=finalizers),pins,seams

def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append(dict(precision=pc,output=a));pins.update(pa);seams|=sa|sb
 for a,b in pins.items():assert bytes.fromhex(b)==h.P.range(a,len(bytes.fromhex(b)))==h.V.range(a,len(bytes.fromhex(b)))
 doc=dict(images={'original':hashlib.sha256((ROOT.parent/'AA.exe').read_bytes()).hexdigest(),'v14':hashlib.sha256((ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe').read_bytes()).hexdigest()},scope=__doc__,rows=rows,executed_pins={hex(a):b for a,b in sorted(pins.items())},intercepted_not_executed=[hex(a)for a in sorted(seams)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-actor-animation.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n',encoding='utf-8',newline='\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),loads=2*sum(len(r['output']['loads'])for r in rows),requests=2*sum(len(r['output']['requests'])for r in rows),pins=len(pins))))
if __name__=='__main__':main()

