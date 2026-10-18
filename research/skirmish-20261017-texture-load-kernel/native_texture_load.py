"""Actual5A8CD0/5A8B40/5A6A10/43A920/87BAA0/87BAC0/87BE40.
Borrowed fixture Resource/SourceTexture/Pixel fields; current named policies,
create/default/stream decode/resize/renderer dimensions/diagnostics/CString and
destruction transports are EXPLICIT children, not full decoder/GPU/Task proof.
All3PC/original-v14 execute separately. Required stops precede omitted bodies.
"""
from pathlib import Path
import sys,json,itertools,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as n
from unicorn import UC_HOOK_MEM_WRITE,UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EDI
class Required(Exception):pass
def cases():
 base=dict(flags=0,locations=0,writable=None,child=False,filename=True,no_ni=0,old=0,default=1,created=1,
  warning=False,assert_value=True,low=0,high=1,width=64,height=64,renderer=None,pixels=True,png=False,
  resize=2,format=[0x22,0],required_format=[0x22,0],render_formats=[[0x22,0]],render_count=None,
  pixel_type=4,bpp=32,bytes=16384,memory=17,source_refs=5,pixel_refs=5,fail=None)
 out=[]
 def add(name,**kw):out.append(dict(base,name=name,**kw))
 add('ordinary')
 for writable,child,filename in itertools.product([None,False,True],[False,True],[False,True]):add(f'location_{writable}_{child}_{filename}',locations=1,writable=writable,child=child,filename=filename)
 for flags,no_ni,old in itertools.product([0,0x10,0x40,1<<20],[0,1,2],[0,1,3]):add(f'path_{flags}_{no_ni}_{old}',flags=flags,no_ni=no_ni,old=old)
 for low,high in itertools.product([0,1,2],[0,1,2]):add(f'quality_{low}_{high}',low=low,high=high)
 for low,high in [(0,1),(1,1),(0,0)]:add(f'null_created_{low}_{high}',created=0,low=low,high=high)
 add('null_default',flags=0x40,default=0)
 add('direct_warning_already_set',flags=0x10,warning=True,old=0)
 add('direct_assert_false',flags=0x10,warning=False,old=0,assert_value=False)
 for size,pixels in itertools.product([(0,0),(0,63),(63,0),(65,64),(65,67),(2048,2048),(1100,80)],[False,True]):add(f'dimensions_{size}_{pixels}',width=size[0],height=size[1],pixels=pixels)
 add('renderer_has_priority',width=65,height=67,renderer=[64,128])
 for resize,png in itertools.product([0,1,2],[False,True]):add(f'resize_{resize}_{png}',width=65,height=67,resize=resize,png=png)
 add('resize_format_first_mismatch',width=65,height=67,required_format=[7,999])
 add('resize_format_second_mismatch',width=65,height=67,required_format=[0x22,999])
 add('render_first_short_circuit',render_formats=[[7,999],[0x22,0]])
 add('render_format_bad',render_formats=[[7,999]])
 add('render_format_negative',render_count=-1)
 add('format_warning',pixel_type=1,bpp=24)
 add('format_type5_skips_bpp',pixel_type=5,bpp=24)
 add('format_bpp32_skips_warning',pixel_type=1,bpp=32)
 add('keep_pixels_reject',flags=1<<20,pixels=False)
 add('large_unbacked_unsigned',width=0xffffffff,height=1,pixels=False)
 for fail,kw in [
  ('loading',{}),('default',dict(flags=0x40)),('create',{}),('low',{}),('high',{}),
  ('open',{}),('load',{}),('close',{}),('reload',dict(flags=0x10,old=1)),
  ('assert',dict(flags=0x10)),('missing',dict(flags=0x40,default=0)),
  ('keep-diagnostic',dict(flags=1<<20,pixels=False)),('width',{}),('height',{}),
  ('png',dict(width=65,height=67)),('resize-warning',dict(width=65,height=67)),
  ('resize-format',dict(width=65,height=67,required_format=[7,0])),('resize',dict(width=65,height=67)),
  ('oversized',dict(width=2048,height=2048)),('account',{}),
  ('render-format',dict(render_formats=[[7,999]])),('pixel-warning',dict(pixel_type=1,bpp=24))]:add('required_'+fail,fail=fail,**kw)
 return out
def run(c,image,pc):
 m=n.Machine(pc)
 if image=='v14':
  old=(n.original.image_size+0xfff)&~0xfff;new=(n.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(n.original.base+old,new-old)
  for vs,rva,raw,off in n.modified.sections:m.u.mem_write(n.modified.base+rva,n.modified.b[off:off+raw])
 resource=m.alloc(0x100);events=[];active=[False];sources={};pixels={};source_ids={0:0};pixel_ids={0:0}
 def emit(event):
  events.append(event)
  if c['fail']==event[0]:raise Required(event[0])
 def string(text,wide=True):
  b=text.encode('utf-16le'if wide else'ascii')+(b'\0\0'if wide else b'\0');h=m.alloc(16+len(b));m.w(h+4,len(text));m.w(h+0xc,100);m.u.mem_write(h+16,b);return h+16
 m.w(resource+0x38,string('sample.PNG'if c['png']else'sample.TGA')if c['filename']else string(''))
 m.w(resource+8,c['flags']);m.w(resource+0x28,c['locations']);m.w(resource+0x40,0x12340000 if c['child']else 0)
 m.w(resource+0x54,0xcccccccc);m.w(resource+0x58,0xdddddddd);m.w(resource+0x4c,0x3f800000);m.w(resource+0x50,0x3f800000)
 count=len(c['render_formats'])if c['render_count']is None else c['render_count'];m.w(resource+0x60,count);formats=m.alloc(max(36,36*len(c['render_formats'])))
 for i,fmt in enumerate(c['render_formats']):m.w(formats+i*36+0x1c,fmt[0]);m.w(formats+i*36+0x20,fmt[1])
 m.w(resource+0x5c,formats);m.w(0xb3d8e4,c['memory']);m.w(0xb43d48,c['required_format'][0]);m.w(0xb43d4c,c['required_format'][1]);m.wb(0xb3d8f0,int(c['warning']))
 no_ni=m.alloc(4);m.wb(no_ni,c['no_ni']);m.w(0xb3d8f8,1);m.w(0xb3d8f4,no_ni)
 low=m.alloc(4);high=m.alloc(4);m.wb(low,c['low']);m.wb(high,c['high']);low_slot=m.array([low]);high_slot=m.array([high])
 vt=m.alloc(0x50);pvt=m.alloc(4)
 def destroy(mm):mm.ret(0,4)
 m.w(vt,m.stub(destroy));m.w(pvt,m.stub(destroy));m.w(vt+0x34,0x87baa0);m.w(vt+0x38,0x87bac0);m.w(vt+0x3c,0x87be40)
 for key in [1,2,3]:
  p=m.alloc(0x50);sources[key]=p;source_ids[p]=key;m.w(p,vt);m.w(p+4,c['source_refs']);m.wb(p+0x44,0);m.w(p+0x48,0xeeeeeeee)
  pixel=m.alloc(0x50);pixels[key]=pixel;pixel_ids[pixel]=key;m.w(pixel,pvt);m.w(pixel+4,c['pixel_refs']);m.w(pixel+0xc,c['pixel_type']);m.wb(pixel+0x24,c['bpp']);m.w(pixel+0x28,c['format'][0]);m.w(pixel+0x2c,c['format'][1]);m.w(pixel+0x38,m.array([c['width']]));m.w(pixel+0x3c,m.array([c['height']]));m.w(pixel+0x40,m.array([c['bytes']]));m.w(pixel+0x44,0)
  initial=(c['flags']&0x50!=0 or c['no_ni']==1)and c['pixels']
  m.w(p+0x40,pixel if initial or key!=1 and c['pixels']else 0)
  if c['renderer']is not None:
   renderer=m.alloc(8);rvt=m.alloc(16);m.w(renderer,rvt)
   m.w(rvt+8,m.stub(lambda mm:mm.ret(c['renderer'][0])));m.w(rvt+12,m.stub(lambda mm:mm.ret(c['renderer'][1])));m.w(p+0x2c,renderer)
 m.w(resource+0x3c,sources[c['old']]if c['old']else 0)
 if c['locations']:
  locations=m.alloc(4);location=0
  if c['writable']is not None:
   location=m.alloc(4);lvt=m.alloc(16);m.w(location,lvt);m.w(lvt+12,m.stub(lambda mm:mm.ret(int(c['writable']))))
  m.w(locations,location);m.w(resource+0x24,locations)
 def loading(mm):emit(['loading']);mm.ret()
 m.seam(0x404f90,loading)
 def log(mm):
  line=mm.arg(3);kind={0x229:'missing',0x233:'keep-diagnostic',0x1db:'oversized',0x1bc:'resize-warning',0x1c8:'resize-missing',0x1cc:'resize-format',0x251:'render-format',0x257:'pixel-warning'}[line]
  emit([kind]);mm.ret()
 for a in [0x404ed0,0x404ea0,0x404f60]:m.seam(a,log)
 def default(mm):emit(['default']);mm.ret(sources[c['default']]if c['default']else 0)
 m.seam(0x5a4d80,default)
 def assertion(mm):emit(['assert']);mm.ret(int(c['assert_value']))
 m.seam(0x855d30,assertion)
 def reload(mm):emit(['reload']);mm.ret()
 m.w(vt+0x44,m.stub(reload))
 def create(mm):emit(['create']);mm.ret(sources[c['created']]if c['created']else 0)
 m.seam(0x87be50,create)
 def ansi(mm):out=mm.ecx();m.w(out,string('sample',False));mm.ret(out,4)
 m.seam(0x454810,ansi)
 def quality(mm):
  a=m.r(mm.arg(1));name='low'if a==0x9f8cb8 else'high';assert a in [0x9f8cb8,0x9f8cf8];emit([name]);mm.ret(low_slot if name=='low'else high_slot,4)
 m.seam(0x405370,quality)
 stream=m.alloc(4);svt=m.alloc(0x14);m.w(stream,svt);m.w(svt+0x10,m.stub(lambda mm:mm.ret(0,4)))
 def open_stream(mm):emit(['open']);mm.ret(stream)
 m.seam(0x5aa340,open_stream);m.seam(0x59a9b0,lambda mm:mm.ret(mm.ecx(),8))
 def load(mm):emit(['load']);m.w(mm.ecx()+0x40,pixels[1]if c['pixels']else 0);mm.ret(1,4)
 m.w(vt+0x40,m.stub(load))
 def close(mm):emit(['close']);mm.ret()
 m.seam(0x8761a0,close)
 def name(mm):out=mm.arg(1);m.w(out,string('sample'));mm.ret(out,4)
 m.seam(0x5aa860,name)
 def extension(mm):out=mm.arg(1);m.w(out,string('PNG'if c['png']else'TGA'));mm.ret(out)
 m.seam(0x85a660,extension)
 def compare(mm):emit(['png']);mm.ret(0 if c['png']else 1)
 m.seam(0x96cad6,compare)
 def resize(mm):emit(['resize']);mm.ret(sources[c['resize']]if c['resize']else 0)
 m.seam(0x87c5c0,resize)
 def account(mm):assert mm.arg(1)==mm.arg(2);emit(['account',mm.arg(1)]);mm.ret(0,8)
 m.seam(0x5aa270,account)
 #Compatibility diagnostics transport formatting, without bypassing its real
 #native compare/list loops. Keep CString references positive for their cleanup.
 format_owner=m.alloc(4);fvt=m.alloc(16);m.w(format_owner,fvt);m.w(fvt+12,m.stub(lambda mm:mm.ret(string('format')-16)))
 m.seam(0x856370,lambda mm:mm.ret(format_owner))
 def format_list(mm):out=mm.u.reg_read(n.UC_X86_REG_EAX);m.w(out,string('formats'));mm.ret(out)
 m.seam(0x5a8380,format_list)
 def width(mm):emit(['width'])
 def height(mm):emit(['height'])
 m.observe(0x87baa0,width);m.observe(0x87bac0,height)
 def null_guard(mm):
  at=mm.u.reg_read(n.UC_X86_REG_EIP)
  if at==0x5a8e53 and mm.u.reg_read(UC_X86_REG_EDI)==0:raise Required('null-direct')
  if at in [0x5a8c30,0x5a8c57]and mm.u.reg_read(UC_X86_REG_ESI)==0:raise Required('null-quality'if at==0x5a8c30 else'null-stream')
  if at==0x5a8f8d and mm.u.reg_read(n.UC_X86_REG_EBX)==0:raise Required('null-resized')
 for a in [0x5a8e53,0x5a8c30,0x5a8c57,0x5a8f8d]:m.observe(a,null_guard)
 def writes(u,access,a,size,value,_):
  if not active[0]:return
  if a in [resource+8,resource+0x3c,resource+0x54,resource+0x58,resource+0x4c,resource+0x50]:events.append(['entry',hex(a-resource),source_ids.get(value,value)if a==resource+0x3c else value])
  elif a==0xb3d8e4:events.append(['memory',value])
  elif a==0xb3d8f0:events.append(['warning',value])
  else:
   for key,p in sources.items():
    if a in [p+4,p+0x40,p+0x44,p+0x48]:events.append(['source',key,hex(a-p),pixel_ids.get(value,value)if a==p+0x40 else value]);return
   for key,p in pixels.items():
    if a==p+4:events.append(['pixel',key,value]);return
 m.u.hook_add(UC_HOOK_MEM_WRITE,writes)
 active[0]=True;stopped=None;result=None
 try:result=m.call(0x5a8cd0,resource,[])
 except Required as e:stopped=str(e)
 output=dict(events=events,flags=m.r(resource+8),current=source_ids.get(m.r(resource+0x3c)),words={hex(a):m.r(resource+a)for a in [0x54,0x58,0x4c,0x50]},memory=m.r(0xb3d8e4),warning=m.rb(0xb3d8f0),
  sources={str(k):dict(refs=m.r(p+4),pixels=pixel_ids.get(m.r(p+0x40)),discard=m.rb(p+0x44),quality=m.r(p+0x48))for k,p in sources.items()},pixel_refs={str(k):m.r(p+4)for k,p in pixels.items()},stopped=stopped,result=result)
 return output,m.visited
def main():
 rows=[];pins={}
 for pc,c in itertools.product(n.PRECISIONS,cases()):
  a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b,(pc,c,a,b);pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert n.original.range(va,len(raw))==n.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,original_sha256=n.sha(n.original.b),v14_sha256=n.sha(n.modified.b),rows=rows,native_executions=len(rows)*2,instructions=n.instructions(pins))
 path=HERE/'native-texture-load.json'
 if '--write'in sys.argv:
  with path.open('x')as f:json.dump(doc,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=n.sha(path.read_bytes()))))
if __name__=='__main__':main()
