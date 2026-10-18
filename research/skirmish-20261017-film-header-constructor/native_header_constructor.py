"""Actual fresh5A23F0(false) Header constructor +5A14C0 metadata/reset.
The independent511DF0 WorldRules and50F890 Parameters constructors execute.
Existing CString/allocator/current-var/logging transport seams are inherited
from the pinned Creator constructor harness. Explicit6FB1F0 supplied-clock
seam or required BEFORE-body stop; no actual Windows clock/source-application
metadata producer is claimed. No game file/process is modified.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-mission-start'))
from native_creator_constructor import Machine as Base,h
from unicorn.x86_const import UC_X86_REG_EDX
class Stop(Exception):pass

def run(case,image,pc):
 m=Base(image,pc,dict(width=192,height=192,independent=0,resource_count=3))
 for address,name in [(0xb3888c,'version'),(0xb388a0,'application'),(0xb38894,'release')]:m.cs(address,case[name])
 m.w(0xb3650c,case['numeric']);m.w(0xb3d8cc,case['count'])
 events=[];stopped=None
 def clock():
  events.append('6FB1F0')
  if case['stop']=='clock':raise Stop('6FB1F0')
  m.u.reg_write(UC_X86_REG_EDX,case['time'][1]);m.ret(0,case['time'][0])
 m.seams[0x6fb1f0]=clock
 if case['stop'] in ('rules','parameters'):
  address=0x511df0 if case['stop']=='rules'else 0x50f890
  def stop():raise Stop(case['stop'])
  m.seams[address]=stop
 target=m.alloc(0x68);m.u.mem_write(target,b'\xcd'*0x68)
 try:
  returned=m.call(0x5a23f0,[0],this=target,callee_pop=4);assert returned==target
 except Stop as e:stopped=str(e)
 rules=m.r(target+4);params=m.r(target+8)
 out=dict(stopped=stopped,count=m.r(0xb3d8cc),events=events,
  words={str(off):m.r(target+off)for off in [0xc,0x10,0x14,0x18,0x1c,0x30,0x34,0x40,0x48,0x4c,0x50,0x54]},
  byte64=None if case['stop']=='clock'else m.rb(target+0x64),
  strings={str(off):m.rs(target+off)for off in [0x20,0x24,0x28,0x2c,0x38,0x3c,0x58,0x5c,0x60]},
  rules=None if not rules else m.read_wc(rules),
  parameters=None if not params else m.read_header(params),
  rules_present=rules!=0,parameters_present=params!=0)
 if stopped is None:
  old_rules=rules;old_params=params
  other=m.alloc(0x68);m.u.mem_write(other,b'\xcd'*0x68)
  returned=m.call(0x5a23f0,[0],this=other,callee_pop=4);assert returned==other
  out['second_count']=m.r(0xb3d8cc)
  out['independent_rules']=m.r(other+4)!=old_rules
  out['independent_parameters']=m.r(other+8)!=old_params
  out['independent_rules_arrays']=m.r(m.r(other+4)+0xc)!=m.r(old_rules+0xc)
  out['independent_team_arrays']=m.r(m.r(other+8)+4)!=m.r(old_params+4)
  out['independent_kingdom_arrays']=m.r(m.r(other+8)+0x10)!=m.r(old_params+0x10)
 return out,m.visited,m.intercepted

def cases():
 for count,time in [(0,[0,0]),(1,[0x12345678,0x9abcdef0]),(0xffffffff,[0xffffffff,0xffffffff])]:
  for stop in ['none','clock','rules','parameters']:
   yield dict(version='1.0.5.0',application='A&A',release='English AA',numeric=1000500,count=count,time=time,stop=stop)

def main():
 rows=[];visited={};seams=set()
 for pc in h.PRECISION:
  for case in cases():
   a,pa,sa=run(case,'original',pc);b,pb,sb=run(case,'v14',pc);assert a==b,(case,pc,a,b)
   visited.update(pa);visited.update(pb);seams|=sa|sb;rows.append(dict(input=case,precision=pc,output=a))
 pins=[];boundaries={0x6fb1f0,0x511df0,0x50f890}
 #A stopped boundary might execute in another case: distinguish the stopped
 #entry from executed instructions rather than suppressing its real body.
 for a,n in sorted(visited.items()):
  if a in seams and a not in {0x511df0,0x50f890}:continue
  if not(0x401000<=a<0x9ef000):continue
  raw=h.P.range(a,n);assert raw==h.V.range(a,n);pins.append(dict(va=hex(a),bytes=raw.hex()))
 doc=dict(schema=1,scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  cases=rows,executions=2*len(rows),executed_instructions=pins,
  stopped_boundaries=[dict(va=hex(a),bytes=h.P.range(a,6).hex())for a in sorted(boundaries)],
  successful_seam_entries=[hex(a)for a in sorted(seams-{0x511df0,0x50f890})],
  successful_clock_transport='6FB1F0 returns supplied low/high only; no Windows clock body',
  inspected_caller_pins=[dict(va=hex(a),bytes=h.P.range(a,n).hex())for a,n in [(0x5a31f1,12),(0x5a2874,12)]])
 path=HERE/'native-header-constructor.json'
 if '--write'in sys.argv:
  with path.open('x',encoding='utf-8')as f:f.write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text(encoding='utf-8'))==doc
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
