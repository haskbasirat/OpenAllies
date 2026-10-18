"""49DC40 BaseTerrain null-source table plus actual4EA260 consumer.
Proper49DC40 SEH/register prologue executes; earlier unrelated ObjectData
loader is an explicit transport to49EE1A, and49F06C is an observed cutoff.
Native allocations/array constructors/field stores and empty49D750 execute.
The actual HQ source uses eight biomes, null normal/connecting defaults,
zero BiomeBaseTerrain children. No positive variation/paint claim.
"""
from pathlib import Path
import sys,json,hashlib
P=Path(__file__).resolve().parent;R=P.parents[1]
sys.path.insert(0,str(R/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_EIP
class Cut(Exception):pass
def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for _,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 data=m.alloc(0x700);source=m.alloc(32);base=m.alloc(32);game=m.alloc(0x900)
 m.w(0xb36530,game);m.w(game+0x5e8,c['biomes']);m.w(data+0x98,1);m.w(data+0xa4,1)
 strings={}
 def text(p):
  v=[]
  while m.r(p)&65535:v.append(m.r(p)&65535);p+=2
  return ''.join(map(chr,v))
 def string(v):
  if v not in strings:
   p=m.alloc(16+len(v)*2+2);m.w(p+4,len(v));m.u.mem_write(p+16,(v+'\0').encode('utf-16-le'));strings[v]=p+16
  return strings[v]
 def before(mm):mm.ret(data,4);mm.u.reg_write(UC_X86_REG_EIP,0x49ee1a)
 m.seam(0x4bb310,before)
 m.seam(0x5b4410,lambda mm:mm.ret(base if c['present']else 0,8))
 m.seam(0x5b5130,lambda mm:mm.ret_st0_f32(c['padding'],4))
 m.seam(0x5b54d0,lambda mm:mm.ret(m.array([string('')]),4))
 m.seam(0x5b4220,lambda mm:mm.ret(0,4))
 m.seam(0x6f7810,lambda mm:mm.ret(m.alloc(mm.arg(1))));m.seam(0x6f7430,lambda mm:mm.ret())
 m.seam(0x410c40,lambda mm:(_ for _ in ()).throw(Cut('required diagnostic')))
 m.seam(0x49f06c,lambda mm:(_ for _ in ()).throw(Cut('after BaseTerrain')))
 error=None
 try:m.call(0x49dc40,data,[source])
 except Cut as e:error=str(e)
 fields={hex(o):m.r(data+o)for o in [0x8c,0x94,0x98,0xa0,0xa4]}
 normal=[m.r(m.r(data+0x90)+4*i)for i in range(m.r(data+0x94))]
 connecting=[m.r(m.r(data+0x9c)+4*i)for i in range(m.r(data+0xa0))]
 consumer=[]
 if error=='after BaseTerrain' and c['biomes']>0:
  world=m.alloc(0x200);gmap=m.alloc(0x100);biome=m.alloc(0x100);obj=m.alloc(0x180)
  m.w(0xb36534,world);m.w(world+0x30,gmap);m.w(gmap+0x24,biome);m.w(obj,0xa273f0);m.w(obj+4,data)
  for ordinal in range(c['biomes']):
   m.w(biome+0xc,ordinal);m.call(0x4ea260,obj);consumer.append(m.u.reg_read(nm.UC_X86_REG_EAX))
 m.verify_v14();assert not m.null_accesses
 return dict(fields=fields,normal=normal,connecting=connecting,error=error,consumer=consumer),m.visited
def main():
 rows=[];pins={}
 for present,count,padding in [(False,0,0),(False,8,0),(True,0,0),(True,1,0),(True,8,0x3f800000),(True,8,0xbf800000),(True,8,0x3e99999a)]:
  c=dict(present=present,biomes=count,padding=padding)
  for pc in nm.PRECISIONS:
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;rows.append(dict(input=c,pc=pc,native=a));pins.update(pa);pins.update(pb)
 out={**nm.header(__file__,__doc__),'rows':rows,'native_executions':len(rows)*2,'executed_pins':nm.instructions(pins)}
 path=P/'native-base-terrain-null.json';encoded=json.dumps(out,indent=2)+'\n'
 if '--write'in sys.argv:
  with path.open('x')as f:f.write(encoded)
 else:assert json.loads(path.read_text())==out
 print(dict(rows=len(rows),runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
if __name__=='__main__':main()
