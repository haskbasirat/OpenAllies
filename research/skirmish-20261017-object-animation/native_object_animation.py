"""Actual ObjectAnimationSet49BF20,49DA40->49C020->49D900 and49B7E0.
The typed node getters and Sequence factory are explicit source inputs;
native enum497F10 registration is captured, then actual497220 traverses a
one-bucket map containing those exact registrations. Actual source format
405D60/CString/vector bodies execute.4763C0 source-folder transport delegates
the separately proven relative-path algorithm. Positive CreateVFX/PlaySFX
source-count reads and debug assertion are required cutoffs, never completed loads.
Request virtual+C is a recorded child service, with optional required stops
and current-priority mutation. No Sequence decode/resource readiness claim.
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
 enum=[]
 def registration(mm,u):
  s=u.reg_read(UC_X86_REG_ESP);enum.append({'name':mm.raw_wstr(mm.r(s+8)),'kind':mm.r(s+4)});mm.ret(pop=12)
 m.stubs[0x4976b0]=registration;m.thiscall(0x497f10,m.alloc(0x40),[])
 buckets=m.alloc(4);m.w(0xb3b138,buckets);m.w(0xb3b13c,1);head=0
 for e in reversed(enum):
  record=m.alloc(12);m.w(record,e['kind']);m.w(record+4,m.cstring(e['name']))
  node=m.alloc(8);m.w(node,record);m.w(node+4,head);head=node
 m.w(buckets,head)
 owner=m.alloc(0x6c);m.u.mem_write(owner,b'\xa5'*0x6c);m.thiscall(0x49bf20,owner,[])
 constructor={'overall_count':m.r(owner+4),'overall_capacity':m.r(owner+8),
  'typed_count':[m.r(owner+0x10+12*i)for i in range(8)],'typed_capacity':[m.r(owner+0x14+12*i)for i in range(8)]}
 inputs={};trace=[];resource=m.alloc(0x40);m.w(resource+0x1c,0)
 def string_get(mm,u):
  key=mm.raw_wstr(mm.r(u.reg_read(UC_X86_REG_ESP)+4));trace.append('read:'+key)
  if inputs.get('stop')=='read:'+key:raise Required('read:'+key)
  slot=mm.alloc(4);mm.w(slot,mm.cstring(inputs[key]));mm.ret(slot,pop=4)
 m.stubs[0x5b54d0]=string_get
 def number_get(mm,u):
  key=mm.raw_wstr(mm.r(u.reg_read(UC_X86_REG_ESP)+4));trace.append('read:'+key)
  if inputs.get('stop')=='read:'+key:raise Required('read:'+key)
  value=mm.alloc(4);mm.w(value,inputs[key]);stub=mm.alloc(16)
  mm.u.mem_write(stub,b'\xd9\x05'+struct.pack('<I',value)+b'\xc2\x04\x00');u.reg_write(UC_X86_REG_EIP,stub)
 m.stubs[0x5b4ae0]=number_get
 def boolean_get(mm,u):
  key=mm.raw_wstr(mm.r(u.reg_read(UC_X86_REG_ESP)+4));trace.append('read:'+key)
  if inputs.get('stop')=='read:'+key:raise Required('read:'+key)
  mm.ret(inputs[key],pop=4)
 m.stubs[0x5b5380]=boolean_get
 def relative(mm,u):
  arg=mm.r(u.reg_read(UC_X86_REG_ESP)+4);name=mm.wstr(mm.r(arg));folder=inputs['folder']
  out=name.lstrip('/')if name.startswith('/')else folder+'/'+name
  trace.append('relative:'+out);mm.w(arg,mm.cstring(out));mm.ret(arg,pop=4)
 m.stubs[0x4763c0]=relative
 def factory(mm,u):
  name=mm.wstr(mm.r(mm.r(u.reg_read(UC_X86_REG_ESP)+4)));trace.append('sequence:'+name)
  if inputs.get('stop')=='sequence':raise Required('sequence')
  mm.ret(resource if inputs['resource']else 0)
 m.stubs[0x5a80d0]=factory
 def child(label):
  def callback(mm,u):trace.append(label);raise Required(label)
  return callback
 m.stubs[0x855d30]=child('assert');m.stubs[0x410c40]=child('duplicate')
 #Execute the actual two loaders for empty children. Positive children are
 #required at entry before any invented pair append/Find or Audio lookup.
 def count(mm,u):
  key=mm.raw_wstr(mm.r(u.reg_read(UC_X86_REG_ESP)+4));trace.append('children:'+key)
  if inputs.get('stop')==key:raise Required(key)
  mm.ret(0,pop=4)
 m.stubs[0x5b4220]=count
 def rows():
  base=dict(kf='%s%s.kf',prefix='unit_',folder='Art/Units',length=0x3fc00000,exclusive=0,start_min=0x80000000,start_max=0x3f800000,resource=True,stop=None)
  for i,e in enumerate(enum):yield dict(base,ids='anim'+str(i),type=e['name'].lower(),exclusive=(i%2))
  yield dict(base,ids='ANIM0',type='CONST')
  for i,stop in enumerate(['sequence','read:type','read:length','read:exclusive','read:start_min','read:start_max','CreateVFX','PlaySFX']):yield dict(base,ids='stop'+str(i),type='IDLE',stop=stop)
  yield dict(base,ids='unknown',type='unregistered')
  yield dict(base,ids='missing',type='IDLE',resource=False)
  yield dict(base,ids='literal',type='SELECTED',kf='/Shared/%%literal.kf')
 loaded=[]
 for inputs in rows():
  trace=[];before=m.r(owner+4);arg=m.alloc(4);m.w(arg,m.cstring(inputs['prefix']));status='ok';pending=None
  #Observe the real new49C020 allocation, including exact failing field prefix.
  def pending_(u,at,size,_):
   nonlocal pending
   if at==0x49c020:pending=u.reg_read(UC_X86_REG_ECX)
  hook=m.u.hook_add(UC_HOOK_CODE,pending_)
  try:m.thiscall(0x49da40,owner,[m.alloc(4),arg])
  except Required as e:status='required:'+str(e)
  m.u.hook_del(hook)
  def fields(p):
   return None if p is None else dict(ordinal=m.r(p),ids=m.wstr(m.r(p+4)),kind=(m.r(p+8)-(1<<32)if m.r(p+8)&0x80000000 else m.r(p+8)),length=m.r(p+12),exclusive=bytes(m.u.mem_read(p+24,1))[0],start_min=m.r(p+16),start_max=m.r(p+20),same_resource=m.r(p+28)==resource)
  after=m.r(owner+4)
  loaded.append({'input':inputs,'status':status,'trace':trace,'before':before,'count':after,'pending':fields(pending),
   'typed_counts':[m.r(owner+0x10+12*i)for i in range(8)],
   'typed_same':all(m.r(m.r(owner+0xc+12*i)+j*4)in[m.r(m.r(owner)+k*4)for k in range(after)]for i in range(8)for j in range(m.r(owner+0x10+12*i)))})
 #Actual49B7E0 loop with nullable references and reread current priority;
 #every virtual call is a named child, never decoded/ready success.
 requests=[]
 for delta in (0,1,0xffffffff):
  for fail in (None,0,1):
   target=m.alloc(12);vector=m.alloc(16);m.w(target,vector);m.w(target+4,4);calls=[]
   for i in range(4):
    a=m.alloc(0x20);m.w(vector+4*i,a)
    if i==1:continue
    r=m.alloc(16);vt=m.alloc(16);m.w(r,vt);m.w(a+28,r)
    def request(mm,u,index=i):
     s=u.reg_read(UC_X86_REG_ESP);calls.append({'index':index,'delta':mm.r(s+4),'priority':mm.r(s+8)})
     if fail==len(calls)-1:raise Required('resource')
     mm.w(0xb282f0,(mm.r(0xb282f0)+3)&0xffffffff);mm.ret(pop=8)
    m.w(vt+12,m.stub(request))
   m.w(0xb282f0,1);status='ok'
   try:m.thiscall(0x49b7e0,target,[delta])
   except Required:status='required:resource'
   requests.append(dict(delta=delta,fail=fail,status=status,calls=calls,priority=m.r(0xb282f0)))
 return {'enum':enum,'constructor':constructor,'loads':loaded,'requests':requests},pins,seams

def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append({'precision':pc,'output':a});pins.update(pa);seams|=sa|sb
 for a,b in pins.items():assert bytes.fromhex(b)==h.P.range(a,len(bytes.fromhex(b)))==h.V.range(a,len(bytes.fromhex(b)))
 doc={'images':{'original':hashlib.sha256((ROOT.parent/'AA.exe').read_bytes()).hexdigest(),'v14':hashlib.sha256((ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe').read_bytes()).hexdigest()},'scope':__doc__,'rows':rows,'executed_pins':{hex(a):b for a,b in sorted(pins.items())},'intercepted_not_executed':[hex(a)for a in sorted(seams)],'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
 path=HERE/'native-object-animation.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n',encoding='utf-8',newline='\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':2*len(rows),'loads':2*sum(len(r['output']['loads'])for r in rows),'requests':2*sum(len(r['output']['requests'])for r in rows),'genuine_pins':len(pins)}))
if __name__=='__main__':main()

