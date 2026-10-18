"""Full native773CE0 status sequencing and numeric spills. CString/translation,
mutex, timer and diagnostic are explicit child transports; no logger formatting,
OS timer producer, TextManager fallback or allocator parity is claimed.
"""
from pathlib import Path
import sys,json,itertools,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_WRITE
H=lambda b:hashlib.sha256(b).hexdigest();ORIGINAL=h.P;V14=h.V
SEAMS=[0x4b9f40,0x405b80,0x4012c0,0x856400,0x59aba0,0x404f60]
class Machine(h.Machine):
 def run(self,i):
  obj=self.alloc(0x180);vt=self.alloc(16);lock=h.STUBS+0x10000;unlock=lock+16;self.w32(vt+4,lock,unlock);self.w32(obj+0x12c,vt);self.w32(obj+0x14c,77)
  def string(text):
   data=self.alloc(16+2*(len(text)+1));self.w32(data,0,0,len(text),2);self.u.mem_write(data+16,(text+'\0').encode('utf-16le'));p=self.alloc(4);self.w32(p,data+16);return p
  old=string('old');raw=string('raw');translated=string('localized');self.w32(obj+0x148,self.r32(old));self.w32(0xb36544,self.alloc(0x80))
  trace=[];cutoff=None
  def stop(name):
   nonlocal cutoff
   if i['fail']==name:cutoff=name;self.u.emu_stop();return True
   return False
  def locking(m):trace.append('lock');m.ret()
  def unlocking(m):trace.append('unlock');m.ret()
  def translate(m):trace.append('translate');m.ret(8,translated)
  def cow(m):m.ret(0,m.arg(0))
  def display(m):trace.append('display_copy');m.ret(0,raw)
  def store(m):
   ptr=m.r32(m.arg(0));assert m.ecx()==obj+0x148;trace.append(['store148',3 if ptr==m.r32(translated)else 2]);m.w32(m.ecx(),ptr);m.ret(4,m.ecx())
  def timer(m):
   assert m.ecx()==0xb396e0;trace.append('timer')
   if not stop('timer'):m.ret(0,i['milliseconds'])
  def diagnostic(m):
   bits64=m.arg(5)|(m.arg(6)<<32);f=struct.unpack('<d',struct.pack('<Q',bits64))[0];bits=h.f2b(f);trace.append(['diagnostic',bits])
   if not stop('diagnostic'):m.ret()
  def write(u,access,a,n,v,_):
   if a==obj+0x14c:trace.append(['store14c',v])
  self.u.hook_add(UC_HOOK_MEM_WRITE,write)
  self.seams.update({lock:locking,unlock:unlocking,0x4b9f40:translate,0x4012c0:cow,0x856400:display,0x405b80:store,0x59aba0:timer,0x404f60:diagnostic})
  try:self.call(0x773ce0,obj,[raw,i['word'],i['translate']])
  except AssertionError:assert cutoff is not None
  text=3 if self.r32(obj+0x148)==self.r32(translated)else 2 if self.r32(obj+0x148)==self.r32(raw)else 1
  return dict(text=text,word=self.r32(obj+0x14c),trace=trace,error=cutoff)
def main():
 h.VISITED.clear();rows=[]
 for pc,flag,ms,fail in itertools.product(h.PRECISION,[0,1,128],[0,1,0x7fffffff,0x80000000,0xffffffff],['none','timer','diagnostic']):
  i=dict(translate=flag,milliseconds=ms,word=0x12345678,fail=fail);h.P=ORIGINAL;a=Machine(pc).run(i);h.P=V14;b=Machine(pc).run(i);assert a==b;rows.append(dict(pc=pc,input=i,output=a))
 h.P=ORIGINAL;pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in SEAMS}
 for a,v in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(v)))==V14.range(int(a,16),len(bytes.fromhex(v)))
 data=(json.dumps(dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),rows=rows,pins=pins,seams=[hex(a)for a in SEAMS]),indent=1)+'\n').encode();p=HERE/'native-session-status.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),native_runs=2*len(rows),executed_pins=len(pins),sha256=H(data))))
if __name__=='__main__':main()

