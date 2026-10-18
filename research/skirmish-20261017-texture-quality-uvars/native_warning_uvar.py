"""Actual full41F9D0 registration, UVars*.tgi source writes and warning read.
Real4B3C10 updates SAME named Bool. Real5A6E90 prefix selects diagnostic
level1 for nonzero or10 for zero, then stops BEFORE manager/texture factory.
Inherited line/heap/CString/mutex/logger transports; no default texture or
GPU-ready proof. All source files are actual original/v14 corpus bytes.
"""
from pathlib import Path
import sys,json,hashlib,argparse
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_field_order as F
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP
NAME='DataMissingResourceWarning'
class Done(Exception):pass

def sources():
 original=GAME/'reconstruction/assets/original/Data';overlay=GAME/'reconstruction/native-game/runtime-expanded-v14/Data'
 for name in sorted({p.name for folder in[original,overlay]for p in folder.glob('UVars*.tgi')}):
  path=overlay/name
  if not path.is_file():path=original/name
  yield name,path.read_bytes()

def run(image,pc):
 F.T.P=image;m=F.Machine(b'',pc);pins={};seams=set();phase=['ctor'];levels=[]
 def observed(u,at,size,_):
  if phase[0]=='warning'and at==0x404fc0:
   levels.append(m.arg(1));raise Done()
  if F.ORIGINAL.base<=at<F.ORIGINAL.base+F.ORIGINAL.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,observed);m.invoke(0x4013e0,0xb43b40,[]);settings=0xb39938
 m.invoke(0x4b2ce0,settings,[]);m.invoke(0x41f9d0,settings,[])
 variable=None
 for i in range(m.r(settings+12)):
  p=m.r(m.r(settings+8)+4*i)
  if m.text(m.r(p+0x1c))==NAME:assert variable is None;variable=p
 assert variable
 def snapshot():return dict(kind=m.r(variable+0x24),flags=m.r(variable+0x28),current=m.r(variable)&255,default=(m.r(variable)>>8)&255)
 initial=snapshot();assert initial==dict(kind=1,flags=0,current=1,default=1),initial
 stages=[]
 for name,raw in sources():
  m.lines=F.parser.lines_of(raw);m.at=0;m.thrown=None;m.messages=[];parsed=m.parse();assert parsed['error']is None,parsed
  for i in range(m.r(m.parsed_root+0x38)):
   node=m.r(m.r(m.parsed_root+0x34)+4*i)
   if m.text(m.r(node)).lower()=='vars':m.invoke(0x4b3c10,settings,[node])
  stages.append(dict(file=name,sha256=hashlib.sha256(raw).hexdigest(),value=snapshot()))
 assert [s['value']['current']for s in stages]==[1,0,0],stages
 phase[0]='warning';samples=[]
 for raw in [0,1,2,128,255]:
  m.invoke(0x4b3190,variable,[raw])
  try:m.invoke(0x5a6e90,0,[m.string(list(map(ord,'missing-test.dds')))])
  except Done:pass
  else:raise AssertionError('warning failed to reach diagnostic boundary')
  assert levels[-1]==(1 if raw else 10)
  samples.append(dict(current=raw,diagnostic_level=levels[-1],value=snapshot()))
 return dict(initial=initial,stages=stages,samples=samples),pins,seams

def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');p.add_argument('--probe',action='store_true');args=p.parse_args();rows=[];pins={};seams=set()
 for pc in(['Double53']if args.probe else F.PCS):
  a,ap,as_=run(F.ORIGINAL,pc);b,bp,bs=run(F.EXPANDED,pc);assert a==b and ap==bp;rows.append(dict(precision=pc,native=a));pins.update(ap);seams|=as_|bs
 for at,raw in pins.items():assert F.ORIGINAL.range(at,len(bytes.fromhex(raw))).hex()==F.EXPANDED.range(at,len(bytes.fromhex(raw))).hex()==raw
 out=dict(scope=__doc__,rows=rows,executed_instructions={hex(at):raw for at,raw in sorted(pins.items())},transport_entries=[hex(at)for at in sorted(seams)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 if args.probe:print(json.dumps(dict(probe=True,rows=rows)));return
 path=HERE/'native-warning-uvar.json'
 if args.create:
  with path.open('x')as file:json.dump(out,file,indent=2);file.write('\n')
 else:assert json.loads(path.read_bytes())==out
 print(json.dumps(dict(paired_rows=len(rows),separate_machines=2*len(rows),warning_samples_per_machine=5,pins=len(pins),fixture_sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
