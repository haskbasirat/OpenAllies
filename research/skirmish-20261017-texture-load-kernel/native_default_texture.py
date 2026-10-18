"""Actual default5A4D80 ->875AC0 PixelData ->87BF00/87BDD0 SourceTexture.
Executes real PixelFormat static initializer9E75F0/8696E0, Pixel allocation
layout8C5A40, color writes, Source list87B930 and Pixel retain. Allocation and
OS thread/mutex transports are seams. Positive renderer vf4C and destruction
are explicit children: their successful transport is NOT real GPU proof.
"""
from pathlib import Path
import sys,json,itertools,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as n
class Required(Exception):pass
def run(c,image,pc):
 m=n.Machine(pc)
 if image=='v14':
  old=(n.original.image_size+0xfff)&~0xfff;new=(n.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(n.original.base+old,new-old)
  for vs,rva,raw,off in n.modified.sections:m.u.mem_write(n.modified.base+rva,n.modified.b[off:off+raw])
 allocated=[];events=[];sources=[];pixels=[]
 def allocate(mm):
  size=mm.arg(1);p=mm.alloc(size);mm.u.mem_write(p,b'\xcd'*size);allocated.append([p,size]);mm.ret(p)
 m.seam(0x6f7810,allocate)
 def thread(mm):mm.ret(0x1729)
 def mutex(mm):mm.ret(1,4)
 m.w(0x9f01e4,m.stub(thread));m.w(0x9f01f0,m.stub(mutex));m.w(0x9f01f4,m.stub(mutex))
 #Both globals are explicit fixture inputs, not inferred runtime policy.
 m.w(0xb3d8e0,c['color_bits'])
 for i,a in enumerate([0xb3d8dc,0xb3d8d8,0xb3d8d4,0xb3d8d0]):m.w(a,0x11223340+i)
 m.wb(0xb323b0,c['renderer_mode'])
 render_target=struct.unpack('<I',n.original.range(0xab8fe8+0x4c,4))[0]
 destroy_target=struct.unpack('<I',n.original.range(0xab8fe8,4))[0]
 def render(mm):
  events.append(['renderer',len(sources)])
  if c['renderer_result']=='required':raise Required('source-renderer-4c')
  mm.ret(int(c['renderer_result']))
 def destroy(mm):events.append(['destroy',len(sources),mm.arg(1)]);mm.ret(0,4)
 m.seam(render_target,render);m.seam(destroy_target,destroy)
 def pixel(mm):pixels.append(mm.ecx())
 def source(mm):sources.append(mm.ecx())
 m.observe(0x875ac0,pixel);m.observe(0x87bdd0,source)
 m.call(0x9e75f0)
 fmt=[m.r(0xb43d2c+i*4)for i in range(9)];prefs=[m.r(0xb323b4+i*4)for i in range(3)]
 returns=[];stopped=None
 for call in range(c['calls']):
  try:returns.append(m.call(0x5a4d80))
  except Required as e:stopped=str(e);break
 def pixel_fields(p):
  count=m.r(p+0x44);off=m.r(p+0x40);data=m.r(p+0x34);size=m.r(off+count*4)
  return dict(refs=m.r(p+4),format=[m.r(p+0xc+i*4)for i in range(9)],widths=[m.r(m.r(p+0x38)+i*4)for i in range(count)],
   heights=[m.r(m.r(p+0x3c)+i*4)for i in range(count)],offsets=[m.r(off+i*4)for i in range(count+1)],
   stride=m.r(p+0x48),levels=count,word4c=m.r(p+0x4c),byte_count=size,rgba=list(m.u.mem_read(data,size)))
 def source_fields(p):
  pixel=m.r(p+0x40);return dict(refs=m.r(p+4),preferences=[m.r(p+0x20+i*4)for i in range(3)],renderer_present=m.r(p+0x2c)!=0,
   prev=0 if m.r(p+0x30)==0 else sources.index(m.r(p+0x30))+1,next=0 if m.r(p+0x34)==0 else sources.index(m.r(p+0x34))+1,
   pixel=0 if not pixel else pixels.index(pixel)+1,filename=m.r(p+0x38),word3c=m.r(p+0x3c),discard=m.rb(p+0x44),quality=m.r(p+0x48))
 output=dict(format=fmt,preferences=prefs,color_bits=m.r(0xb3d8e0),colors=[m.r(a)for a in [0xb3d8dc,0xb3d8d8,0xb3d8d4,0xb3d8d0]],
  allocation_sizes=[size for _,size in allocated],events=events,returns=[0 if not p else sources.index(p)+1 for p in returns],stopped=stopped,
  pixels=[pixel_fields(p)for p in pixels],sources=[source_fields(p)for p in sources],global_object_count=m.r(0xb45f14),
  list_head=0 if not m.r(0xb44438)else sources.index(m.r(0xb44438))+1,list_tail=0 if not m.r(0xb4443c)else sources.index(m.r(0xb4443c))+1)
 return output,m.visited
def main():
 rows=[];pins={}
 cases=[dict(color_bits=bits,renderer_mode=0,renderer_result=True,calls=2)for bits in [0,1,2,4,8,15]]
 cases +=[dict(color_bits=0,renderer_mode=mode,renderer_result=result,calls=1)for mode,result in itertools.product([1,255],[True,False,'required'])]
 for pc,c in itertools.product(n.PRECISIONS,cases):
  a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b,(pc,c,a,b);pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert n.original.range(va,len(raw))==n.modified.range(va,len(raw))==raw
 #Pin actual current PE defaults and the CRT table's registration of the real
 #PixelFormat initializer. Executing initializer directly does not establish
 #every later dynamic write; application retains its own actual current owner.
 raw_refs=[];needle=struct.pack('<I',0x9e75f0);start=0
 while True:
  at=n.original.b.find(needle,start)
  if at<0:break
  start=at+1
  for vs,rva,raw,off in n.original.sections:
   if off<=at<off+raw:
    va=n.original.base+rva+at-off;assert n.modified.range(va,4)==needle;raw_refs.append(hex(va))
 doc=dict(schema=1,scope=__doc__,original_sha256=n.sha(n.original.b),v14_sha256=n.sha(n.modified.b),rows=rows,native_executions=len(rows)*2,
  default_preferences_raw=n.original.range(0xb323b4,12).hex(),pixel_format_initializer_references=raw_refs,instructions=n.instructions(pins))
 path=HERE/'native-default-texture.json'
 if '--write'in sys.argv:
  with path.open('x')as f:json.dump(doc,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=n.sha(path.read_bytes()))))
if __name__=='__main__':main()
