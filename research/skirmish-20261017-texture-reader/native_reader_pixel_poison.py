"""Separate native PNG allocation-poison diagnostic. Does not change either
frozen reader producer or use poisoned output as a universal valid PNG target.
"""
from pathlib import Path
import sys,json,itertools
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));import native_reader_pixels as pixels
n=pixels.n

def run(c,image,pc,poison):
 original=n.Machine
 class Poison(original):
  def stub(self,fn):
   if getattr(fn,'__name__','')=='heap_alloc':
    def wrapped(mm):
     from unicorn.x86_const import UC_X86_REG_EAX
     size=mm.arg(3);fn(mm);p=mm.u.reg_read(UC_X86_REG_EAX)
     mm.u.mem_write(p,bytes([poison])*size)
    return super().stub(wrapped)
   return super().stub(fn)
 n.Machine=Poison
 try:return pixels.run(c,image,pc)
 finally:n.Machine=original

def main():
 rows=[];pins={}
 cases=[c for c in pixels.cases()if c['extension']=='png']
 for c,pc,poison in itertools.product(cases,n.PRECISIONS,[0,0xa5,0xff]):
  a,pa=run(c,'original',pc,poison);b,pb=run(c,'v14',pc,poison);assert a==b
  pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,poison=poison,input=c,output=a))
 for va,raw in pins.items():assert n.original.range(va,len(raw))==n.modified.range(va,len(raw))==raw
 summary=[]
 for c in cases:
  outputs={str(r['poison']):r['output']['normalized_rgba']for r in rows if r['input']['label']==c['label']and r['precision']=='Double53'}
  summary.append(dict(label=c['label'],rgba_by_poison=outputs,all_equal=len(set(outputs.values()))==1))
 doc=dict(schema=1,scope=__doc__,pixels_producer_sha256=n.sha(Path(pixels.__file__).read_bytes()),
  original_sha256=n.sha(n.original.b),v14_sha256=n.sha(n.modified.b),rows=rows,summary=summary,
  native_executions=2*len(rows),instructions=n.instructions(pins))
 p=HERE/'native-reader-pixel-poison.json'
 if '--write'in sys.argv:
  with p.open('x')as f:json.dump(doc,f,indent=2);f.write('\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),native_executions=2*len(rows),pins=len(pins),summary=summary,sha256=n.sha(p.read_bytes())),indent=2))
if __name__=='__main__':main()
