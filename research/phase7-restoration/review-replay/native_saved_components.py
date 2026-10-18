"""Original saved component readers, with resolved getter/context seams only."""
from pathlib import Path
import sys,struct,json,hashlib,random
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'research/phase6-kingdoms'))
from native_kingdoms import Machine as Base,P,V,VISITED
from unicorn.x86_const import *
W=lambda *v:struct.pack('<'+'I'*len(v),*v)
B=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
class Machine(Base):
 def __init__(self,payload,pc=0x27f):
  super().__init__(payload,28);self.u.reg_write(UC_X86_REG_FPCW,pc)
  self.write(self.object+0x14,B(200));self.write(self.object+0x1c,B(-1));self.write(self.object+0x20,0xc479c000);self.write(self.object+0x24,B(0));self.write(self.object+0x28,B(2.5))
  self.u.mem_write(0x2e00100,b'\xd9\x05'+W(0x2e00110)+b'\xc2\x04\x00');self.write(0x2e00110,B(2.5))
 def hook(self,u,pc,size,arg):
  if pc==0x4f0480:self.ret(value=1);return
  if pc==0x4d0720:self.ret(value=self.view);return
  if pc==0x42a590:self.ret(8);return
  if pc==0x45d6f0:u.reg_write(UC_X86_REG_EIP,0x2e00100);return
  if pc==0x4db0f0:self.ret(value=1);return
  super().hook(u,pc,size,arg)
 def run(self,address):
  if address==0x570fa0:self.u.mem_write(self.object+0x1c,b'\x01');self.write(self.object+0x24,B(2))
  if address in (0x56c320,0x58a770,0x58abd0):
   for off in range(0x10,0x8c,4):self.write(self.object+off,0)
   owner=self.alloc(0x200);definition=self.alloc(0x800);self.write(owner+4,definition);self.write(owner+0x14,self.empty);self.write(definition+8,self.empty);self.write(self.object+4,owner)
   self.write(definition+0x424,1);self.write(definition+0x428,B(50))
  if address==0x57f970:
   owner=self.alloc(0x200);definition=self.alloc(0x800);self.write(owner+4,definition);self.write(owner+0x14,self.empty);self.write(definition+8,self.empty);self.write(self.object+4,owner)
   self.write(self.object+0x18,0);self.write(self.object+0x24,B(50));self.write(self.object+0x50,2);table=self.alloc(8);self.write(self.object+0x4c,table)
   for i in range(2):self.write(table+i*4,self.alloc(28))
  if address==0x566bc0:
   self.write(self.object+0x10,self.alloc(64));self.write(self.object+0x14,16);self.write(self.object+0x1c,self.alloc(64));self.write(self.object+0x20,16)
  if address==0x563930:
   self.write(self.object+0x10,0xa28900);self.view=self.alloc(0x100);self.animation=self.alloc(0x80)
   for kind in range(3):
    table=self.alloc(4);self.write(table,self.animation);self.write(self.view+kind*12+0xc,table);self.write(self.view+kind*12+0x10,1)
  self.write(self.stack,self.stop);self.write(self.stack+4,self.world);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.object)
  self.u.emu_start(address,self.stop,count=100000);assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
  return {'consumed':self.r.at,'owner_flags':self.read(self.read(self.object+4)+8),'words':{hex(o):self.read(self.object+o) for o in range(0x10,0x8c,4)},'bytes':{hex(o):self.u.mem_read(self.object+o,1)[0] for o in (0x4c,0x54,0x56,0x5c,0x6c)},'reads':self.reads}
def main():
 rng=random.Random(0x5618c0);cases=[]
 def execute(slot,payload,address,expected,pc='Double53'):
  cw={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[pc];m=Machine(payload+b'FOLLOWING',cw);native=m.run(address);assert native['consumed']==len(payload),(slot,native['consumed'],len(payload));cases.append({'slot':slot,'precision':pc,'payload':payload.hex(),'expected':expected,'native':native})
 for pc in ['Single24','Double53','Extended64']:
  for version in range(5):
   for sample,dying in enumerate([0,0x80000000,B(-1),B(2),0x7fc01234]):
    up=W(1,0) if version>=2 else b'';health=B(rng.random());sink=rng.getrandbits(32);last=rng.getrandbits(32)
    payload=W(version)+up+(W(dying) if version else b'')+W(health)+(W(sink) if version>=3 else b'')+(W(last) if version>=4 else b'')
    execute(0,payload,0x5618c0,{'Body':{'version':version,'updatable':{'version':1,'serialized_id':0} if up else None,'dying_started_bits':dying if version else None,'health_multiplier_bits':health,'sink_duration_bits':sink if version>=3 else None,'last_heal_time_bits':last if version>=4 else None}},pc)
 for version in range(9):
  p=W(1,0,version);e={'updatable':{'version':1,'serialized_id':0},'version':version}
  def words(name,n,condition=True):
   nonlocal p
   values=[rng.getrandbits(32) for _ in range(n)] if condition else None;e[name]=values if n!=1 or values is None else values[0]
   if values:p+=W(*values)
  def byte(name,condition):
   nonlocal p
   value=1 if condition else None;e[name]=value
   if value is not None:p+=bytes([value])
  words('vector_10',3);words('word_20',1);words('word_24',1)
  e['speed_multiplier_bits']=B(.75) if version>=8 else None
  if version>=8:p+=W(B(.75))
  words('legacy_vector',3,version<4);words('word_44',1);words('word_48',1);byte('byte_4c',1<=version<7);byte('byte_54',version>=2);words('vector_38',3,version>=3);words('word_50',1,version>=3);byte('byte_56',version>=5);words('word_58',1,version>=5);byte('byte_5c',version>=6);words('vector_60',3,version>=6);byte('byte_6c',version>=6);words('vector_70',3,version>=6);words('vector_7c',3,version>=6)
  execute(3,p,0x576e90,{'Mover':e})
 for upversion in (0,1):
  up=W(upversion)+(W(0) if upversion else b'');expected={'version':upversion,'serialized_id':0 if upversion else None}
  for slot in [9,18,22,28]:execute(slot,up+W(0),0x562ed0,{'UpdatableOnly':{'updatable':expected,'version':0}})
 for version in range(4):
  up=W(1,0) if version else b'';legacy=b'\xff' if version<2 else b'';count=W(0) if version>=3 else b''
  execute(8,W(version)+up+legacy+count,0x570fa0,{'Element':{'version':version,'updatable':{'version':1,'serialized_id':0} if up else None,'legacy_byte':255 if legacy else None,'history_count':0 if count else None}})
 execute(8,W(3,1,0,0xffffffff),0x570fa0,{'Element':{'version':3,'updatable':{'version':1,'serialized_id':0},'legacy_byte':None,'history_count':0xffffffff}})
 execute(17,W(0,0x3f800000),0x571780,{'Experience':{'version':0,'word_08':0x3f800000}})
 for version in (0,1,3):
  p=W(1,0,version,0xdeadbeef)+(W(0x12345678) if version else b'');execute(19,p,0x575890,{'MobileDeployment':{'updatable':{'version':1,'serialized_id':0},'version':version,'word_10':0xdeadbeef,'word_14':0x12345678 if version else None}})
 for version in range(3):
  for kind in range(3):
   values=[rng.getrandbits(32) for _ in range(4)];active=255 if kind else 0
   p=W(1,0,version,values[0])+(W(1234) if version==0 else b'')+W(kind,0,0,*values[1:])+bytes([active])+(W(7) if version>=2 else b'')
   execute(2,p,0x563930,{'Character':{'updatable':{'version':1,'serialized_id':0},'version':version,'word_28':values[0],'legacy_word':1234 if version==0 else None,'animation_kind':kind,'animation_index':0,'animation_version':0,'elapsed_bits':values[1],'resumed_at_bits':values[2],'rate_bits':values[3],'active_byte':active,'variant':7 if version>=2 else None}})
 for version in range(2):
  for count in range(4):
   a=[B(rng.random()) for _ in range(count)];b=[B(rng.random()) for _ in range(3-count)];p=W(1,0,version,len(a),*a)+(W(len(b),*b)+struct.pack('<H',7)+'upgrade'.encode('utf16')[2:] if version else W(0,0))
   execute(15,p,0x566bc0,{'Construction':{'updatable':{'version':1,'serialized_id':0},'version':version,'words_10':{'serialized_count':len(a),'values':a},'words_1c':{'serialized_count':len(b),'values':b} if version else None,'reference_28_utf16':[ord(c) for c in 'upgrade'] if version else None,'legacy_upgrade_words':[0,0] if version==0 else None}})
 for version in range(5,10):
  for sample in range(4):
   up={'version':1,'serialized_id':0};vals=[rng.getrandbits(32)for _ in range(7)];mult=B(.75)
   p=W(1,0,version,vals[0],mult,vals[1],vals[2],vals[3],0xffffffff,vals[4],vals[5],sample);states=[]
   for i in range(sample):
    v=[rng.getrandbits(32)for _ in range(6)];present=255 if i%2 else 0
    p+=W(*v[:2])+(W(v[2]) if version>=7 else b'')+(bytes([present])+W(v[3],v[4]) if version>=8 else b'')+(W(v[5]) if version>=9 else b'')
    states.append({'words_00_04':v[:2],'word_08':v[2] if version>=7 else None,'present_byte':present if version>=8 else None,'words_10_14':v[3:5] if version>=8 else None,'word_18':v[5] if version>=9 else None})
   p+=W(vals[6])+(W(12345) if version>=6 else b'')+W(1)
   av=sample%2;abilities=[{'version':av,'word_0c':1,'word_10':2,'word_14':3,'byte_18':1,'words':{'serialized_count':2,'values':[4,5]} if av else None}]
   p+=W(av,1,2,3)+b'\x01'+(W(2,4,5) if av else b'')+(b'\xff' if version>=7 else b'')
   execute(5,p,0x57f970,{'Performer':{'updatable':up,'version':version,'word_10':vals[0],'multiplier_20':mult,'word_28':vals[1],'words_2c_30':vals[2:4],'ability_index':-1,'words_3c_40':vals[4:6],'states':{'serialized_count':sample,'values':states},'word_58':vals[6],'word_44':12345 if version>=6 else None,'abilities':{'serialized_count':1,'values':abilities},'byte_5c':255 if version>=7 else None}})
 for version in range(3):
  p=W(version)+(W(1,0)if version else b'')+(W(B(.5),1234)if version>=2 else b'')
  execute(11,p,0x58abd0,{'Supply':{'version':version,'updatable':{'version':1,'serialized_id':0}if version else None,'amount_multiplier_bits':B(.5)if version>=2 else None,'word_1c':1234 if version>=2 else None}})
 for version in range(5):
  for count in range(3):
   pairs=[[rng.getrandbits(32),rng.getrandbits(32)]for _ in range(count)];handles=[rng.getrandbits(32)for _ in range(count)]
   p=W(1,0,version)+(b'\x02'if version>=1 else b'')+(b'\xff'if version>=2 else b'')+(W(count,*[v for pair in pairs for v in pair])if version>=3 else b'')+(W(9876)if version>=4 else b'')+(W(count,*handles,0)if version>=3 else b'')
   execute(10,p,0x56c320,{'Denizen':{'updatable':{'version':1,'serialized_id':0},'version':version,'byte_20':2 if version>=1 else None,'byte_21':255 if version>=2 else None,'occupants':{'serialized_count':count,'values':pairs}if version>=3 else None,'word_34':9876 if version>=4 else None,'handles':{'serialized_count':count,'values':handles}if version>=3 else None,'command_count':0 if version>=3 else None}})
 for version in range(14):
  p=W(version)+(W(1,0)if version>=6 else b'')+W(0)+(W(0)if version==0 else b'')+(W(0,12,13)if version>=2 else b'')+(W(14)if version>=11 else b'')+(W(15)if version>=12 else b'')+(W(16)if version>=7 else b'')
  execute(13,p,0x58a770,{'Structure':{'version':version,'updatable':{'version':1,'serialized_id':0}if version>=6 else None,'discarded_object_handle':0,'legacy_reference_count':0 if version==0 else None,'queue_count':0 if version>=2 else None,'word_20':12 if version>=2 else None,'word_2c':13 if version>=2 else None,'word_24':14 if version>=11 else None,'word_28':15 if version>=12 else None,'word_34':16 if version>=7 else None}})
 out={'schema':1,'scope':__doc__,'cases':cases,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())]};path=Path(__file__).with_name('native-saved-components.json');path.write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'cases':len(cases),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))

_original_run = Machine.run
_abi_calls = 0
def _review_run(self,*args,**kwargs):
 global _abi_calls
 regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
 for i,r in enumerate(regs): self.u.reg_write(r,0x11111111*(i+1))
 result=_original_run(self,*args,**kwargs)
 assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+8
 assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
 _abi_calls+=1
 return result
Machine.run=_review_run

if __name__=='__main__':
 main()
 print({'independent_abi_calls':_abi_calls})
