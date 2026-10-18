"""Actual post-resource task gate/same-resource assignment and4FDCE0 prefix.
Task intervals execute real4F02F0; a reached4FDCE0 is an explicit call boundary,
not a claimed successful mission. The separate prefix executes actual all-slot
iterator/World44 reads, with logging/merge/Kingdom-start/application-getter
transport seams. It stops before required warning/editor/map_prepare children.
No source-map restoration, native allocation or application-ready claim.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
ORIGINAL=h.P
class Required(Exception):pass

def gate(image,pc,mode,started,same):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);events=[];seams=set()
 def seam(a,f):m.seams[a]=f;seams.add(a)
 def start(_):events.append('4FDCE0_boundary');m.ret(value=1)
 def other(_):events.append('required_5A1230');raise Required()
 seam(0x4fdce0,start);seam(0x5a1230,other)
 world=m.alloc(0x300);resource=m.alloc(0xb0);task=m.alloc(0x30)
 m.w32(0xb36534,world);m.w32(world+0x48,started);prior_resource=resource if same else m.alloc(0xb0);m.w32(world+0x28,prior_resource);m.w32(task+0x24,resource)
 m.u.reg_write(UC_X86_REG_ESP,h.STACK);m.u.reg_write(UC_X86_REG_ESI,task)
 begin,end={1:(0x5a3ed0,0x5a3f19),2:(0x5a3e93,0x5a3ea2),4:(0x5a3f40,0x5a3f89)}[mode];outcome='ok'
 try:m.u.emu_start(begin,end,count=10000);assert m.u.reg_read(UC_X86_REG_EIP)==end
 except Required:outcome='required_5A1230'
 return {'events':events,'outcome':outcome,'world_resource_unchanged':m.r32(world+0x28)==prior_resource},seams

def prefix(image,pc,verified,appmode,slots):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);events=[];seams=set();kingdoms={}
 def seam(a,f):m.seams[a]=f;seams.add(a)
 seam(0x404f60,lambda _:m.ret())
 seam(0x4f57f0,lambda _:(events.append('merge'),m.ret())[-1])
 seam(0x4e6db0,lambda _:(events.append(['kingdom',kingdoms[m.ecx()]]),m.ret())[-1])
 app=m.alloc(0x20);m.w32(app+12,appmode);seam(0x40d960,lambda _:m.ret(value=app))
 def stop(label):
  def f(_):events.append(label);raise Required(label)
  return f
 for a,label in [(0x401b00,'required_warning'),(0x938960,'required_editor'),(0x4fda70,'required_map_prepare')]:seam(a,stop(label))
 world=m.alloc(0x300);m.w32(0xb36534,world);m.w32(world+0x44,verified)
 array=m.alloc(max(1,len(slots))*4);m.w32(world+0x148,array,len(slots),len(slots))
 for i,value in enumerate(slots):
  if value is not None:p=m.alloc(0x400);kingdoms[p]=value;m.w32(array+4*i,p)
 outcome='unexpected_return'
 try:m.call(0x4fdce0,world)
 except Required as e:outcome=str(e)
 return {'events':events,'outcome':outcome},seams

def main():
 gates=[];prefixes=[];seams=set()
 for image,pc in itertools.product(('original','v14'),h.PRECISION):
  for mode,started,same in itertools.product((1,2,4),(0,1,0xffffffff),(False,True)):
   output,used=gate(image,pc,mode,started,same);seams|=used
   gates.append({'image':image,'precision':pc,'input':{'mode':mode,'started48':started,'same':same},'output':output})
  for verified,appmode,slots in itertools.product((1,2,0xffffffff),(1,4),([], [7,None,9])):
   output,used=prefix(image,pc,verified,appmode,slots);seams|=used
   prefixes.append({'image':image,'precision':pc,'input':{'verified44':verified,'application_mode':appmode,'slots':slots},'output':output})
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'seams_not_executed':[hex(a)for a in sorted(seams)],
  'executed_instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a not in seams],
  'task_gates':gates,'prefixes':prefixes}
 p=HERE/'native-mission-prefix.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'task_gates':len(gates),'prefixes':len(prefixes),'executed_pins':len(doc['executed_instructions']),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
