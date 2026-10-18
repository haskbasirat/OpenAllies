"""Whole parser/4B3C10 source-created named Bool and real4B2E60 lookup.
Real5A8D3F..5A8D80 cache/read/flag prefix is executed, ending BEFORE the
texture decoder branch. Heap/CString/input-line/mutex/logger transports are
the inherited explicitly declared parser transports, not Bool/lookup seams.
No whole213-UVar registration, profile, texture decode or GPU claim.
"""
from pathlib import Path
import sys,json,hashlib,itertools,argparse
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'))
import native_field_order as F
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESI,UC_X86_REG_EBP,UC_X86_REG_ESP
NAME='ResourceNoNiTextures'
class Done(Exception):pass

def cases():
 for first,override in [(False,None),(True,None),(False,True),(True,False)]:
  yield dict(name=f'initial{first}_override{override}',sources=[f'flag {NAME} = {str(first).lower()}']+([]if override is None else[f'{NAME} = {str(override).lower()}']))
 #Use the exact authored line as input, not a manufactured default.
 path=GAME/'reconstruction/assets/original/Data/UVars.tgi';raw=path.read_bytes()
 lines=[line.strip().decode('ascii')for line in raw.splitlines()if b'flag ResourceNoNiTextures ='in line]
 assert len(lines)==1
 yield dict(name='actual_UVars_tgi_line112',sources=lines,source_file=str(path.relative_to(GAME)),source_sha256=hashlib.sha256(raw).hexdigest())

def run(image,pc,case):
 F.T.P=image;m=F.Machine(b'',pc);pins={};seams=set();lookups=[];stop=[False]
 def executed(u,at,size,_):
  if stop[0]and at==0x5a8d80:raise Done()
  if at==0x4b2e60:lookups.append([m.text(m.arg()),m.arg(1)])
  if F.ORIGINAL.base<=at<F.ORIGINAL.base+F.ORIGINAL.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,executed)
 settings=0xb39938;m.invoke(0x4b2ce0,settings,[]);stages=[];variable=None
 for declaration in case['sources']:
  source=('[Vars]\n{\n'+declaration+'\n}\n').encode()
  m.lines=F.parser.lines_of(source);m.at=0;m.thrown=None;m.messages=[]
  parsed=m.parse();assert parsed['error']is None,parsed
  node=m.r(m.r(m.parsed_root+0x34));m.invoke(0x4b3c10,settings,[node])
  assert m.r(settings+12)==1
  current=m.r(m.r(settings+8));assert variable is None or current==variable;variable=current
  stages.append(dict(current=m.r(variable)&255,default=(m.r(variable)>>8)&255,kind=m.r(variable+0x24),flags=m.r(variable+0x28),name=m.text(m.r(variable+0x1c))))
 assert all(r['kind']==1 and r['flags']==0 and r['name']==NAME for r in stages)
 looked_up=m.invoke(0x4b2e60,settings,[m.string(list(map(ord,NAME))),1]);assert looked_up==variable
 m.w(0xb3d8f8,0);m.w(0xb3d8f4,0xcccccccc)
 resource=m.alloc(0x80);samples=[];stop[0]=True
 for value,flags in itertools.product([0,1,2,128,255],[0,0x40]):
  m.invoke(0x4b3190,variable,[value]);assert m.r(variable)&255==value
  m.w(resource+8,flags);m.u.reg_write(UC_X86_REG_ESI,resource);m.u.reg_write(UC_X86_REG_EBP,0);m.u.reg_write(UC_X86_REG_ESP,m.stack)
  try:m.u.emu_start(0x5a8d3f,m.stop,count=4000000)
  except Done:pass
  else:raise AssertionError('prefix did not reach declared boundary')
  result=m.r(resource+8);assert result==(flags|0x40 if value==1 else flags)
  assert m.r(0xb3d8f4)==variable
  samples.append(dict(requested=value,before_flags=flags,current=m.r(variable)&255,default=(m.r(variable)>>8)&255,after_flags=result))
 assert lookups==[[NAME,1],[NAME,1]],lookups
 return dict(stages=stages,samples=samples,lookups=lookups,cached_pointer_is_same_allocation=True),pins,seams

def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');a=p.parse_args();rows=[];pins={};seams=set()
 assert F.ORIGINAL.range(0xa461d8,2*(len(NAME)+1))==F.EXPANDED.range(0xa461d8,2*(len(NAME)+1))==(NAME+'\0').encode('utf-16-le')
 for case,pc in itertools.product(cases(),F.PCS):
  left,lp,ls=run(F.ORIGINAL,pc,case);right,rp,rs=run(F.EXPANDED,pc,case)
  assert left==right and lp==rp;rows.append(dict(input=case,precision=pc,native=left));pins.update(lp);seams|=ls|rs
 for at,raw in pins.items():assert F.ORIGINAL.range(at,len(bytes.fromhex(raw))).hex()==F.EXPANDED.range(at,len(bytes.fromhex(raw))).hex()==raw
 out=dict(scope=__doc__,rows=rows,instructions={hex(at):raw for at,raw in sorted(pins.items())},transport_entries=[hex(v)for v in sorted(seams)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-texture-uvar.json'
 if a.create:
  with path.open('x')as file:json.dump(out,file,indent=2);file.write('\n')
 else:assert json.loads(path.read_bytes())==out
 print(json.dumps(dict(paired_rows=len(rows),separate_executions=2*len(rows),pins=len(pins),fixture_sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
