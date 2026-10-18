"""Actual parser/Settings creates source Bool and assigns registered C0Point3.
Registered C0 tuple is a lab prefill separately backed by real401BD0 capture.
No standalone source-declared default is invented for RendererUseRenderLists.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_field_order as F
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP
def run(image,pc,flag,typed):
 F.T.P=image;source=('[Vars]\n{\nflag RendererUseRenderLists = '+str(flag).lower()+'\n'+('point3 'if typed else'')+'StencilShadowColorMultiplier = 0.7,0.7,0.6\n}\n').encode();m=F.Machine(source,pc);pins={};seams=set()
 def executed(u,at,size,_):
  if F.ORIGINAL.base<=at<F.ORIGINAL.base+F.ORIGINAL.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,executed);parsed=m.parse();assert parsed['error']is None;node=m.r(m.r(m.parsed_root+0x34));settings=m.alloc(0x80);m.invoke(0x4b2ce0,settings,[])
 var=m.alloc(0x2c)
 for off,value in[(0x10,m.string([])),(0x1c,m.string(list(map(ord,'StencilShadowColorMultiplier')))),(0x24,5),(0x28,0),(4,0x3f000000),(8,0x3f000000),(12,0x3f000000)]:m.w(var+off,value)
 m.invoke(0x4b3470,settings,[0,var]);m.invoke(0x4b3c10,settings,[node]);assert m.r(settings+12)==2
 rows=[]
 for i in range(2):
  p=m.r(m.r(settings+8)+i*4);rows.append(dict(name=m.text(m.r(p+0x1c)),kind=m.r(p+0x24),flags=m.r(p+0x28),word=m.r(p),point3=[m.r(p+o)for o in(4,8,12)]))
 assert rows[0]['name']=='StencilShadowColorMultiplier'and rows[0]['point3']==[0x3f333333,0x3f333333,0x3f19999a]
 assert rows[1]['name']=='RendererUseRenderLists'and rows[1]['kind']==1 and rows[1]['flags']==0 and rows[1]['word']&255==int(flag)
 return rows,pins,seams
def main():
 rows=[];pins={};seams=set()
 for flag,typed,pc in itertools.product((False,True),(False,True),F.PCS):
  a,pa,sa=run(F.ORIGINAL,pc,flag,typed);b,pb,sb=run(F.EXPANDED,pc,flag,typed);assert a==b and pa==pb;rows.append(dict(flag=flag,typed_point3=typed,precision=pc,output=a));pins.update(pa);seams|=sa|sb
 for va,b in pins.items():assert F.ORIGINAL.range(va,len(bytes.fromhex(b))).hex()==F.EXPANDED.range(va,len(bytes.fromhex(b))).hex()==b
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,executions=2*len(rows),instructions={hex(a):b for a,b in sorted(pins.items())},transport_entries=[hex(a)for a in sorted(seams)])
 path=HERE/'native-camera-render-source-policy.json'
 if'--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
