"""Full native49B9F0/45DD80 dispatch, actual4BB040/49B970 stores.
Current Data/Aspect/Model/Animation/Ability fields are bounded explicit inputs;
child functions are required services. Mutating callbacks prove rereads and
captured counts. This is not whole source loading or backend completion.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ECX
class Required(Exception):pass
def cases():
 base=dict(actor=True,count=2,flags=0x4000,unique=True,portrait=True,textures=True,banner=True,animations=True,abilities=True,extra=True,settlement=True,mask=0x80004,initial=7,delta=3,mode=0xffffffff,stop=None,mutation=None)
 for actor,count,flags,unique in itertools.product([False,True],[-1,0,1,2],[0,0x4000],[False,True]):yield dict(base,actor=actor,count=count,flags=flags,unique=unique)
 for k in ['portrait','textures','banner','animations','abilities','extra','settlement']:
  yield dict(base,**{k:False})
 for mutation in ['aspect_count','aspect_model','aspect_fields','animation_inner','animation_outer','ability_count','mask_clear','mask_set']:
  yield dict(base,mutation=mutation,mask=0 if mutation=='mask_set' else base['mask'])
 for stop in ['portrait','aspect0','texture236','banner','viewfx3','animation0','actorfx404','ability0','ability2','extra','settlement']:
  yield dict(base,stop=stop)
 yield dict(base,initial=0xffffffff,delta=2,mode=0)

def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 data=m.alloc(0x5b0);m.w(data+0x20,c['initial']);m.w(data+0x1c,c['flags']);m.wb(data+0xfc,c['unique']);m.w(data+0x174,c['mask'])
 models=[m.alloc(12)for _ in range(3)]
 for i,p in enumerate(models):m.w(p+8,0x400+i)
 aspects=[m.alloc(0x6c)for _ in range(2)];array=m.alloc(8)
 for i,p in enumerate(aspects):m.w(p+0x28,models[i]);m.w(array+4*i,p)
 m.w(data+0x10c,array);m.w(data+0x110,c['count']&0xffffffff)
 trace=[];sets=[]
 def child(mm,label,nargs=2):
  trace.append(dict(call=label,args=[mm.arg(i+1)for i in range(nargs)],count=mm.r(data+0x20)))
  if c['stop']==label:raise Required(label)
  change=c['mutation']
  if label=='aspect0':
   if change=='aspect_count':mm.w(data+0x110,1)
   if change=='aspect_model':mm.w(aspects[0]+0x28,models[2])
   if change=='aspect_fields':mm.w(data+0x1c,0);mm.wb(data+0xfc,0)
  if label=='animation0':
   if change=='animation_inner':mm.w(sets[1]+4,1)
   if change=='animation_outer':mm.w(data+0x1c0,2)
  if label=='ability0' and change=='ability_count':mm.w(data+0x1f0,0);mm.w(data+0x220,1)
  if label=='actorfx404':
   if change=='mask_clear':mm.w(data+0x174,0)
   if change=='mask_set':mm.w(data+0x174,0x80000)
  mm.ret(0,nargs*4)
 def virtual(label,slot=0x10,nargs=2):
  p=m.alloc(4);vt=m.alloc(0x18);m.w(p,vt);m.w(vt+slot,m.stub(lambda mm:child(mm,label,nargs)));return p
 m.w(data+0x104,virtual('portrait',0x14)if c['portrait']else 0)
 m.w(data+0x534,virtual('settlement',0x14)if c['settlement']else 0)
 for off in [0xec,0xf0,0x138]:m.w(data+off,virtual('texture'+str(off))if c['textures']else 0)
 m.w(data+0x134,m.alloc(4)if c['banner']else 0)
 m.seam(0x46c7a0,lambda mm:child(mm,'banner'))
 def aspect(mm):child(mm,'aspect'+str(aspects.index(mm.u.reg_read(UC_X86_REG_ECX))))
 m.seam(0x46aec0,aspect)
 def fx(mm):
  off=mm.u.reg_read(UC_X86_REG_ECX)-data
  child(mm,('viewfx'+str((off-0xbc)//8))if 0xbc<=off<0xec else'actorfx'+str(off))
 m.seam(0x4288b0,fx)
 outer=m.alloc(12);sets=[0,m.alloc(8),m.alloc(8)]
 for i,p in enumerate(sets):m.w(outer+4*i,p)
 for i,p in enumerate(sets[1:]):
  arr=m.alloc(8);m.w(p,arr);m.w(p+4,2 if i==0 else 1)
  for j in range(2):m.w(arr+4*j,virtual('animation'+str(i*2+j),0,1))
 m.w(data+0x1bc,outer);m.w(data+0x1c0,3 if c['animations']else 0)
 abilities=[m.alloc(4)for _ in range(4)];a=m.alloc(8);b=m.alloc(8)
 for i in range(2):m.w(a+4*i,abilities[i]);m.w(b+4*i,abilities[i+2])
 m.w(data+0x1ec,a);m.w(data+0x1f0,2 if c['abilities']else 0);m.w(data+0x21c,b);m.w(data+0x220,2 if c['abilities']else 0)
 extra=m.alloc(4);m.w(data+0x2a4,extra if c['extra']else 0)
 def ability(mm):
  p=mm.u.reg_read(UC_X86_REG_ECX);child(mm,'extra'if p==extra else'ability'+str(abilities.index(p)))
 m.seam(0x4be420,ability)
 status='ok'
 try:m.call(0x45dd80 if c['actor']else 0x49b9f0,data,[c['delta'],c['mode']])
 except Required as e:status='required_'+str(e)
 return dict(status=status,count=m.r(data+0x20),flags=[m.r(p+8)for p in models],trace=trace),m.visited

def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases():
   pair=[]
   for image in ['original','v14']:
    output,visited=run(c,image,pc);pair.append(output);pins.update(visited)
   assert pair[0]==pair[1];rows.append(dict(precision=pc,input=c,output=pair[0]))
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),'harness_sha256':hashlib.sha256((ROOT/'research/phase8-ai/native_machine.py').read_bytes()).hexdigest()}
 path=HERE/'native-object-actor-request.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
