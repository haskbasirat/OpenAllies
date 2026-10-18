"""Real41F9D0 prefix and appended quality Bool constructors/registration.
The complete actual Settings hash/vector/register bodies execute through
ForceLow publication. Real5A8BBC..5A8C39 quality lookup/current-byte prefix
and NiSourceTexture87BE40 mode store execute; stop before resource sourceIO.
Inherited heap/CString/mutex transports only. No graphics-device writer,
file decoder, Ni creation or GPU proof.
"""
from pathlib import Path
import sys,json,itertools,argparse,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_field_order as F
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESI,UC_X86_REG_ESP
FORCE='RendererForceLowQualityTextures';HIGH='RendererUseHighQualityTextures'
class Done(Exception):pass

def run(image,pc):
 F.T.P=image;m=F.Machine(b'',pc);pins={};seams=set();phase=['ctor'];reads=[]
 def observed(u,at,size,_):
  if phase[0]=='ctor'and at==0x4243eb:raise Done()
  if phase[0]=='policy'and at==0x5a8c39:raise Done()
  if phase[0]=='policy'and at==0x405370:reads.append(m.text(m.r(m.arg())))
  if F.ORIGINAL.base<=at<F.ORIGINAL.base+F.ORIGINAL.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,observed)
 #Unrelated RenderStandardFont registration copies the actual shared empty
 #CString. Execute its native constructor instead of supplying pointer bytes.
 m.invoke(0x4013e0,0xb43b40,[])
 settings=0xb39938;m.invoke(0x4b2ce0,settings,[])
 try:m.invoke(0x41f9d0,settings,[])
 except Done:pass
 except Exception:
  print('ctor recent',[hex(p)for p in m.recent],'EIP',hex(m.u.reg_read(UC_X86_REG_EIP)),'count',m.r(settings+12),flush=True);raise
 else:raise AssertionError('constructor did not reach ForceLow publication')
 named={};constructors=[]
 for i in range(m.r(settings+12)):
  p=m.r(m.r(settings+8)+4*i);name=m.text(m.r(p+0x1c))
  if name in(FORCE,HIGH):
   named[name]=p;constructors.append(dict(name=name,kind=m.r(p+0x24),flags=m.r(p+0x28),current=m.r(p)&255,default=(m.r(p)>>8)&255))
 assert len(named)==2
 assert constructors==[dict(name=HIGH,kind=1,flags=16,current=1,default=1),dict(name=FORCE,kind=1,flags=16,current=0,default=0)]
 source=m.alloc(0x100);vt=m.alloc(0x60);m.w(source,vt);m.w(vt+0x3c,0x87be40)
 phase[0]='policy';samples=[]
 for force,high,before in itertools.product([0,1,2,128,255],[0,1,2,128,255],[0,0xcccccccc]):
  m.invoke(0x4b3190,named[FORCE],[force]);m.invoke(0x4b3190,named[HIGH],[high]);m.w(source+0x48,before);reads.clear()
  m.u.reg_write(UC_X86_REG_ESI,source);m.u.reg_write(UC_X86_REG_ESP,m.stack)
  try:m.u.emu_start(0x5a8bbc,m.stop,count=4000000)
  except Done:pass
  else:raise AssertionError('policy did not reach declared boundary')
  expected=[FORCE]+([HIGH]if force==0 else[]);assert reads==expected,reads
  mode=m.r(source+0x48);assert mode==(1 if force!=0 or high==0 else before),(force,high,before,mode)
  samples.append(dict(force=force,high=high,before_mode=before,after_mode=mode,reads=list(reads)))
 return dict(constructors=constructors,registered_count_before_stop=m.r(settings+12),samples=samples),pins,seams

def main():
 parser=argparse.ArgumentParser();parser.add_argument('--create',action='store_true');args=parser.parse_args();rows=[];pins={};seams=set()
 for pc in F.PCS:
  a,ap,as_=run(F.ORIGINAL,pc);b,bp,bs=run(F.EXPANDED,pc);assert a==b and ap==bp;rows.append(dict(precision=pc,native=a));pins.update(ap);seams|=as_|bs
 for at,raw in pins.items():assert F.ORIGINAL.range(at,len(bytes.fromhex(raw))).hex()==F.EXPANDED.range(at,len(bytes.fromhex(raw))).hex()==raw
 out=dict(scope=__doc__,rows=rows,executed_instructions={hex(at):raw for at,raw in sorted(pins.items())},transport_entries=[hex(at)for at in sorted(seams)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-quality-uvars.json'
 if args.create:
  with path.open('x')as file:json.dump(out,file,indent=2);file.write('\n')
 else:assert json.loads(path.read_bytes())==out
 print(json.dumps(dict(paired_rows=len(rows),separate_executions=2*len(rows),policy_samples_per_machine=50,pins=len(pins),fixture_sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
