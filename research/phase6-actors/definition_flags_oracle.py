"""Original complete base flag loop and bounded actor component-mask decisions."""
from pathlib import Path
import json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase5-actors'))
from original_creation import Machine as Base,P,V,H,bits
from definition_prerequisites import utf16
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
from unicorn.x86_const import *

FLAGS=[]
for i in range(40):
 bit,ptr=struct.unpack('<II',P.range(0xa11748+i*8,8))
 if not bit:break
 FLAGS.append({'bit':bit,'name':utf16(ptr)})

class Machine(Base):
 def hook(self,u,pc,n,z):
  if pc in getattr(self,'ends',[]):u.emu_stop();return
  if pc==0x47d960:self.ret(pop=4)
  elif pc==0x5b4410:self.ret(0x2256000,pop=8)
  elif pc==0x5b5380:self.ret(int(self.values[self.text(self.arg())]),pop=4)
  elif pc==0x301f380:self.ret(int(self.is_actor))
  elif pc==0x404ed0:self.ret()
  else:super().hook(u,pc,n,z)
 def flags(self,initial,values,is_actor):
  self.values=values;self.is_actor=is_actor;self.w(self.definition,0x2256100);self.w(0x2256138,0x301f380);self.w(self.definition+0x1c,initial)
  self.invoke(0x4bb310,self.definition,[0x2256000]);return self.r(self.definition+0x1c)

def main():
 m=Machine(0x27f);flagcases=[]
 for initial in [0,0xffffffff,0x12345678]:
  for actor in [False,True]:
   for selected in [None,'all']+[v['name'] for v in FLAGS]:
    values={v['name']:selected=='all' or selected==v['name'] for v in FLAGS}
    flagcases.append({'initial':initial,'is_actor':actor,'values':values,'result':m.flags(initial,values,actor)})
 ins=list(Cs(CS_ARCH_X86,CS_MODE_32).disasm(P.range(0x464bd0,16628),0x464bd0));components=[]
 for k,i in enumerate(ins):
  if i.mnemonic!='or' or len(i.op_str.split(', '))!=2:continue
  dst,val=i.op_str.split(', ')
  try:mask=int(val,0)
  except ValueError:continue
  if mask<=0 or mask&(mask-1):continue
  mem='[ebp + 0x174]' in dst
  prior=any(t.mnemonic=='mov' and t.op_str==dst+', dword ptr [ebp + 0x174]' for t in ins[max(0,k-2):k])
  if not(mem or prior):continue
  for j in range(k-1,max(0,k-20),-1):
   if ins[j].mnemonic=='call' and ins[j].op_str=='0x5b4410':break
  else:continue
  name=None
  for t in ins[max(0,j-5):j]:
   if t.mnemonic=='push' and t.op_str.startswith('0x'):
    try:s=utf16(int(t.op_str,16))
    except Exception:continue
    if s.endswith('Component'):name=s
  assert name
  end=i.address+i.size
  if not mem:
   for t in ins[k+1:k+8]:
    if t.mnemonic=='mov' and t.op_str.startswith('dword ptr [ebp + 0x174],'):
     end=t.address+t.size;break
   else:raise AssertionError(name)
  start=ins[j+1].address
  absent=next(int(t.op_str,16) for t in ins[j+1:k] if t.mnemonic=='je')
  rows=[]
  for present in [False,True]:
   initial=0x80000000 | ((1<<13) if name=='ResearchComponent' else 0)
   m.ends=[end,absent];m.w(m.definition+0x174,initial);m.w(m.stack,0)
   for r in [UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_ECX,UC_X86_REG_EDX]:m.u.reg_write(r,0)
   m.u.reg_write(UC_X86_REG_EBP,m.definition);m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_EAX,0x2256000 if present else 0)
   m.u.emu_start(start,0,count=1000);assert m.u.reg_read(UC_X86_REG_EIP) in m.ends
   rows.append({'present':present,'initial':initial,'result':m.r(m.definition+0x174)})
  components.append({'name':name,'bit':mask,'start':hex(start),'store_end':hex(end),'absent':hex(absent),'cases':rows})
 assert len(components)==30
 m.ends=[];filters=[]
 for mask in [0,32,32|(1<<20),32|(1<<23),0x7fffffff]:
  for count in [0,1,9]:
   m.w(m.definition+0x174,mask);m.w(m.definition+0x1f0,count);m.u.reg_write(UC_X86_REG_EBP,m.definition)
   m.u.emu_start(0x46886c,0x46889e,count=1000)
   filters.append({'mask':mask,'ability_count':count,'result':m.r(m.definition+0x174)})
 final_flags=[]
 m.w(m.definition,0x2256100);m.w(0x225613c,0x4c9fa0);m.w(m.definition+0x18,0x2256200)
 for mask in [0,1,2,8,128,16384,9,136,0x7fffffff]:
  for speed in [-1.0,0.0,2.5]:
   for category in [19,31,32]:
    for flags in [0x20000000,0xffffffff]:
     m.w(m.definition+0x174,mask);m.w(m.definition+0x228,bits(speed));m.w(0x225620c,category);m.w(m.definition+0x1c,flags)
     m.u.reg_write(UC_X86_REG_EBP,m.definition);m.u.reg_write(UC_X86_REG_ESP,m.stack)
     m.u.emu_start(0x468995,0x468a43,count=1000)
     final_flags.append({'initial_flags':flags,'mask':mask,'speed_bits':bits(speed),'category_index':category,'result':m.r(m.definition+0x1c)})
 windows=[]
 for a,z in [(0x4bb310,0x4bb3f0),(0xa11748,0xa11748+19*8),(0x464bd0,0x468cc4),(0x4bbe70,0x4bbef4)]:
  raw=P.range(a,z-a);assert raw==V.range(a,z-a);windows.append({'start':hex(a),'end':hex(z),'sha256':H(raw)})
 out={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'base_flag_table':FLAGS,'flag_cases':flagcases,'component_branches':components,'performer_filters':filters,'final_actor_flags':final_flags,'windows':windows,'seams':['Full4BB310 flag loop runs, with typed child/boolean reads and IsActor provided; common data loader and logger are leaves.','Component branch fixtures enter immediately after resolved child lookup and stop at mask store or absent target. They do not execute component data loaders.','Performer filter executes original46886C..46889E with explicit completed ability count.','Actorfinalflags execute original468995..468A43 with native standardActorData virtual3C=4C9FA0; diagnosticsink only.','Source-derived node presence and template resolution have separate grammar evidence; full original data loader is not executed.']}
 p=HERE/'definition-flags-native.json';p.write_text(json.dumps(out,indent=2)+'\n');print(H(p.read_bytes()))
if __name__=='__main__':main()
