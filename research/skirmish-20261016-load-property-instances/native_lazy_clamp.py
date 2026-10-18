"""Lazy mixed-modifier prefixes and source-field guards, real502B30 instructions.
Already-attached plain instance fixture; native allocation and numeric actor
consequence/GTID lookup are named seams. Missing SVar is stopped at its exact
load instruction after aggregate stores. No caller or full battle claim.
"""
from pathlib import Path
import sys,json,hashlib,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
class Cut(Exception):pass
f=nm.f32

def inputs():
 add=lambda i,v:{'Numeric':dict(kind='Add',index=i,amount=f(v),repair=False)}
 mul=lambda i,v:{'Numeric':dict(kind='Multiply',index=i,amount=f(v),repair=False)}
 return [dict(name='add_then_missing_clamp',mods=[add(3,1),mul(19,-2)],source=0x10001),
 dict(name='resistance_missing_clamp',mods=[{'Resistance':dict(damage_type=7,amount=f(.2))}],source=0x10001),
 dict(name='negative_special_skips_clamp',mods=[mul(21,-10)],source=0x10001),
 dict(name='periodic_heal_skips_source',mods=[{'Table':{'PeriodicHeal':dict(amount=f(.1))}}],source=0xfee1dead),
 dict(name='live_periodic_source_after_callback',mods=[add(3,1),{'Table':{'PeriodicDamage':dict(damage_type=7,amount=f(.1))}}],source=0xfee1dead)]

def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 actor=m.alloc(0x200);p=m.alloc(0xe0);instance=m.alloc(0x18);definition=m.alloc(0x90);dtype=m.alloc(0x30);events=[]
 def heap(mm):mm.ret(mm.alloc(mm.arg(1)))
 m.seam(0x6f7810,heap)
 #Actual Properties ctor, with only its immutable AbilityType count supplied.
 data=m.alloc(0x900);domain=m.alloc(0x40);m.w(0xb36530,data);m.w(data+0x3a8,domain);m.w(domain+0x28,19)
 m.call(0x5812f0,p,[actor]);m.w(actor+0x78,p)
 m.w(instance,0xa29678);m.w(instance+12,definition);m.w(instance+20,c['source']);m.w(p+0x14,1);m.w(m.r(p+0x10),instance)
 modptrs=[]
 def consequence(mm):
  z=mm.ecx();v=mm.r(z+16);events.append(dict(NumericChanged=dict(kind='Add'if mm.r(z+8)==1 else'Multiply',index=v,amount=mm.r(z+12),repair=False,value=mm.r(p+0x20+4*v))))
  if c['name']=='live_periodic_source_after_callback':m.w(instance+20,0x10002)
  mm.ret(pop=8)
 for v in c['mods']:
  z=m.alloc(0x30);modptrs.append(z)
  if'Numeric'in v:
   v=v['Numeric'];vt=m.alloc(0x34);m.u.mem_write(vt,nm.original.range(0xa2d79c if v['kind']=='Add'else 0xa2d690,0x34));m.w(vt+0x30,m.stub(consequence));m.w(z,vt);m.w(z+8,1 if v['kind']=='Add'else 2);m.w(z+12,v['amount']);m.w(z+16,v['index'])
  elif'Resistance'in v:
   v=v['Resistance'];m.w(z,0xa2d708);m.w(z+8,5);m.w(z+12,v['amount']);m.w(z+16,dtype)
  elif'PeriodicHeal'in v['Table']:
   v=v['Table']['PeriodicHeal'];m.w(z+8,7);m.w(z+12,v['amount'])
  else:
   v=v['Table']['PeriodicDamage'];m.w(z+8,6);m.w(z+12,dtype);m.w(z+16,v['amount'])
 links=[m.alloc(8)for _ in modptrs]
 for i,z in enumerate(modptrs):m.w(links[i],z);m.w(links[i]+4,links[i+1]if i+1<len(links)else 0)
 m.w(definition+0x1c,links[0]);sourceobj=m.alloc(0x30);m.w(sourceobj+20,0x10002)
 def source(mm):
  got=mm.r(mm.ecx()+20);assert c['name']=='live_periodic_source_after_callback' and got==0x10002
  mm.ret(sourceobj)
 m.seam(0x5031e0,source)
 def no_clamp(mm):raise Cut('missing_clamp')
 m.observe(0x50236b,no_clamp);m.observe(0x50252f,no_clamp)
 boundary=None
 try:m.call(0x502b30,p+0x1c,[instance,p+0x10,actor])
 except Cut as e:boundary=str(e)
 factors=[]
 for i in range(m.r(p+0x1c+0x78)):
  q=m.r(m.r(p+0x1c+0x74)+4*i);assert m.r(q)==dtype;factors.append(dict(damage_type=7,factor=m.r(q+4),resistance=m.r(q+8)))
 heals=[]
 for i in range(m.r(p+0x1c+0xa8)):
  q=m.r(m.r(p+0x1c+0xa4)+4*i);heals.append(dict(modifier=dict(definition=1,index=modptrs.index(q)),amount=m.r(q+12)))
 damages=[]
 for i in range(m.r(p+0x1c+0x9c)):
  q=m.r(m.r(p+0x1c+0x98)+4*i);z=m.r(q);assert m.r(z+12)==dtype;damages.append(dict(modifier=dict(definition=1,index=modptrs.index(z)),damage_type=7,amount=m.r(z+16),source_id=m.r(q+4),allegiance_id=m.r(q+8)))
 return dict(scalars=[m.r(p+0x20+4*i)for i in range(25)],damage_factors=factors,periodic_heals=heals,periodic_damage=damages,source=m.r(instance+20),events=events,boundary=boundary),m.visited

def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in inputs():
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;rows.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
 for va,raw in pins.items():assert raw==nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))
 doc=dict(scope=__doc__,rows=rows,native_executions=2*len(rows),instructions=[dict(va=hex(va),bytes=raw.hex())for va,raw in sorted(pins.items())])
 path=HERE/'native-lazy-clamp.json'
 if'--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
