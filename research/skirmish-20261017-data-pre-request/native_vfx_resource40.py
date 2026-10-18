"""Selected native VFX Resource40 loader prefix:495570 and495F10.
Common494F30 (Texture caller), GetString/retained CString, factory and warning
are explicit services.4763C0 is composed from its separately executed fixture.
Stops before unrelated remaining type-LoadData; not full source-load order.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ECX
REL=HERE/'native-relative-source-path.json'
class End(Exception):pass
class Required(Exception):pass

def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 def string(value):
  raw=value.encode('utf-16-le');p=m.alloc(len(raw)+20);m.w(p+4,len(raw)//2);m.w(p+12,1000);m.u.mem_write(p+16,raw+b'\0\0');return p+16
 def text(p):
  raw=bytearray()
  for i in range(4096):
   u=bytes(m.u.mem_read(p+i*2,2))
   if u==b'\0\0':return raw.decode('utf-16-le')
   raw+=u
  raise AssertionError('unterminated child CString')
 data=m.alloc(0x80);source=m.alloc(4);value=m.alloc(4);m.w(value,string(c['value']));m.w(data+0x20,7);m.wb(data+0x38,c['unique'])
 resource=m.alloc(12);m.w(resource+8,c['flags']);events=[]
 def get(mm):
  key=text(mm.arg(1));assert key==('texture' if c['kind']=='texture' else 'model');events.append(['get',key])
  mm.ret(value,4)
 m.seam(0x5b54d0,get);m.seam(0x4012c0,lambda mm:mm.ret(mm.arg(1)))
 def relative(mm):
  ref=mm.arg(1);raw=text(mm.r(ref));assert raw==c['value'];events.append(['relative',raw])
  rows=json.loads(REL.read_text())['rows'];selected=[x for x in rows if x['precision']==pc and x['folder']=='Units/GBR' and x['value']==raw];assert len(selected)==1
  mm.w(ref,string(selected[0]['output']));mm.ret(0,4)
 m.seam(0x4763c0,relative)
 def factory(mm):
  events.append(['factory',text(mm.r(mm.arg(1))),None if c['kind']=='model' else mm.arg(2)])
  if c['factory']=='required':raise Required('factory')
  mm.ret(resource if c['factory']=='present' else 0,0)
 m.seam(0x5a77e0 if c['kind']=='texture' else 0x5a8520,factory)
 m.seam(0x494f30,lambda mm:mm.ret(0,4))
 def warning(mm):events.append(['warning',mm.r(data+0x40)==0]);raise Required('warning')
 m.seam(0x404ed0,warning);m.seam(0x49560a if c['kind']=='texture' else 0x495fba,lambda mm:(_ for _ in()).throw(End()))
 status='prefix_complete'
 try:m.call(0x495570 if c['kind']=='texture' else 0x495f10,data,[source])
 except End:pass
 except Required as e:status='required_'+str(e)
 return dict(status=status,pointer='resource' if m.r(data+0x40)==resource else 'null',flags=m.r(resource+8),count=m.r(data+0x20),events=events),m.visited

def main():
 rows=[];pins={}
 for pc,kind,unique,flags,factory,value in itertools.product(nm.PRECISIONS,['texture','model'],[False,True],[0x400,0xffffffff],['present','null','required'],['x.dds','/x.dds']):
  c=dict(kind=kind,unique=unique,flags=flags,factory=factory,value=value);a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),'relative_fixture_sha256':hashlib.sha256(REL.read_bytes()).hexdigest(),'harness_sha256':hashlib.sha256((ROOT/'research/phase8-ai/native_machine.py').read_bytes()).hexdigest()}
 path=HERE/'native-vfx-resource40.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
