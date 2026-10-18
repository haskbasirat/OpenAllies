"""Bounded494F30..494FA0 source classifier: permanent iff VFXData+24==1.
Base loader, typed getters and model-kind virtual are explicit input services;
original x87 comparison and ordered duration/class stores execute unchanged.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
class Cutoff(Exception):pass

def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 data=m.alloc(0x80);vt=m.alloc(0x80);source=m.alloc(4);m.w(data,vt);m.w(data+0x24,0xcdcdcdcd);m.w(data+0x28,0xcdcdcdcd);events=[]
 m.w(vt+0x6c,m.stub(lambda mm:(events.append('model_kind'),mm.ret(c['model']))))
 m.seam(0x4bb310,lambda mm:(events.append('base'),mm.ret(pop=4)))
 m.seam(0x5b5380,lambda mm:(events.append('permanent'),mm.ret(c['permanent'],4)))
 m.seam(0x5b4ae0,lambda mm:(events.append('duration'),mm.ret_st0_f32(c['duration'],4)))
 m.observe(0x494fa0,lambda mm:(_ for _ in ()).throw(Cutoff()))
 try:m.call(0x494f30,data,[source]);raise AssertionError('no cutoff')
 except Cutoff:pass
 assert not m.null_accesses
 from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_FPSW
 assert m.u.reg_read(UC_X86_REG_EIP)==0x494fa0
 assert ((m.u.reg_read(UC_X86_REG_FPSW)>>11)&7)==0
 return dict(class_word=m.r(data+0x24),duration=m.r(data+0x28),events=events),m.visited

def main():
 rows=[];pins={}
 for pc,permanent,duration,model in itertools.product(nm.PRECISIONS,[0,1],[0,0x80000000,1,0x80000001,0x3f800000,0xbf800000,0x7f7fffff,0xff7fffff],[0,1,2]):
  c=dict(permanent=permanent,duration=duration,model=model);a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
  assert (a['class_word']==1)==bool(permanent);pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=2*len(rows),instructions=[dict(va=hex(v),bytes=b.hex())for v,b in sorted(pins.items())])
 p=HERE/'native-vfx-active-source.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
