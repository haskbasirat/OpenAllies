"""Native fresh SVars field hash insertion/growth/traversal and Settings order.
Whole5BD080 parser with prior line/EOF/heap/string/mutex host transports;
actual fields, constructors,496F60/491F80 and4B3C10 execute. Reference model
uses bucket order, not lexical/authored sorting. No inherited field merge,
failed allocation/throw, arbitrary expressions, or whole startup claim.
"""
from pathlib import Path
import sys,json,hashlib,re
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/phase8-restoration'));import native_tgi_parser as parser
import trace_registry as T
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_FPCW
ORIGINAL=T.P;EXPANDED=T.V
PCS={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
def sha(b):return hashlib.sha256(b).hexdigest()
def field_hash(name):
 h=0
 for c in name.encode('utf-16-le')[::2]:
  #All current field names in this bounded producer are ASCII.
  if 97<=c<=122:c-=32
  h=(h*33+c)&0xffffffff
 return h

def expected_order(names):
 buckets=[[]for _ in range(5)];count=0
 for index,name in enumerate(names):
  assert name.isascii() and name.lower()not in [n.lower()for n in names[:index]]
  if count>=len(buckets)*2:
   size=len(buckets)*2+1
   while size%3==0 or size%5==0:size+=2
   new=[[]for _ in range(size)]
   for chain in buckets:
    for prior in chain:new[field_hash(names[prior])%size].insert(0,prior)
   buckets=new
  buckets[field_hash(name)%len(buckets)].insert(0,index);count+=1
 return [i for chain in buckets for i in chain],len(buckets)

class Machine(parser.Machine):
 def __init__(self,source,pc):
  super().__init__(parser.lines_of(source));self.u.mem_map(0x5000000,0x1000000);self.large_heap=0x5000000
  self.u.reg_write(UC_X86_REG_FPCW,PCS[pc]);self.parsed_root=None
 def alloc(self,size):
  if not hasattr(self,'large_heap'):return super().alloc(size)
  p=self.large_heap;self.large_heap+=(size+15)&~15;assert self.large_heap<0x6000000;return p
 def node(self,p,depth=0):
  if depth==0:self.parsed_root=p
  return super().node(p,depth)
 def hook(self,u,pc,size,z):
  if pc in(0x404f60,0x404ed0,0x404ea0):self.ret()
  elif pc==0x5b3b10:
   out=self.arg();self.w(out,self.string(list(map(ord,'field-order-source'))));self.ret(out,pop=4)
  else:super().hook(u,pc,size,z)

def run(image,pc,source,names):
 T.P=image;m=Machine(source,pc);pins={};seams=set()
 def executed(u,at,size,_):
  if ORIGINAL.base<=at<ORIGINAL.base+ORIGINAL.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,executed)
 result=m.parse();assert result['error']is None,result
 root=m.parsed_root;assert m.r(root+0x38)==1
 node=m.r(m.r(root+0x34));buckets=m.r(node+0x40);capacity=m.r(node+0x44);count=m.r(node+0x48)
 assert count==len(names),(count,len(names))
 fields=[]
 for bucket in range(capacity):
  link=m.r(buckets+bucket*4)if buckets else 0
  while link:
   f=m.r(link);name=m.text(m.r(f+4));kind=m.invoke(m.r(m.r(f)+0x48),f,[])
   fields.append({'name':name,'source_kind':kind,'vtable':m.r(f),'line':m.r(f+0xc),'value_word':m.r(f+0x10)})
   link=m.r(link+4)
 order,size=expected_order(names);assert capacity==size
 assert [f['name']for f in fields]==[names[i]for i in order]
 settings=m.alloc(0x80);m.invoke(0x4b2ce0,settings,[]);m.invoke(0x4b3c10,settings,[node])
 array=m.r(settings+8);added=[]
 for index in range(m.r(settings+12)):
  var=m.r(array+index*4);added.append({'name':m.text(m.r(var+0x1c)),'kind':m.r(var+36),'flags':m.r(var+40)})
 assert [a['name']for a in added]==[f['name']for f in fields if f['source_kind']!=9],added
 return {'capacity':capacity,'fields':fields,'settings_added':added},pins,seams

def cases():
 for size in [0,1,2,5,9,10,11,22,23,46,47,94,95,194,195,231]:
  names=[f'Field{i:03d}'for i in range(size)]
  #Reverse/stride authored order deliberately differs from lexical order.
  names=names[::2]+names[1::2][::-1]
  yield f'bool_count{size}',('[SVars]\n{\n'+''.join(f'bool {n} = true\n'for n in names)+'}\n').encode(),names
 forms=[('int','int IntField = -7'),('fixed','fixed FixedField = 0.25'),('float','float FloatField = 0.25'),
  ('bool','bool BoolField = true'),('flag','flag FlagField = false'),('string','string StringField = token'),
  ('point3','point3 PointField = 1,2,-3'),('inferred_fixed','DenizenResupplyRateUnderSiege = 0.25'),
  ('inferred_true','SupplyProvideUnderSiege = true'),('inferred_false','DisableSAI = false')]
 names=[re.search(r'(?:^| )([A-Za-z][A-Za-z0-9_]*) =',line).group(1)for _,line in forms]
 yield 'actual_kind_forms',('[SVars]\n{\n'+'\n'.join(line for _,line in forms)+'\n}\n').encode(),names

def main():
 rows=[];pins={};seams=set()
 for name,source,names in cases():
  for pc in PCS:
   a,pa,sa=run(ORIGINAL,pc,source,names);b,pb,sb=run(EXPANDED,pc,source,names)
   assert a==b;assert pa==pb;rows.append({'case':name,'precision':pc,'source_hex':source.hex(),'authored_names':names,'native':a});pins.update(pa);seams|=sa|sb
 for va,raw in pins.items():assert ORIGINAL.range(va,len(bytes.fromhex(raw)))==EXPANDED.range(va,len(bytes.fromhex(raw)))==bytes.fromhex(raw)
 out={'schema':1,'scope':__doc__,'rows':rows,'paired_rows':len(rows),'native_executions':2*len(rows),
  'executed_instruction_pins':{f'{va:08x}':raw for va,raw in sorted(pins.items())},'successful_host_transport_entries':[f'{a:08x}'for a in sorted(seams)],
  'producer_sha256':sha(Path(__file__).read_bytes())}
 target=HERE/'native-field-order.json'
 if '--write'in sys.argv:
  with target.open('x')as f:json.dump(out,f,indent=1);f.write('\n')
 else:assert json.loads(target.read_bytes())==out
 print(json.dumps({'rows':len(rows),'executions':len(rows)*2,'pins':len(pins),'sha256':sha(target.read_bytes())}))
if __name__=='__main__':main()
