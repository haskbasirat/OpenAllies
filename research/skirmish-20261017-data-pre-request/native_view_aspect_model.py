"""454F70 whole function composes469FE0 then a SECOND5A8520 using the
CURRENT Model+38 CString address. Both children are explicit native services;
469FE0's actual body is separately proved by native_aspect_request.py.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ECX
class Required(Exception):pass

def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 aspect=m.alloc(0x6c);models=[m.alloc(0x40) for _ in range(3)];m.w(aspect+0x28,models[0]);trace=[]
 def resolve(mm):
  assert mm.u.reg_read(UC_X86_REG_ECX)==aspect
  trace.append(dict(call='resolve',model=models.index(mm.r(aspect+0x28))))
  if c['stop']=='resolve':raise Required('resolve')
  mm.w(aspect+0x28,models[c['after']]);mm.ret(0,0)
 def factory(mm):
  model=models.index(mm.arg(1)-0x38)
  trace.append(dict(call='factory38',model=model))
  if c['stop']=='factory':raise Required('factory')
  mm.ret(models[2],0)
 m.seam(0x469fe0,resolve);m.seam(0x5a8520,factory)
 status='ok';result=None
 try:result=models.index(m.call(0x454f70,0,[aspect]))
 except Required as e:status='required_'+str(e)
 return dict(status=status,result=result,model=models.index(m.r(aspect+0x28)),trace=trace),m.visited
def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for after,stop in itertools.product([0,1],[None,'resolve','factory']):
   c=dict(after=after,stop=stop);a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
   rows.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins)};path=HERE/'native-view-aspect-model.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
