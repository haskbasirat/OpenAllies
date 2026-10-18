"""Actual469FE0 Model resolution and46AEC0 Aspect request order/partial fields.
Current GMap/Biome, Aspect and Resource fields are bounded explicit inputs.
Model5A8520, Resource vf10, AnimationSet49B7E0, extra textures46AE20 and
debug assertion are explicit child services, not completed source/GPU loading.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ECX
class Required(Exception):pass

def cases():
 for sensitive,model,same,skin,loaded,extra in itertools.product([False,True],repeat=6):
  yield dict(sensitive=sensitive,model_present=model,same_context=same,skin=skin,glow=False,animation=False,loaded=loaded,extra=extra,
   unique=False,factory_same=False,count=7,delta=3,mode=4,stop=None,mutate_count=False,model_switch=False)
 base=dict(sensitive=True,model_present=False,same_context=False,skin=True,glow=True,animation=True,loaded=True,extra=False,
  unique=False,factory_same=False,count=0xffffffff,delta=2,mode=0xffffffff,stop=None,mutate_count=False,model_switch=False)
 for key in ['factory','skin','glow','animation','model','extra']:
  yield dict(base,stop=key)
 for change in [dict(mutate_count=True),dict(model_switch=True),dict(unique=True),dict(factory_same=True,model_present=True),dict(sensitive=False,model_present=True)]:
  yield dict(base,**change)

def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 aspect=m.alloc(0x6c);world=m.alloc(0x34);gmap=m.alloc(0x28);biomes=[m.alloc(4),m.alloc(4)]
 m.w(0xb36534,world);m.w(world+0x30,gmap);m.w(gmap+0x24,biomes[1]);m.w(aspect+0x24,biomes[1] if c['same_context'] else biomes[0])
 m.wb(aspect+0x1c,c['sensitive']);m.wb(aspect+0x54,c['extra']);m.wb(aspect+0x68,c['unique']);m.w(aspect+0x18,c['count']);m.w(aspect+0x4c,7)
 trace=[];models=[]
 def call(label,mm):
  args=[mm.arg(1)] if label=='animation' else [mm.arg(1),mm.arg(2)]
  trace.append(dict(call=label,args=args,count=mm.r(aspect+0x18)))
  if c['stop']==label:raise Required(label)
  if label=='model' and c['model_switch']:mm.w(aspect+0x28,models[2])
  if label=='extra':
   mm.wb(aspect+0x54,1)
   if c['mutate_count']:mm.w(aspect+0x18,123)
  mm.ret(0,4 if label=='animation' else 8)
 def resource(label,flags=0):
  p=m.alloc(12);vt=m.alloc(0x14);m.w(p,vt);m.w(p+8,flags);m.w(vt+0x10,m.stub(lambda mm:call(label,mm)));return p
 models=[resource('model',int(c['loaded'])),resource('model',int(c['loaded'])),resource('model',int(not c['loaded']))]
 m.w(aspect+0x28,models[0] if c['model_present'] else 0)
 m.w(aspect+0x30,resource('skin') if c['skin'] else 0);m.w(aspect+0x34,resource('glow') if c['glow'] else 0)
 m.w(aspect+0x38,m.alloc(4) if c['animation'] else 0)
 m.seam(0x49b7e0,lambda mm:call('animation',mm));m.seam(0x46ae20,lambda mm:call('extra',mm))
 def factory(mm):
  trace.append(dict(call='factory',args=[],count=mm.r(aspect+0x18)))
  if c['stop']=='factory':raise Required('factory')
  mm.ret(models[0] if c['factory_same'] else models[1],0) #cdecl; caller adds4.
 m.seam(0x5a8520,factory)
 def assertion(mm):
  trace.append(dict(call='assert',args=[],count=mm.r(aspect+0x18)))
  mm.ret(1,0) #cdecl debug assertion child; explicit continue result.
 m.seam(0x855d30,assertion)
 status='ok'
 try:m.call(0x46aec0,aspect,[c['delta'],c['mode']])
 except Required as e:status='required_'+str(e)
 model=m.r(aspect+0x28)
 return dict(status=status,count=m.r(aspect+0x18),context=biomes.index(m.r(aspect+0x24)),model=None if not model else models.index(model),
  flags=[m.r(p+8) for p in models],extra_count=m.r(aspect+0x4c),extra=bool(m.rb(aspect+0x54)),trace=trace),m.visited

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
 path=HERE/'native-aspect-request.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
