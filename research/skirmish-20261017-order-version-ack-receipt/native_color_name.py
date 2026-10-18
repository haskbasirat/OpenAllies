"""Real474400 Color constructor and47E0F0 ->5B5620(name)->405B80.

Native TGI parser creates every source node. Both PEs/all precision controls
execute the actual localized-field primitive and Color+10 copy/14 store.
4B9F40 is the named definition-time table boundary; missing keys stop there.
RGB5B4C30 is a named later callback/cutoff, and pool/heap are transport.
This proves the reached immutable label projection, not whole ColorDataLoad.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-restoration'))
from native_tgi_parser import Machine as Parser,lines_of,P,V
import trace_registry
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ECX,UC_X86_REG_FPCW

def sha(b):return hashlib.sha256(b).hexdigest()
def units(s):
 b=s.encode('utf-16-le','surrogatepass');return list(struct.unpack('<'+'H'*(len(b)//2),b))
class Cutoff(Exception):pass
class Machine(Parser):
 def __init__(self,image,src):
  self.color_ready=False;self.actual={};self.record_actual=False
  old=trace_registry.P
  try:trace_registry.P=image;super().__init__(lines_of(src))
  finally:trace_registry.P=old
  self.w(0xb43b48,self.string([]));self.w(0xb36544,self.alloc(0x40))
  self.trace=[];self.table={'blue_key':units('Blue'),'':units('Empty-key')}
  self.u.hook_add(UC_HOOK_MEM_WRITE,self.write)
 def stored(self,p):
  n=self.r(p-12);return list(struct.unpack('<'+'H'*n,self.u.mem_read(p,n*2)))
 def write(self,u,access,address,size,value,user):
  if self.color_ready and address==self.color+0x10:self.trace.append(['name_pointer_store_before_buffer_copy',self.stored(value)])
  if self.color_ready and address==self.color+0x14:self.trace.append(['store14',value])
 def hook(self,u,pc,size,user):
  if self.color_ready:
   if pc==0x4b9f40:
    key=self.text(self.r(self.arg()));self.trace.append(['table',key])
    if key not in self.table:raise Cutoff('missing definition-time lookup')
    out=self.alloc(4);self.w(out,self.string(self.table[key]));self.ret(out,pop=8);return
   if pc==0x410c40:raise Cutoff('duplicate color name')
   if pc==0x5b4c30:
    assert self.text(self.arg(1))=='color';self.trace.append(['rgb',self.stored(self.r(self.color+0x10)),self.u.mem_read(self.color+0x14,1)[0]])
    raise Cutoff('after stored label before RGB')
  super().hook(u,pc,size,user)
  if self.record_actual and u.reg_read(UC_X86_REG_EIP)==pc:self.actual[pc]=size

def case(image,precision,src,preset):
 m=Machine(image,src)
 resource=m.alloc(0x100);m.w(resource+0x38,m.string(units('ColorSource.tgi')))
 stream=m.alloc(0x40);vt=m.alloc(0x100);m.w(stream,vt);m.w(vt+0x48,0x301f600)
 m.u.reg_write(UC_X86_REG_FPCW,{24:0x7f,53:0x27f,64:0x37f}[precision])
 m.invoke(0x5bd080,resource,[stream]);assert m.thrown is None
 root=m.r(resource+0x3c);node=next(m.r(m.r(root+0x34)+i*4)for i in range(m.r(root+0x38))if m.text(m.r(m.r(m.r(root+0x34)+i*4)))=='Color')
 avars=m.alloc(0x100);point=m.alloc(0x10);m.w(avars+0x78,point);m.w(0xb39990,avars)
 m.u.mem_write(point+4,struct.pack('<3I',0,0x3e99999a,0x3f800000))
 m.color=m.alloc(0x24);m.actual={};m.record_actual=True
 m.invoke(0x474400,m.color,[m.string(units('blue'))])
 ctor={'name':m.stored(m.r(m.color+0x10)),'flag14':m.u.mem_read(m.color+0x14,1)[0],'ordinal':m.r(m.color+0xc),'vtable':hex(m.r(m.color))}
 if preset:m.w(m.color+0x10,m.string(units(preset)))
 m.trace=[];m.color_ready=True
 try:m.invoke(0x47e0f0,m.color,[node]);raise AssertionError('expected explicit RGB/source cutoff')
 except Cutoff as e:cutoff=str(e)
 return {'constructor':ctor,'stored':m.stored(m.r(m.color+0x10)),'flag14':m.u.mem_read(m.color+0x14,1)[0],'trace':m.trace,'cutoff':cutoff},m.actual

def main():
 template=b'[Template T]\n{\n string IDS\n localized name\n}\n'
 inputs=[('literal',b'name = "Literal Blue"',''),('translated',b'name = "#blue_key"',''),('empty',b'name = ""',''),('hash_only',b'name = "#"',''),('missing',b'name = "#absent"',''),('duplicate',b'name = "#blue_key"','old')]
 rows=[];pins={}
 for image_name,image in [('original',P),('v14',V)]:
  for precision in [24,53,64]:
   for label,body,preset in inputs:
    src=template+b'[Color template=T]\n{\n IDS=blue\n '+body+b'\n}\n'
    output,visited=case(image,precision,src,preset);pins.update(visited);rows.append({'image':image_name,'precision':precision,'case':label,'source_hex':src.hex(),'preset':preset,'output':output})
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 instructions=[]
 for va,size in sorted(pins.items()):
  if P.base<=va<P.base+P.image_size:
   raw=P.range(va,size);assert raw==V.range(va,size);instructions.append({'va':hex(va),'bytes':raw.hex()})
 doc={'schema':1,'scope':__doc__,'producer_sha256':sha(Path(__file__).read_bytes()),'original_sha256':sha(P.b),'v14_sha256':sha(V.b),'rows':rows,'instructions':instructions}
 out=HERE/'native-color-name.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps({'rows':len(rows),'pins':len(instructions),'sha256':sha(out.read_bytes())}))
if __name__=='__main__':main()
