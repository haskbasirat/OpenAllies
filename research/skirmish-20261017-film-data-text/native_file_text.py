"""Actual877A50->858660->876C60 open, memory rawforward,876600 flush/close.
Successful CreateFileW/WriteFile/SetFilePointer/CloseHandle transports are named;
the directory preparation callback is explicit, for a preexisting test parent.
No native filesystem manager constructor/failure/Win32 sharing proof is claimed.
"""
from pathlib import Path
import json,sys,hashlib
import native_data_text as b
h=b.h;HERE=Path(__file__).resolve().parent
H=lambda v:hashlib.sha256(v).hexdigest()
class Machine(b.Machine):
 def __init__(self,image,pc):
  super().__init__(image,pc)
  del self.seams[0x858660];del self.seams[0x858230]
  self.file_bytes=bytearray();self.file_calls=[];self.position=0
  self.fs=self.alloc(4)
  def directory(m):self.file_calls.append(['directory',m.cstring_text(m.r(m.arg(0)))]);m.ret(4,1)
  self.w(self.fs,self.vtable({0x34:directory}));self.w(0xb43b74,self.fs)
  def create(m):
   self.file_calls.append(['CreateFileW',m.read_wide(m.arg(0)),m.arg(1),m.arg(2),m.arg(4),m.arg(5)])
   m.ret(28,0x110)
  self.w(0xb3640c,self.stub(create))
  def write(m):
   assert m.arg(0)==0x110 and m.arg(4)==0
   n=m.arg(2);data=bytes(m.u.mem_read(m.arg(1),n));self.file_calls.append(['WriteFile',n])
   if self.position+n>len(self.file_bytes):self.file_bytes.extend(b'\0'*(self.position+n-len(self.file_bytes)))
   self.file_bytes[self.position:self.position+n]=data;self.position+=n
   m.w(m.arg(3),n);m.ret(20,1)
  self.w(0x9f01b8,self.stub(write))
  def seek(m):
   assert m.arg(0)==0x110
   distance=m.arg(1);method=m.arg(3)
   if distance&0x80000000:distance-=1<<32
   self.position=distance+(0 if method==0 else self.position if method==1 else len(self.file_bytes))
   self.file_calls.append(['SetFilePointer',distance,method]);m.ret(16,self.position)
  self.w(0x9f0288,self.stub(seek))
  def close(m):assert m.arg(0)==0x110;self.file_calls.append(['CloseHandle']);m.ret(4,1)
  self.w(0x9f0234,self.stub(close))
  def flush(m):assert m.arg(0)==0x110;self.file_calls.append(['FlushFileBuffers']);m.ret(4,1)
  self.w(0x9f0180,self.stub(flush))
  def size(m):
   assert m.arg(0)==0x110
   if m.arg(1):m.w(m.arg(1),0)
   self.file_calls.append(['GetFileSize']);m.ret(8,len(self.file_bytes))
  self.w(0x9f0274,self.stub(size))
 def run_file(self,c):
  name=self.alloc(4);self.w(name,self.cstring(c['name']))
  wrapper=self.alloc(16,0xa5);self.call(0x856e90,wrapper,[name,0x6010a])
  self.call(0x856bf0,wrapper,[self.cstring(c['text'])])
  self.call(0x877a50,wrapper,[name,0])
  self.call(0x856e10,wrapper,[])
  assert self.file_bytes==c['text'].encode('latin1').replace(b'\n',b'\r\n')
  assert self.file_calls[1][2:]==[0x40000000,0,2,0x80]
  return {'bytes':self.file_bytes.hex(),'calls':self.file_calls}
def main():
 rows=[]
 for text in['','hello\n','\u00ff\u0080\r\n\tend','x'*16384+'tail']:
  c={'name':'E:\\Rewrite\\Userdata\\native.TGI','text':text}
  for pc in h.CW:
   out=[Machine(image,pc).run_file(c)for image in(h.P,h.V)];assert out[0]==out[1]
   rows.append({'input':c,'pc':pc,'native':out[0]})
 seamset={0x6f7810,0x6f7430,0x856370,0x401b00,0x856560,0x9738a9}
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'base_producer_sha256':H(Path(b.__file__).read_bytes()),'original_sha256':H(h.P.b),'v14_sha256':H(h.V.b),'rows':rows,'machines':len(rows)*2,'pins':[{'va':hex(a),'bytes':v,'kind':'transport_entry'if a in seamset else'executed'}for a,v in sorted(h.VISITED.items())]}
 p=HERE/'native-file-text.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':len(rows)*2,'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
