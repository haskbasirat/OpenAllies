"""Actual135 native5065D0 registrations then both source files4B3C10.
Uses the prior whole-parser host line/EOF/heap/string/mutex transports.
Registration/Settingshash/vector/add/typed setter bodies execute; no captured
current values or registration transport is supplied. No WorldRules/reset,
changed process policies, failed allocation/throw or running-world claim.
"""
from pathlib import Path
import sys,json,hashlib
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];GAME=ROOT.parent
sys.path.insert(0,str(HERE));import native_field_order as order
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP

def snapshot(m,settings):
 out=[];array=m.r(settings+8)
 for index in range(m.r(settings+12)):
  var=m.r(array+index*4);kind=m.r(var+0x24);flags=m.r(var+0x28)
  value=([m.r(var)&255]if kind==1 else[m.r(var)]if kind in(2,3)else[m.r(var+4+i*4)for i in range(3)]if kind==5 else m.text(m.r(var+0x10)))
  assert kind in(1,2,3,4,5)
  out.append({'index':index,'name':m.text(m.r(var+0x1c)),'kind':kind,'flags':flags,'value':value,'bounds':[m.r(var+0x14),m.r(var+0x18)]if kind in(2,3)else None})
 return out

def files():
 out=[]
 for name in['Game/SVars.tgi','Game/SVars_AA.tgi']:
  overlay=GAME/'reconstruction/native-game/runtime-expanded-v14/Data'/name
  path=overlay if overlay.is_file()else GAME/'reconstruction/assets/original/Data'/name
  raw=path.read_bytes();out.append((name,raw))
 return out

def run(image,pc):
 order.T.P=image;m=order.Machine(b'',pc);pins={};seams=set()
 def executed(u,at,size,_):
  if order.ORIGINAL.base<=at<order.ORIGINAL.base+order.ORIGINAL.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,executed)
 settings=m.alloc(0x80);m.invoke(0x4b2ce0,settings,[]);m.invoke(0x5065d0,settings,[])
 initial=snapshot(m,settings);assert len(initial)==135
 stages=[]
 for name,source in files():
  m.lines=order.parser.lines_of(source);m.at=0;m.thrown=None;m.messages=[]
  parsed=m.parse();assert parsed['error']is None,parsed
  root=m.parsed_root;assert m.r(root+0x38)==1
  node=m.r(m.r(root+0x34));m.invoke(0x4b3c10,settings,[node])
  stages.append({'file':name,'sha256':hashlib.sha256(source).hexdigest(),'source_field_count':m.r(node+0x48),'registrations':snapshot(m,settings)})
 assert len(stages[-1]['registrations'])==231,len(stages[-1]['registrations'])
 return {'constructors':initial,'stages':stages},pins,seams

def main():
 rows=[];pins={};seams=set()
 for pc in(['Double53']if '--probe'in sys.argv else order.PCS):
  a,pa,sa=run(order.ORIGINAL,pc);b,pb,sb=run(order.EXPANDED,pc);assert a==b and pa==pb
  rows.append({'precision':pc,'native':a});pins.update(pa);seams|=sa|sb
 if '--probe'in sys.argv:print(json.dumps({'probe':True,'constructor_count':135,'final_count':len(rows[0]['native']['stages'][-1]['registrations'])}));return
 for va,raw in pins.items():assert order.ORIGINAL.range(va,len(bytes.fromhex(raw)))==order.EXPANDED.range(va,len(bytes.fromhex(raw)))==bytes.fromhex(raw)
 out={'schema':1,'scope':__doc__,'rows':rows,'paired_rows':len(rows),'native_executions':len(rows)*2,
  'executed_instruction_pins':{f'{va:08x}':raw for va,raw in sorted(pins.items())},'successful_host_transport_entries':[f'{a:08x}'for a in sorted(seams)],
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'hash_parser_harness_sha256':hashlib.sha256((HERE/'native_field_order.py').read_bytes()).hexdigest()}
 target=HERE/'native-full-sources.json'
 if '--write'in sys.argv:
  with target.open('x')as f:json.dump(out,f,indent=1);f.write('\n')
 else:assert json.loads(target.read_bytes())==out
 print(json.dumps({'rows':len(rows),'executions':2*len(rows),'constructor_count':135,'final_count':231,'pins':len(pins),'sha256':hashlib.sha256(target.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
