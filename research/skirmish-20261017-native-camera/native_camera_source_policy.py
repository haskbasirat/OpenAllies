"""Actual parser+4B3C10 flags2/34 Camera source skip and continuation.

Registered Float tuples are laboratory prefills, separately backed by the
executed AVar constructor fixture. Real parser/Settings lookup/loop/setter run.
404ED0 diagnostic output is an explicit observer/transport; source Float getter
is watched to distinguish a genuine skip from parsing then discarding.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'))
import native_field_order as F
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ECX
class Machine(F.Machine):
 def __init__(self,source,pc):self.diagnostics=[];self.getters=[];self.watched={};super().__init__(source,pc)
 def hook(self,u,pc,size,z):
  if pc==0x404ed0:
   self.diagnostics.append(dict(line=self.arg(2),name=self.text(self.arg(5)),format=self.text(self.arg(4))));self.ret()
  else:
   if pc in self.watched:self.getters.append(self.watched[pc])
   super().hook(u,pc,size,z)
def run(image,pc,case):
 F.T.P=image;source=('[Vars]\n{\n'+case['declaration']+'\nfloat CameraZoomMax = 1.25\n}\n').encode();m=Machine(source,pc);pins={};seams=set()
 def executed(u,at,size,_):
  if F.ORIGINAL.base<=at<F.ORIGINAL.base+F.ORIGINAL.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,executed);parsed=m.parse();assert parsed['error']is None;root=m.parsed_root;node=m.r(m.r(root+0x34));settings=m.alloc(0x80);m.invoke(0x4b2ce0,settings,[])
 vars=[]
 for i,(name,word,flags)in enumerate([('CameraHeight',0x42c80000,case['flags']),('CameraZoomMax',0x3fc00000,0)]):
  var=m.alloc(0x2c)
  for off,value in[(0,word),(0x10,m.string([])),(0x14,0x41200000),(0x18,0x43c80000),(0x1c,m.string(list(map(ord,name)))),(0x24,2),(0x28,flags)]:m.w(var+off,value)
  m.invoke(0x4b3470,settings,[i,var]);vars.append(m.r(m.r(settings+8)+4*i))
 # A shared virtual Float getter address can serve both fields: bind the
 # receiver identity in the observer rather than invent a per-field stub.
 current=m.r(node+0x40);cap=m.r(node+0x44);height_field=None
 for slot in range(cap):
  link=m.r(current+4*slot)
  while link:
   f=m.r(link)
   if m.text(m.r(f+4))=='CameraHeight':height_field=f
   link=m.r(link+4)
 assert height_field
 getter=m.r(m.r(height_field)+8)
 def observe_getter(u,at,size,_):
  if at==getter and m.u.reg_read(UC_X86_REG_ECX)==height_field:m.getters.append('CameraHeight numeric getter')
 m.u.hook_add(UC_HOOK_CODE,observe_getter);m.invoke(0x4b3c10,settings,[node])
 result=dict(height=m.r(vars[0]),zoom=m.r(vars[1]),diagnostics=m.diagnostics,height_numeric_getter_calls=len(m.getters))
 assert result['height']==0x42c80000 and result['zoom']==0x3fa00000 and not m.getters,result
 return result,pins,seams
def main():
 rows=[];pins={};seams=set()
 for flags,raw in itertools.product((2,34),('100','999')):
  c=dict(flags=flags,declaration='float CameraHeight = '+raw)
  for pc in F.PCS:
   a,pa,sa=run(F.ORIGINAL,pc,c);b,pb,sb=run(F.EXPANDED,pc,c);assert a==b and pa==pb;rows.append(dict(input=c,precision=pc,output=a));pins.update(pa);seams|=sa|sb
 for va,b in pins.items():assert F.ORIGINAL.range(va,len(bytes.fromhex(b))).hex()==F.EXPANDED.range(va,len(bytes.fromhex(b))).hex()==b
 doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,executions=len(rows)*2,instructions={hex(a):b for a,b in sorted(pins.items())},transport_entries=[hex(a)for a in sorted(seams)])
 path=HERE/'native-camera-source-policy.json'
 if'--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
