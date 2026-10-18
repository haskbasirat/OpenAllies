"""Original4C0A80 load/write/validation order; TGI/glyph services are fixtures.

4C0A80 assigns display color, glyph and animation filter. 410C40 throws the
original loading exception; rejected cases stop at that call without simulating
Windows exception unwinding. No armor/damage equation is inferred from this loader.
"""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base, original, modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
bits=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]

class Machine(Base):
 DATA=0x2002000
 NODE=0x2003000
 STRING=0x2004000
 def __init__(self,c):
  super().__init__();self.c=c;self.events=[];self.rejected=None;self.executed={}
  self.u.mem_map(0,4096)
  self.u.mem_write(self.DATA+0x18,struct.pack('<6I',*c['initial']['color'],c['initial']['glyph'],c['initial']['animation_filter']&0xffffffff))
  self.w(self.STRING+12,100);self.w(self.STRING+32,self.STRING+16)
  self.w(0xb36530,0x2005000);self.w(0x2005000+0x3bc,0x2005100)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+4*i)
 def ret(self,value=0,pop=0):
  stack=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value)
  self.u.reg_write(UC_X86_REG_EIP,self.r(stack));self.u.reg_write(UC_X86_REG_ESP,stack+4+pop)
 def hook(self,u,pc,size,_):
  if 0x400000<=pc<0xc00000:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.executed[pc]=raw
  if pc==0x47d960:self.events.append('base');self.ret(pop=4)
  elif pc==0x5b4c30:
   self.events.append('color');self.u.mem_write(self.arg(),struct.pack('<3I',*self.c['color']));self.ret(self.arg(),8)
  elif pc==0x5b54d0:self.events.append('glyph');self.ret(self.STRING+32,4)
  elif pc==0x4012c0:self.ret(self.STRING)
  elif pc==0x48d500:self.ret(self.c['glyph'],4)
  elif pc==0x5b4ed0:self.events.append('animation_filter');self.ret(self.c['animation_filter']&0xffffffff,4)
  elif pc==0x410c40:
   self.rejected='unknown damage-type glyph' if self.c['glyph']==0 else 'negative damage animation filter';u.emu_stop()
 def run(self):
  self.reset(0);self.w(self.STACK+4,self.NODE);self.u.reg_write(UC_X86_REG_ECX,self.DATA)
  regs={register:0x22ee0000+i for i,register in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
  for register,value in regs.items():self.u.reg_write(register,value)
  self.u.emu_start(0x4c0a80,self.STOP,count=30000)
  if self.rejected is None:
   assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
   for register,value in regs.items():assert self.u.reg_read(register)==value
  values=struct.unpack('<5Ii',self.u.mem_read(self.DATA+0x18,24))
  return {'state':{'color':list(values[:4]),'glyph':values[4],'animation_filter':values[5]},'events':self.events,'error':self.rejected}

def main():
 initial={'color':[1,2,3,4],'glyph':777,'animation_filter':88}
 cases=[];visited={}
 for name,rgb,glyph,filter in [('gunfire',[1,0,0],1,0),('flame',[1,.5,0],2,1),('explosive',[1,1,0],3,2),('armor_piercing',[0,0,1],4,2),
      ('missing_glyph',[.3,.4,.5],0,2),('negative_filter',[1,1,1],5,-1),('large_filter',[1,1,1],6,0x7fffffff),('minimum_filter',[1,1,1],1,-0x80000000)]:
  c={'name':name,'initial':initial,'color':list(map(bits,rgb)),'glyph':glyph,'animation_filter':filter}
  m=Machine(c);cases.append({'input':c,'output':m.run()});visited.update(m.executed)
 sources={}
 for path in ['Game/damage_types.tgi','Templates/template_damage_type.tgi']:
  p=ROOT.parent/'reconstruction/assets/original/Data'/path;sources[path]=hashlib.sha256(p.read_bytes()).hexdigest()
 report={'schema':1,'passed':True,'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'oracle_dependencies':{'research/phase4-rng/original_rng.py':hashlib.sha256((ROOT/'research/phase4-rng/original_rng.py').read_bytes()).hexdigest()},
  'method':__doc__,'cases':cases,'authored_sources':sources,
  'visited_windows':[{'address':hex(pc),'bytes':raw.hex()} for pc,raw in sorted(visited.items())],
  'scope':'Four original authored RGB/filter combinations with fixture glyph handles, plus failed/edge validation; base typed loading and glyph registry are required external services.'}
 (HERE/'native-damage-type-fixtures.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
 print(json.dumps({'cases':len(cases),'sha256':hashlib.sha256((HERE/'native-damage-type-fixtures.json').read_bytes()).hexdigest()}))
if __name__=='__main__':main()
