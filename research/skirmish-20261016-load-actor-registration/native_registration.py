"""4D62F0 and actual477FC0 descriptor table; explicit transport/effect seams."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn.x86_const import *
P=h.P;V=h.V
H=lambda b:hashlib.sha256(b).hexdigest()
class Stop(Exception):pass
class Machine(h.Machine):
 def __init__(self,pc):
  super().__init__(pc);self.actor=self.alloc(0x200);self.data=self.alloc(0x400);self.array=self.alloc(0x1000)
  self.w32(0xb36530,self.data);self.w32(self.data+0x2e4,self.array);self.trace=[];self.records=[];self.names={}
 def text(self,p):
  b=bytearray()
  while bytes(self.u.mem_read(p,2))!=b'\0\0':b+=self.u.mem_read(p,2);p+=2
  return b.decode('utf-16le')
 def table(self):
  provider=self.alloc(0x20);vt=self.alloc(0x20);empty=self.alloc(0x30);stub=self.stub_next;self.stub_next+=16
  self.w32(provider,vt);self.w32(vt+12,stub);self.w32(empty+12,100000)
  self.seams[0x6f7810]=lambda m:m.ret(value=m.alloc(m.arg()))
  def cstring(m):
   s=m.text(m.arg());p=m.alloc(16+2*(len(s)+1));m.w32(p+4,len(s));m.w32(p+12,100000);m.u.mem_write(p+16,s.encode('utf-16le')+b'\0\0');m.names[p+16]=s;m.w32(m.ecx(),p+16);m.ret(4,p+16)
  self.seams[0x401b00]=cstring
  self.seams[0x4012c0]=lambda m:m.ret(value=m.arg())
  self.seams[0x856370]=lambda m:m.ret(value=provider)
  self.seams[stub]=lambda m:m.ret(value=empty+16)
  def transport(m):
   assert m.ecx()==m.data+0x2c0
   p=m.arg();index=m.r32(p+12);assert index==len(m.records)
   m.records.append({'index':index,'name':m.names[m.r32(p+8)],'group':m.r32(p+0x2c),'exclusive':bool(m.r8(p+0x30))})
   m.w32(m.array+index*4,p);m.w32(m.data+0x2e8,len(m.records));m.ret(4,p)
  self.seams[0x4b4e60]=transport
  self.call(0x477fc0,self.data)
  assert self.final_esp==h.STACK+4 and self.fpu_top()==0
  return self.records
 def run(self,c):
  self.w32(self.actor+0x100,c['bits']&0xffffffff,c['bits']>>32);self.w32(self.data+0x2e8,c['count'])
  for i in range(max(0,c['count'])):
   descriptor=self.alloc(0x10);self.w32(descriptor+12,i);self.w32(self.array+4*i,descriptor)
  def enter(m):
   assert m.ecx()==m.actor and m.arg(1)==0
   i=m.r32(m.arg()+12);m.trace.append(['enter',i,0])
   if c.get('stop')==i:raise Stop()
   for mutation in c.get('mutations',[]):
    if mutation['index']==i:
     bits=mutation['bits'];m.w32(m.actor+0x100,bits&0xffffffff,bits>>32)
     if 'count' in mutation:m.w32(m.data+0x2e8,mutation['count'])
   m.ret(8)
  def grid(m):
   assert m.ecx()==m.actor;m.trace.append(['grid'])
   if c.get('stop_grid'):raise Stop()
   m.ret()
  self.seams[0x4d4a90]=enter;self.seams[0x4cfea0]=grid
  result='complete'
  try:
   self.call(0x4d62f0,self.actor);assert self.final_esp==h.STACK+4 and self.fpu_top()==0
  except Stop:result='required_service'
  bits=self.r32(self.actor+0x100)|(self.r32(self.actor+0x104)<<32)
  return {'result':result,'trace':self.trace,'bits':bits,'stored_count':self.r32(self.data+0x2e8)}
def main():
 rows=[];tables=[];inputs=[]
 for count in [-2,0,1,2,31,32,33,42,63,64,65,80,255,256,257,258]:
  for bits in [0,1,1<<31,1<<32,1<<41,1<<63,(1<<64)-1]:inputs.append({'count':count,'bits':bits})
 inputs += [
 {'count':42,'bits':3,'mutations':[{'index':0,'bits':(1<<41)|1,'count':0}]},
 {'count':42,'bits':1,'mutations':[{'index':0,'bits':5,'count':99}]},
 {'count':42,'bits':7,'stop':1},
 {'count':42,'bits':3,'mutations':[{'index':0,'bits':9}],'stop':3},
 {'count':0,'bits':0,'stop_grid':True},
 {'count':42,'bits':0,'stop_grid':True},
 ]
 for image,label in [(P,'original'),(V,'v14')]:
  h.P=image
  for pc in h.PRECISION:
   m=Machine(pc);tables.append({'image':label,'precision':pc,'records':m.table()})
   for c in inputs:
    m=Machine(pc);rows.append({'image':label,'precision':pc,'input':c,'output':m.run(c)})
 h.P=P
 for a,b in h.VISITED.items():assert P.range(a,len(bytes.fromhex(b)))==V.range(a,len(bytes.fromhex(b)))==bytes.fromhex(b)
 for t in tables:assert t['records']==tables[0]['records']
 for i in range(len(inputs)):
  assert all(rows[i]['output']==rows[j*len(inputs)+i]['output'] for j in range(6))
 out={'schema':1,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'producer_sha256':H(Path(__file__).read_bytes()),'table_runs':tables,'case_count':len(rows),'cases':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())],
 'scope':'Full4D62F0 and477FC0/474270/473490 table constructor. Heap/CString/DataManager insertion transport explicit seams; count/descriptor/runtime actor bits actual native storage.4D4A90 and4CFEA0 are observed required callbacks, not their bodies. All inputs fixture domains; source/runtime binding separately tested. Counts beyond42 are boundary probes, not production manager support. CRT96D710 uses CL low8, shifts>=64 produce0.'}
 path=HERE/'native-registration.json';path.open('x').write(json.dumps(out,indent=1)+'\n');print(json.dumps({'rows':len(rows),'tables':len(tables),'pins':len(h.VISITED),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
