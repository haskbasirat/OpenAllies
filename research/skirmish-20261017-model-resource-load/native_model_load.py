"""Actual5A99C0 field/order oracle, allPC/bothimages. Selected NIF decoder,
87AA10 metadata, graph preparation/root wrap/update, controller/selection and
current UI are declared transports. Native scalar flags/bounds stores,
memory multipliers and publication execute unchanged. No decoded NIF claim.
"""
from pathlib import Path
import sys,json,hashlib,gc,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE,UC_HOOK_MEM_WRITE,UC_HOOK_MEM_READ
from unicorn.x86_const import *
class Stop(Exception):pass
def run(c,image,pc,m=None):
 full.order.T.P=image;m=h.Machine()if m is None else m;m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);events=[];pins={};seams=set();active=[False]
 def event(name,*args):
  row=[name,*args];events.append(row)
  if c['fail']==name or(c['fail']=='texture1'and name=='texture'and args[0]==1):raise Stop(c['fail'])
 def capture(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
  if active[0]and at==0x5a9a28:event('filename')
  if active[0]and at==0x5a9b73:event('present')
  if active[0]and at==0x5a9bb7:event('context')
 capture_hook=m.u.hook_add(UC_HOOK_CODE,capture)
 resource=m.alloc(0x90);root=m.alloc(0x100);rootvt=m.alloc(0x70);wrapped=[0];stream=m.alloc(4);svt=m.alloc(0x34);metadata=m.alloc(0x28)
 m.w(resource+8,c['flags']);m.w(resource+0x38,m.cstring(c['model_name']if c['filename']else''));m.w(resource+0x8c,m.cstring(''))
 for o,v in [(0xc,17),(0x10,19),(0x48,0x41200000),(0x64,0x3f800000),(0x68,0x3f800000),(0x6c,0x3f800000),(0x70,0x3f800000),(0x74,0x3f800000),(0x78,0),(0x7c,0),(0x84,0)]:m.w(resource+o,v)
 m.w(root,rootvt);m.w(root+4,100);m.w(root+0xc,m.cstring('Root'));m.w(stream,svt)
 m.w(metadata+4,c['value48']);m.w(metadata+8,c['radius']);m.w(metadata+0xc,c['max_z']);m.w(metadata+0x10,c['min_z']);m.w(metadata+0x14,c['footprint']);m.w(metadata+0x18,c['center'][0]);m.w(metadata+0x1c,c['center'][1]);m.w(metadata+0x20,len(c['textures']))
 vector=m.alloc(12);m.w(vector,1);m.w(resource+0x80,vector+4);m.w(resource+0x88,1);m.thiscall(0x5a6d20,vector+4,[])
 m.u.mem_write(metadata+1,bytes([c['byte1']]));m.u.mem_write(metadata+0x24,bytes([c['byte24']]))
 no_ni=m.alloc(4);m.w(no_ni,c['no_ni']);m.w(0xb3d924,3);m.w(0xb3d920,no_ni)
 avars=m.alloc(0xd0);count=m.alloc(4);m.w(count,c['hardware_count']);m.w(avars+0xcc,count);m.w(0xb39990,avars)
 uvars=m.alloc(0xdc);skip=m.alloc(4);m.w(skip,c['skip']);m.w(uvars+0xd8,skip);m.w(0xb39940,uvars)
 def stub(at,f):m.stubs[at]=f
 stub(0x59ab60,lambda m,u:(event('begin'),m.ret())[1])
 stub(0x5aa340,lambda m,u:(event('open'),m.ret(stream))[1])
 stub(0x6a71f0,lambda m,u:m.ret(pop=4));m.w(rootvt,0x6a71f0)
 stub(0x6a7210,lambda m,u:(event('size'),m.ret(c['bytes']))[1]);m.w(svt+0x30,0x6a7210);m.w(svt+0x10,0x6a71f0)
 stub(0x59a930,lambda m,u:m.ret(u.reg_read(UC_X86_REG_ECX),pop=4))
 def decode(m,u):event('decode');out=m.r(u.reg_read(UC_X86_REG_ESP)+4);m.w(out,root if c['payload']else 0);m.ret(out)
 stub(0x5a7120,decode);stub(0x876010,lambda m,u:(event('close'),m.ret())[1])
 stub(0x5a6610,lambda m,u:(event('default'),m.ret(root if c['payload']else 0))[1])
 stub(0x5aa860,lambda m,u:(m.w(m.r(u.reg_read(UC_X86_REG_ESP)+4),m.cstring('model')),m.ret(m.r(u.reg_read(UC_X86_REG_ESP)+4),pop=4))[1])
 def unable(m,u):event('unable');raise Stop('unable')
 stub(0x4252a0,unable)
 def profile(m,u):
  label=m.raw_wstr(m.r(u.reg_read(UC_X86_REG_ESP)+4))
  if label not in ['LOAD','REMOVEBASEMAP']:event('profile',label)
  m.ret(pop=4)
 stub(0x40f9d0,profile)
 stub(0x73a560,lambda m,u:(event('hardware'),m.ret(int(c['hardware'])))[1])
 stub(0x5a65b0,lambda m,u:(event('hardware_transform',m.r(u.reg_read(UC_X86_REG_ESP)+8)),m.ret(root,pop=8))[1])
 stub(0x87aa10,lambda m,u:(event('metadata',m.r(u.reg_read(UC_X86_REG_ESP)+12)&255),m.ret(metadata))[1])
 stub(0x8784f0,lambda m,u:(event('names'),m.ret(root))[1])
 def texture(m,u):
  index=m.r(u.reg_read(UC_X86_REG_ESP)+4);event('texture',index);raw=c['textures'][index].encode('ascii')+b'\0';p=m.alloc(len(raw));m.u.mem_write(p,raw);m.ret(p,pop=4)
 stub(0x8873c0,texture)
 def ascii_name(m,u):
  p=m.r(u.reg_read(UC_X86_REG_ESP)+4);raw=bytearray()
  while m.u.mem_read(p,1)!=b'\0':raw.extend(m.u.mem_read(p,1));p+=1
  out=u.reg_read(UC_X86_REG_ECX);m.w(out,m.cstring(raw.decode('ascii')));m.ret(out,pop=4)
 stub(0x4094f0,ascii_name)
 stub(0x871bc0,lambda m,u:(event('effect',0x871bc0),m.ret())[1]);stub(0x871b60,lambda m,u:(event('effect',m.r(u.reg_read(UC_X86_REG_ESP)+8)),m.ret())[1]);stub(0x871b40,lambda m,u:(event('effect',0x871b40),m.ret())[1])
 stub(0x5a9840,lambda m,u:(event('controllers'),m.ret(int(c['controllers'])))[1])
 def selection(m,u):
  event('selection');r=u.reg_read(UC_X86_REG_ECX)
  for i,v in enumerate(c['selection']):m.w(r+0x4c+4*i,v)
  m.ret(pop=4)
 stub(0x5a94f0,selection)
 def ctor(m,u):
  event('wrap');p=u.reg_read(UC_X86_REG_ECX);wrapped[0]=p;m.w(p,rootvt);m.w(p+4,1);m.w(p+0xc,m.cstring(''));m.ret(p,pop=4)
 stub(0x867690,ctor);stub(0x86aa60,lambda m,u:m.ret(pop=4));stub(0x6a7220,lambda m,u:m.ret(pop=8));m.w(rootvt+0x6c,0x6a7220)
 stub(0x871cc0,lambda m,u:m.ret(0));stub(0x86e8d0,lambda m,u:(event('update'),m.ret(pop=8))[1]);stub(0x41c3f0,lambda m,u:(event('display'),m.ret())[1])
 offsets=[8,0xc,0x10,0x40,*range(0x48,0x80,4),0x84]
 def writes(u,access,a,size,value,_):
  if active[0]and size==4 and a-resource in offsets:events.append(['write',a-resource,int(bool(value))if a-resource==0x40 else value&0xffffffff])
 def reads(u,access,a,size,value,_):
  if active[0]and a==no_ni:event('no_ni')
  if active[0]and a==count:event('hardware_count')
  if active[0]and a==skip:event('skip')
 write_hook=m.u.hook_add(UC_HOOK_MEM_WRITE,writes);read_hook=m.u.hook_add(UC_HOOK_MEM_READ,reads)
 result=None;stopped=None;active[0]=True
 try:result=m.thiscall(0x5a99c0,resource,[])['eax']
 except Stop as e:stopped=str(e)
 active[0]=False
 pairs=[[m.wstr(m.r(m.r(resource+0x80)+8*i)),m.wstr(m.r(m.r(resource+0x80)+8*i+4))]for i in range(m.r(resource+0x84))]
 output=dict(events=events,words={str(o):int(bool(m.r(resource+o)))if o==0x40 else m.r(resource+o)for o in offsets},pairs=pairs,capacity=m.r(resource+0x88),stopped=stopped,result=result)
 m.check_null();m.u.hook_del(capture_hook);m.u.hook_del(write_hook);m.u.hook_del(read_hook);return output,pins,seams
def cases():
 base=dict(flags=0x400,no_ni=0,filename=True,model_name='Art/Models/model.nif',bytes=123,payload=True,hardware=False,hardware_count=27,byte1=0,byte24=0,value48=0x3f800000,radius=0x41200000,max_z=0x40e00000,min_z=0xbf800000,footprint=0x40a00000,center=[0x3f800000,0xc0000000],controllers=False,skip=0,selection=[0xbf800000]*3+[0x3f800000]*3,textures=[],fail=None)
 yield base
 for flags,no_ni in itertools.product([0,0x100,0x400,0x8000,0x80000,0x400000,0x2000,4],[0,1,2]):yield dict(base,flags=flags,no_ni=no_ni)
 for name in ['begin','open','size','decode','close','context','hardware','metadata','effect','controllers','selection','wrap','update','display']:yield dict(base,fail=name)
 for radius,footprint in [(0,0),(0x80000000,0),(0xbf800000,0x41200000),(0x41200000,0),(0x41200000,0xbf800000)]:yield dict(base,radius=radius,footprint=footprint)
 yield dict(base,byte1=1,byte24=1,controllers=True);yield dict(base,payload=False);yield dict(base,filename=False);yield dict(base,hardware=True);yield dict(base,flags=4,skip=1)
 for size in [0,1,5,16777217,0x7fffffff,0x80000001,0xffffffff]:yield dict(base,bytes=size)
 for names in [['Diffuse.dds'],['a.tga','B.dds'],['a.tga','sub/b.tga','/Root.dds','\\Start.dds']]:yield dict(base,textures=names)
 for stop in ['names','texture','texture1']:yield dict(base,textures=['a.dds','b.dds'],fail=stop)
 for model in ['model.nif','Art\\Models\\model.NIF','A:/model.nif','A:model.nif','folder//model.nif']:yield dict(base,model_name=model,textures=['Diffuse.dds','/Root.dds','\\Start.dds'])
def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  full.order.T.P=h.P;original=h.Machine();full.order.T.P=h.V;expanded=h.Machine()
  for c in cases():
   a,pa,sa=run(c,h.P,pc,original);b,pb,sb=run(c,h.V,pc,expanded);assert a==b and pa==pb,(c,pc,a,b)
   rows.append(dict(input=c,pc=pc,output=a));pins.update(pa);seams|=sa|sb
 out=dict(scope=__doc__,cases=rows,executed={hex(a):b for a,b in sorted(pins.items())},intercepted=[hex(a)for a in sorted(seams)])
 data=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'native-model-load.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(paired_rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
