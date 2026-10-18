"""Original Kingdom leaf readers with synthetic variable-count/versioned payloads."""
from pathlib import Path
import sys,struct,json,hashlib,random
HERE=Path(__file__).parent
sys.path.insert(0,str(HERE))
from native_kingdoms import Machine,P,V
from unicorn.x86_const import *
W=lambda *v:struct.pack('<'+'I'*len(v),*v)
S=lambda s:struct.pack('<H',len(s))+s.encode('utf-16-le')
def run(address,payload):
 m=Machine(payload+b'FOLLOWING',28);m.write(m.stack,m.stop);m.write(m.stack+4,m.world);m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_ECX,m.object)
 m.u.emu_start(address,m.stop,count=1000000);assert m.u.reg_read(UC_X86_REG_EIP)==m.stop;assert m.r.at==len(payload),(hex(address),m.r.at,len(payload))
 return {'consumed':m.r.at,'words':{hex(o):m.read(m.object+o) for o in (4,8,12,16,20)},'reads':m.reads}
def main():
 cases=[];rng=random.Random(0x4e8550)
 for version in range(4):
  for sample in range(3):
   a,b,c=rng.getrandbits(32),rng.getrandbits(32),rng.getrandbits(32);key='fixture_property'+str(sample);byte=[0,1,255][sample]
   payload=W(version,a)+(W(b) if version else b'')+S(key)+(bytes([byte])+W(c) if version>=2 else b'')
   native=run(0x503b10,payload);assert native['words']['0x4']==a and native['words']['0x8']==(b if version else 0xffffffff)
   if version>=2:assert native['words']['0x14']==c
   cases.append({'kind':'property','payload':payload.hex(),'native':native,'expected':{'version':version,'word_04':a,'word_08':b if version else None,'reference_utf16':[ord(c) for c in key],'byte_10':byte if version>=2 else None,'word_14':c if version>=2 else None}})
 for version in range(2):
  for sample in range(3):
   values=[rng.getrandbits(32) for _ in range(3)];words=[rng.getrandbits(32) for _ in range(sample)]
   payload=W(version,*values)+bytes([sample])+(W(len(words),*words) if version else b'')
   native=run(0x4c8b40,payload);assert [native['words'][hex(o)] for o in (12,16,20)]==values
   cases.append({'kind':'ability','payload':payload.hex(),'native':native,'expected':{'version':version,'word_0c':values[0],'word_10':values[1],'word_14':values[2],'byte_18':sample,'words':{'serialized_count':len(words),'values':words} if version else None}})
 for version in range(2):
  words=[rng.getrandbits(32) for _ in range(60 if version==0 else 66)];payload=W(version,*words)
  cases.append({'kind':'strategic','payload':payload.hex(),'native':run(0x58ee80,payload),'expected':{'version':version,'words':words}})
 for version in range(3):
  payload=W(version);definition_groups=[];handle_groups=[]
  for i in range(4+(version>1)):
   values=[{'reference_utf16':[ord(c) for c in 'unit_fixture'],'word':rng.getrandbits(32)}] if i%2 else []
   payload+=W(len(values))+b''.join(S('unit_fixture')+W(v['word']) for v in values);definition_groups.append({'serialized_count':len(values),'values':values})
  if version:
   for i in range(4+(version>1)):
    values=[[7,rng.getrandbits(32)]] if i%2 else [];payload+=W(len(values))+b''.join(W(*v) for v in values);handle_groups.append({'serialized_count':len(values),'values':values})
  cases.append({'kind':'tactical','payload':payload.hex(),'native':run(0x58dbe0,payload),'expected':{'version':version,'definition_groups':definition_groups,'handle_groups':handle_groups}})
 ranges={}
 for name,a,n in [('kingdom',0x4e8550,0xe80),('commander',0x4ce0b0,0x113),('ability',0x4c8b40,0x10c),('property',0x503b10,0xa3),('tactical',0x58dbe0,0x2be),('strategic',0x58ee80,0xdd),('strategic_entry',0x58f2e0,0x71),('world_kingdoms',0x4fcc44,0x1c1)]:
  raw=P.range(a,n);assert raw==V.range(a,n);ranges[name]={'va':hex(a),'bytes':n,'sha256':hashlib.sha256(raw).hexdigest()}
 out={'schema':1,'cases':cases,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'ranges':ranges}
 p=HERE/'native-leaves.json';p.write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'cases':len(cases),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
