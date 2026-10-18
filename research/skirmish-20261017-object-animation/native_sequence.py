"""Actual5A5FB0 NiSequence Resource constructor, base5AA890 and commonTask;
actual5A80D0â†’5A7750 typed lookup and classgetter5A5610.9D0710
class-registration linkage is metadata-only (not executed registry setup).
Heap/CString and Win32 atomic transport use the established Resource harness.
Find5AD430/5AEA90 returns SAME actual constructed allocations as named lookup
transport; missing/wrongclass diagnostic entries are explicit required stops.
No KF decode/loadedflag/GPU/requestsuccess is manufactured by this proof.
"""
from pathlib import Path
import sys,json,hashlib,itertools,gc
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_FPCW
class Required(Exception):pass
def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set();allocations=[]
 def inc(m,u):
  p=m.r(u.reg_read(UC_X86_REG_ESP)+4);v=(m.r(p)+1)&0xffffffff;m.w(p,v);m.ret(v,pop=4)
 m.stubs[0x301f750]=inc;m.w(0x9f01e8,0x301f750)
 def capture(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,capture)
 m.thiscall(0x4013e0,0xb43b40,[])
 rows=[]
 for name in ('Art/idle.kf','Art\\walk.KF','Shared/Animations/attack.kf'):
  slot=m.alloc(4);m.w(slot,m.cstring(name));resource=m.cdecl(0x5a5fb0,[slot]);allocations.append(resource)
  task=m.r(resource+0x20);assert m.r(task+0x1c)==resource
  rows.append({'name':name,'base':{hex(i):m.r(resource+i)for i in (4,8,12,16,20,24,28,40,44,48,52)},
   'derived':{hex(i):m.r(resource+i)for i in (60,64,68,72,76)},'vtable':hex(m.r(resource)),
   'task':{hex(i):m.r(task+i)for i in (4,8,12,16,20,24)},'same_task_resource':True,'name_retained':m.wstr(m.r(resource+0x38))})
  assert m.r(resource)==0xa45290 and m.r(resource+8)==0x10
 #Descriptor NAME used only by a required diagnostic is typed input; the
 #positive exact class comparison needs no synthesized parent chain.
 m.w(0xb3d9a8,m.cstring(m.raw_wstr(0xa4659c)))
 descriptor={'base':'0xb3d968','factory':'0x5a5fb0','scope':'9D0710 metadata-only'}
 cases=[];current={}
 def find(m,u):current['trace'].append('cached');m.ret(current['resource']if current['cached']else 0,pop=4)
 def create(m,u):current['trace'].append('source');m.ret(current['resource'],pop=8)
 def warn(m,u):
  current['trace'].append('diagnostic');raise Required()
 m.stubs[0x5ad430]=find;m.stubs[0x5aea90]=create;m.stubs[0x425210]=warn;m.stubs[0x4252a0]=warn
 for cached,present in itertools.product((False,True),(False,True)):
  current={'trace':[],'cached':cached,'resource':allocations[0]if present else 0}
  slot=m.alloc(4);m.w(slot,m.cstring('Art/idle.kf'));out=None;stop=None
  try:out=m.cdecl(0x5a80d0,[slot])
  except Required:stop='diagnostic_required'
  cases.append({'cached':cached,'present':present,'same_resource':out==allocations[0]if out else None,'stop':stop,'trace':current['trace']})
 return {'constructors':rows,'descriptor':descriptor,'lookup':cases,'global_priority':m.r(0xb282f0)},pins,seams
def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append({'precision':pc,'output':a});pins.update(pa);seams|=sa|sb
 for a,b in pins.items():assert bytes.fromhex(b)==h.P.range(a,len(bytes.fromhex(b)))==h.V.range(a,len(bytes.fromhex(b)))
 metadata=h.P.range(0x9d0710,0x2a);assert metadata==h.V.range(0x9d0710,len(metadata))
 doc={'images':{'original':hashlib.sha256((ROOT.parent/'AA.exe').read_bytes()).hexdigest(),'v14':hashlib.sha256((ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe').read_bytes()).hexdigest()},'scope':__doc__,'rows':rows,'executed_pins':{hex(a):b for a,b in sorted(pins.items())},'intercepted_not_executed':[hex(a)for a in sorted(seams)],'metadata_only':{'0x9d0710':metadata.hex()},'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
 path=HERE/'native-sequence.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8',newline='\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':2*len(rows),'constructor_calls':18,'lookup_calls':24,'genuine_pins':len(pins)}))
if __name__=='__main__':main()
