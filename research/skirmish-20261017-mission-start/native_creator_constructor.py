"""Actual CreatorLoad constructor5141F0/513EB0/5137C0 and5144A0 assignment.
The actual parameter constructors, 510DC0, 50F4C0, native vector copies and
WorldRules constructor/default loops execute. Retained current UVar/SVar values,
Biome typed Find, generic current-variable lookup, CString manager, allocators
and logging are explicit transport seams. Source values are supplied inputs,
not a claim of complete profile/console/UI startup. No game-code patching.
"""
from pathlib import Path
import sys,json,itertools,hashlib,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-net-integration'))
from native_launch_assignment import Machine as Base,base_data,rich_launch
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import UC_X86_REG_FPCW,UC_X86_REG_EIP
class Machine(Base):
 def __init__(self,image,pc,inputs):
  self.intercepted=set();super().__init__(False,image);self.mode='channels';self.setup_data(base_data())
  self.u.reg_write(UC_X86_REG_FPCW,h.PRECISION[pc]);self.inputs=inputs
  gd=self.alloc(0x600);self.w(0xb36530,gd);self.w(gd+0x38,inputs['resource_count'])
  arr=self.alloc(16)
  for i in range(3):self.w(arr+4*i,self.def_ptr('WORLD_RULES',i));self.w(self.def_ptr('WORLD_RULES',i)+0x40,100+i)
  self.w(gd+0x118,arr);self.w(gd+0x11c,3)
  typearr=self.alloc(4);self.w(typearr,self.def_ptr('CONDITIONS',0));self.w(gd+0x4a0,typearr)
  uv=self.alloc(0x100);w=self.alloc(4);ht=self.alloc(4);self.w(w,inputs['width']&0xffffffff);self.w(ht,inputs['height']&0xffffffff)
  self.w(uv+0xa8,w);self.w(uv+0xac,ht);self.w(0xb39940,uv)
  self.biome=self.alloc(0x30);self.biome_string=self.alloc(4);self.cs(self.biome_string,'actual_current_biome');self.w(self.r(self.biome_string)-4,100)
  self.indvar=self.alloc(4);v=self.alloc(4);self.w(self.indvar,v);self.w(v,inputs['independent']&0xffffffff)
  self.seams[0x410cb0]=self.string_get;self.seams[0x48d500]=lambda:self.ret(4,self.biome)
  self.seams[0x405370]=lambda:self.ret(4,self.indvar)
 def hook(self,u,pc,size,user):
  super().hook(u,pc,size,user)
  if u.reg_read(UC_X86_REG_EIP)!=pc:self.intercepted.add(pc)
 def string_get(self):
  self.w(self.arg(0),self.r(self.biome_string));self.ret(8,self.arg(0))
 def current(self,p):
  return {'kind_load':self.r(p)==0xa3220c,'source50_null':self.r(p+0x50)==0,
   'width_bits':self.r(p+0x3c),'height_bits':self.r(p+0x40),'biome_same_typed_find':self.r(p+0x44)==self.biome,
   'weather48':self.r(p+0x48),'seed4c':self.r(p+0x4c),'rules':self.read_wc(p+0x20),
   'parameters':self.read_header(p+4),'text0c':[self.rs(self.r(p+0x14)+i*0x6c+0xc)for i in range(self.r(p+0x18))]}
 def run(self,index):
  p=self.alloc(0x54);self.call(0x5141f0,[0],this=p,callee_pop=4);constructed=self.current(p)
  source=rich_launch(random.Random(0x5144a0+index))['game'];game=self.build_game(source)
  #Source key-only/header loader constructs each record before reading it;
  #the wire projection omits+0C and+68. Supply actual fresh ctor companions.
  for i in range(self.r(self.r(game+8)+0x14)):
   self.cs(self.r(self.r(game+8)+0x10)+i*0x6c+0xc,'')
   self.b(self.r(self.r(game+8)+0x10)+i*0x6c+0x68,False)
  before=self.current(p);self.call(0x5144a0,[game],this=p,callee_pop=4);after=self.current(p)
  assert after['parameters']==source['header'];assert after['rules']==source['conditions']
  for key in ('kind_load','source50_null','width_bits','height_bits','biome_same_typed_find','weather48','seed4c'):
   assert before[key]==after[key]
  assert self.r(p+8)!=self.r(self.r(game+8)+4)
  assert self.r(p+0x14)!=self.r(self.r(game+8)+0x10)
  assert self.r(p+0x2c)!=self.r(self.r(game+4)+0xc)
  return {'constructor':constructed,'assignment_source':source,'assigned':after,'outer_arrays_independent':True}
def main():
 inputs=[{'width':w,'height':ht,'independent':n,'resource_count':r}
  for w,ht,n,r in [(192,192,0,3),(16777217,-16777217,1,0),(-2147483648,2147483647,-1,3),(1,256,2,3)]]
 rows=[];visited={};seams=set()
 for image,pc,index in itertools.product(('original','v14'),h.PRECISION,range(len(inputs))):
  m=Machine(image,pc,inputs[index]);output=m.run(index);seams|=m.intercepted
  for a,n in m.visited.items():visited[a]=n
  rows.append({'image':image,'precision':pc,'input':inputs[index],'output':output})
 pins=[]
 for a,n in sorted(visited.items()):
  if a in seams or not(0x401000<=a<0x9ef000):continue
  b=h.P.range(a,n);assert b==h.V.range(a,n)
  pins.append({'va':hex(a),'bytes':b.hex()})
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'seams_not_executed':[hex(a)for a in sorted(seams)],'executed_instructions':pins,'cases':rows}
 path=HERE/'native-creator-constructor.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'cases':len(rows),'executed_pins':len(pins),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
