"""Real 856E90/856BF0/877AC0 memory text stream and 859C60/85AB30 filename.
Original and v14, all x87 PCs. Successful heap/CString transports and one
explicit C-locale CRT-thread owner are supplied; 970FDF->970F7F execute.
877A50 executes around named regular-file output ctor/destructor transports.
859B30 executes around a labelled DeleteFileW transport, success and failure.
No DataFileResource lookup, native Win32 failure/unwind or CRT startup claim.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn.x86_const import *
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(h.Machine):
 def __init__(self,image,pc):
  super().__init__(b'',pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   for vs,r,size,off in image.sections:
    if size:self.u.mem_write(image.base+r,image.b[off:off+size])
  self.trace=[];self.output=[];self.freed=[];self.allocations=[]
  def allocate(m):
   n=m.arg(0);p=m.alloc(n,0xa5);self.allocations.append(n);m.ret(0,p)
  self.seam(0x6f7810,allocate)
  self.seam(0x6f7430,lambda m:(self.freed.append(m.arg(0)),m.ret(0)))
  # A named C-locale CRT-thread transport, not invented successful conversion:
  # native 970F7F checks locale+14==0 and writes exactly U+0000..U+00FF.
  locale=self.alloc(0x50);thread=self.alloc(0x80);self.w(thread+0x64,locale)
  self.w(0xb35444,locale);self.seam(0x9738a9,lambda m:m.ret(0,thread))
  def output(m):
   self.output.append(bytes(m.u.mem_read(m.arg(0),m.arg(1))).hex());m.ret(8,m.u.reg_read(UC_X86_REG_ECX))
  self.output_vtable=self.vtable({4:output})
  self.out=self.alloc(4);self.w(self.out,self.output_vtable)
  def file_ctor(m):
   self.trace.append(['file_open',m.cstring_text(m.r(m.arg(0))),m.arg(1)])
   self.w(m.u.reg_read(UC_X86_REG_ECX),self.output_vtable);m.ret(8,m.u.reg_read(UC_X86_REG_ECX))
  self.seam(0x858660,file_ctor)
  self.seam(0x858230,lambda m:(self.trace.append(['file_close']),m.ret(0)))
  self.delete_result=False
  def delete(m):
   self.trace.append(['DeleteFileW',m.cstring_text(m.arg(0)),self.delete_result]);m.ret(4,int(self.delete_result))
  self.w(0xb3641c,self.stub(delete))
 def cdecl(self,entry,args):
  self.w(h.STACK,h.STOP)
  for i,a in enumerate(args):self.w(h.STACK+4+i*4,a)
  self.u.reg_write(UC_X86_REG_ESP,h.STACK)
  self.u.emu_start(entry,h.STOP,count=10000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==h.STOP
  assert self.u.reg_read(UC_X86_REG_ESP)==h.STACK+4
  return self.u.reg_read(UC_X86_REG_EAX)
 def run(self,c):
  raw=c['text'].encode('latin1');assert b'\0'not in raw
  assert self.r(0xab4788)==0x2010a
  assert self.read_wide(0xa0cf30)=='TGI'
  namebox=self.alloc(4);self.w(namebox,self.cstring(c['name']))
  stripped=self.alloc(4);self.cdecl(0x859c60,[stripped,namebox])
  ext=self.alloc(4);self.w(ext,self.cstring('TGI'));self.cdecl(0x85ab30,[stripped,ext])
  filename=self.cstring_text(self.r(stripped))
  wrapper=self.alloc(0x10,0xa5)
  self.call(0x856e90,wrapper,[stripped,0x2010a|0x40000])
  child=self.r(wrapper+4)
  initial={'wrapper_vtable':self.r(wrapper),'child_vtable':self.r(child),'flags':self.r(child+4),'open':self.b(child+8),'capacity':self.r(child+0x14),'position':self.r(child+0x10),'length':self.r(child+0x18),'encoding_bytes':[self.b(wrapper+i)for i in(0xc,0xd,0xe)]}
  self.call(0x856bf0,wrapper,[self.cstring(c['text'])],count=10000000)
  length=self.r(child+0x18);data=bytes(self.u.mem_read(self.r(child+0x1c),length))
  built={'position':self.r(child+0x10),'length':length,'capacity':self.r(child+0x14),'bytes':data.hex()}
  self.call(0x877ac0,wrapper,[self.out,0xdeadbeef])
  forwarded=self.output[-1]
  # Filename by-value argument used by actual 877A50 then 859B30.
  self.call(0x877a50,wrapper,[stripped,0x12345678])
  separate=self.output[-1]
  results=[]
  for flag in(False,True):
   self.delete_result=flag;results.append(self.cdecl(0x859b30,[self.r(stripped)])&0xff)
  self.call(0x856e10,wrapper,[])
  after={'child':self.r(wrapper+4),'open':self.b(child+8),'position':self.r(child+0x10),'length':self.r(child+0x18),'capacity':self.r(child+0x14),'buffer':self.r(child+0x1c)}
  assert data==raw.replace(b'\n',b'\r\n')
  assert forwarded==data.hex()==separate
  assert results==[0,1]
  return {'filename':filename,'initial':initial,'built':built,'forwarded':forwarded,'separate':separate,'trace':self.trace,'delete_returns':results,'released':after,'heap_sizes':self.allocations,'frees':len(self.freed)}
def cases():
 names=['Save.film','Save','folder.with.dot/name','folder\\Save.film','a.b.c','.hidden','trailing.','']
 texts=['','hello','A\nB\r\nC\rD\tE','[Text language=Default]\n{\n\tname = "Map"\n}\n','\u00ff\u0080\u00e9','x'*16383+'\n'+'y'*1025]
 for i,name in enumerate(names):yield {'name':name,'text':texts[i%len(texts)]}
 for text in texts:yield {'name':'Test.film','text':text}
def main():
 rows=[]
 for c in cases():
  for pc in h.CW:
   out=[Machine(image,pc).run(c)for image in(h.P,h.V)]
   assert out[0]==out[1];rows.append({'input':c,'pc':pc,'native':out[0]})
 seams={0x6f7810:'successful_heap_allocate_transport',0x6f7430:'successful_heap_free_transport',0x856370:'CString_allocator_transport',0x9738a9:'explicit_C_locale_CRT_thread_transport',0x401b00:'CString_literal_transport',0x856560:'CString_literal_transport',0x858660:'regular_file_output_ctor_transport',0x858230:'regular_file_output_dtor_transport'}
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(h.P.b),'v14_sha256':H(h.V.b),'rows':rows,'machines':len(rows)*2,'pins':[{'va':hex(a),'bytes':v,'kind':seams.get(a,'executed')}for a,v in sorted(h.VISITED.items())],'default_flags':0x2010a,'extension':'TGI'}
 p=HERE/'native-data-text.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':len(rows)*2,'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
