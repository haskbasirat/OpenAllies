"""Real parser/4B3C10 Bool ownership and raw setter, followed by actual
87A1E5 indexed52 read and5A9F49 indexed54 short-circuit branch. The pointer
table binds those actual allocations at their separately captured registered
indices; this is not whole213 registration or geometry-normal removal.
Inherited parser/heap/CString/mutex transports are explicit, not Bool seams.
"""
from pathlib import Path
import sys,json,hashlib,itertools,argparse
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'))
import native_field_order as F
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_ESI,UC_X86_REG_ECX
NAMES=['RendererRemoveNormals','ResourceCaching']
class Done(Exception):pass

def run(image,pc,override):
 F.T.P=image;m=F.Machine(b'',pc);pins={};seams=set();active=[None];stop=[None]
 def executed(u,at,size,_):
  if active[0] and at in active[0]:stop[0]=at;raise Done()
  if F.ORIGINAL.base<=at<F.ORIGINAL.base+F.ORIGINAL.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,executed);settings=0xb39938;m.invoke(0x4b2ce0,settings,[])
 stages=[];owners=None
 for values in [(False,True),override]:
  lines=[f'flag {n} = {str(v).lower()}'for n,v in zip(NAMES,values)]
  m.lines=F.parser.lines_of(('[Vars]\n{\n'+'\n'.join(lines)+'\n}\n').encode());m.at=0;m.thrown=None;m.messages=[]
  parsed=m.parse();assert parsed['error']is None,parsed
  node=m.r(m.r(m.parsed_root+0x34));m.invoke(0x4b3c10,settings,[node]);assert m.r(settings+12)==2
  current=[m.invoke(0x4b2e60,settings,[m.string(list(map(ord,n))),1])for n in NAMES]
  assert owners is None or owners==current;owners=current
  row=[dict(name=n,current=m.r(p)&255,default=(m.r(p)>>8)&255,kind=m.r(p+0x24),flags=m.r(p+0x28))for n,p in zip(NAMES,owners)]
  assert all(v['kind']==1 and v['flags']==0 for v in row);stages.append(row)
 table=m.alloc(213*4);m.w(0xb39940,table)
 for index,owner in zip([52,54],owners):m.w(table+4*index,owner)
 samples=[]
 for value in [0,1,2,128,255]:
  for owner in owners:m.invoke(0x4b3190,owner,[value])
  active[0]={0x87a1f5,0x87a219};stop[0]=None;m.u.reg_write(UC_X86_REG_ESP,m.stack)
  try:m.u.emu_start(0x87a1e5,m.stop,count=100)
  except Done:pass
  assert stop[0]==(0x87a1f5 if value else 0x87a219)
  branches=[]
  for flags in [0,4]:
   #5A9F49 reads actual Resource flags8, not a fake Bool argument.
   resource=m.alloc(0x90);m.w(resource+8,flags);m.u.reg_write(UC_X86_REG_ESI,resource)
   m.u.reg_write(UC_X86_REG_ECX,0);m.u.reg_write(UC_X86_REG_ESP,m.stack)
   from unicorn.x86_const import UC_X86_REG_EAX
   m.u.reg_write(UC_X86_REG_EAX,flags);active[0]={0x5a9f6b,0x5aa042};stop[0]=None
   try:m.u.emu_start(0x5a9f49,m.stop,count=100)
   except Done:pass
   assert stop[0]==(0x5aa042 if flags&4 and value else 0x5a9f6b),(flags,value,stop)
   branches.append(dict(flags=flags,skip_bounds=stop[0]==0x5aa042))
  samples.append(dict(requested=value,current=[m.r(p)&255 for p in owners],default=[(m.r(p)>>8)&255 for p in owners],remove_normals=value!=0,branches=branches))
 return dict(stages=stages,samples=samples,same_variable_allocation=True),pins,seams

def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');a=p.parse_args();rows=[];pins={};seams=set()
 registration_path=ROOT/'research/skirmish-20261017-view-init/native-uvar-registration.json';registration=json.loads(registration_path.read_bytes())
 for row in registration['cases']:
  for index,initial in [(52,0),(54,1)]:
   r=row['registrations'][index];assert ''.join(map(chr,r['name']))==NAMES[index==54] and r['kind']==1 and r['flags']==0 and r['bool_current']==r['bool_default']==initial
 raw=(GAME/'reconstruction/assets/original/Data/UVars.tgi').read_bytes();lines=[line.strip().decode('ascii')for line in raw.splitlines()if any(('flag '+n+' =').encode()in line for n in NAMES)]
 assert lines==['flag RendererRemoveNormals = false','flag ResourceCaching = true']
 for override,pc in itertools.product([(False,True),(True,False),(True,True),(False,False)],F.PCS):
  left,lp,ls=run(F.ORIGINAL,pc,override);right,rp,rs=run(F.EXPANDED,pc,override)
  assert left==right and lp==rp;rows.append(dict(precision=pc,override=list(override),native=left));pins.update(lp);seams|=ls|rs
 for at,raw_pin in pins.items():assert F.ORIGINAL.range(at,len(bytes.fromhex(raw_pin))).hex()==F.EXPANDED.range(at,len(bytes.fromhex(raw_pin))).hex()==raw_pin
 out=dict(scope=__doc__,rows=rows,registration_sha256=hashlib.sha256(registration_path.read_bytes()).hexdigest(),actual_source_lines=lines,actual_source_sha256=hashlib.sha256(raw).hexdigest(),instructions={hex(at):raw for at,raw in sorted(pins.items())},transport_entries=[hex(v)for v in sorted(seams)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-model-uvars.json'
 if a.create:
  with path.open('x')as file:json.dump(out,file,indent=2);file.write('\n')
 else:assert json.loads(path.read_bytes())==out
 print(json.dumps(dict(paired_rows=len(rows),executions=2*len(rows),pins=len(pins),fixture_sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
