"""Execute native recruit-list and both virtual item loaders with I/O seams."""
from pathlib import Path
import sys,struct,json,hashlib,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-mapheader'))
from native_header import Machine as HeaderMachine,VISITED
from unicorn.x86_const import *

def word(v):return struct.pack('<I',v&0xffffffff)
def string(s):
 b=s.encode('utf-16-le');return struct.pack('<H',len(b)//2)+b
class Machine(HeaderMachine):
 def __init__(self,data):super().__init__(data);self.loaded=[];self.property=[]
 def hook(self,u,pc,size,ud):
  if pc==0x4c2e60:
   n=struct.unpack('<H',self.r.take(2))[0];b=self.r.take(n*2)
   self.property=list(struct.unpack('<'+'H'*n,b));self.write(self.arg(1),0)
   self.ret(value=self.arg());return
  if pc==0x4e6d10:
   item=u.reg_read(UC_X86_REG_ESI);kind=0 if self.read(item)==0xa29694 else 1
   self.loaded.append({'kind':kind,'property_id':self.property,'enabled':self.u.mem_read(item+8,1)[0],
     'words':[self.read(item+off) for off in ((12,16,20,24,28) if kind==0 else (12,))]})
  super().hook(u,pc,size,ud)
 def run(self):
  context=self.alloc(0x80);self.write(context+0x3c,self.read(self.stream));manager=self.alloc(0x40)
  self.write(self.stack,self.stop);self.write(self.stack+4,context)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,manager)
  self.u.emu_start(0x4e6c20,self.stop,count=2000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
  assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+8
  return {'consumed':self.r.at,'items':self.loaded,'word_24':self.read(manager+0x24)}
def item(kind,version,seed):
 rnd=random.Random(seed);values=[rnd.getrandbits(32) for _ in range(5)];name=['','USAInfantry','Recruit_Heavy','属性'][seed%4]
 data=word(kind)+word(0)+string(name)+bytes([seed%256])+word(version)
 if kind==1:return data+word(values[0])
 if version==0:return data+word(values[0])+word(values[2])+bytes([seed%251])+word(values[4])
 data+=word(values[0])+word(values[1])+word(values[2])+word(values[4])
 if version>=2:data+=word(values[3])
 return data
if __name__=='__main__':
 cases=[]
 for version in (0,1):
  for n in range(32):
   items=[item((i+n)%4==3,0 if (i+n)%4==3 else (i+n)%3,100*n+i) for i in range(n%7)]
   raw=word(version)+word(len(items))+b''.join(items)+(word(0xf0000000+n) if version else b'')
   result=Machine(raw).run();assert result['consumed']==len(raw)
   cases.append({'name':f'v{version}_mixed{n}','hex':raw.hex(),**result})
  for count in (0x80000000,0xffffffff):
   raw=word(version)+word(count)+(word(17) if version else b'');result=Machine(raw).run()
   cases.append({'name':f'v{version}_negative{count}','hex':raw.hex(),**result})
 out={'schema':1,'passed':True,'cases':cases,'visited_native_instructions_and_seam_entries':[{'va':hex(a),'bytes':b} for a,b in sorted(VISITED.items())],
      'scope':'Full4E6C20 list loop and original virtual item Loads. Primitive stream/version reads, allocation and property registry lookup are seams; lookup returnsnull. No runtime duplicate suppression/publication proof.'}
 (HERE/'native-recruitment.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'passed':True,'cases':len(cases),'visited':len(VISITED)}))
