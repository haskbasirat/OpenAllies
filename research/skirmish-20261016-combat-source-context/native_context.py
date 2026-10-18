"""Original/v14 typed Settings source overwrites, source-local numerical context.

Full fixed assignment5B9E20/native CRT, typed field insertion, Settings4B3C10,
registered float2 ctor401710/Add4B3470, setter4B2F80/40E0A0 execute. String
assignment5BB0B0 plus source-created String overwrite also execute. Allocators,
string pool, logger and source-name lookup are declared leaves. No user profile
is discovered; supplied ordered override documents are laboratory inputs.
"""
from pathlib import Path
import json,struct,sys,hashlib
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'research/phase7-definitions'))
from probe_lists import Machine as Base
from original_creation import P,V
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
sha=lambda raw:hashlib.sha256(raw).hexdigest()
PCS={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
LEAF_ENTRIES={0x6f7810,0x6f7430,0x856370,0x85c680,0x9738a9,0x42bc40,0x404f60,0x5b3b10}
SPECS=[('CombatDamageMinimum','1'),('CombatValueRangePower','0.15'),('CombatValueAreaPower','1'),('CombatValueOffenseWeight','0.5'),('CombatValueDefenseWeight','0.5'),('CombatValueMovementWeight','0.2'),('CombatValueMovementDefault','1'),('CombatValueMeanValuesDiscardPercent','0.2'),('CombatValueRangedMinimum','12'),('CombatValueDefenseWeightRanged','0.25')]
class Machine(Base):
 def __init__(self,pc,image):
  super().__init__();self.u.reg_write(UC_X86_REG_FPCW,pc)
  old=(P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
  if new>old:self.u.mem_map(P.base+old,new-old)
  for _,rva,size,off in image.sections:self.u.mem_write(image.base+rva,image.b[off:off+size])
  self.w(0x2255064,self.r(0xb35444));self.seam_entries=set()
  self.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,self.null_guard,begin=0,end=0xfff)
 def null_guard(self,u,access,address,size,value,_):
  assert address+size<=4
  raw=bytes(u.mem_read(u.reg_read(UC_X86_REG_EIP),8));fs=False
  for byte in raw:
   if byte==0x64:fs=True;break
   if byte not in [0x26,0x2e,0x36,0x3e,0x65,0x66,0x67,0xf0,0xf2,0xf3]:break
  assert fs,f'unguarded null access {u.reg_read(UC_X86_REG_EIP):x}'
 def hook(self,u,pc,n,z):
  if pc==0x404f60:self.seam_entries.add(pc);self.ret()
  elif pc in [0x404ed0,0x404ea0]:raise RuntimeError(f'unexpected native warning/error {pc:x}')
  elif pc==0x5b3b10:
   self.seam_entries.add(pc);out=self.arg();self.w(out,self.string(list(map(ord,'CV-source-fixture'))));self.ret(out,pop=4)
  else:super().hook(u,pc,n,z)
 def make(self,name,raw,kind):
  node=self.node('Vars' if kind==6 else 'SVars');parser=self.alloc(0x80);token=self.alloc(0x30)
  self.w(parser,node);self.w(parser+0x40,self.string(list(map(ord,name))));self.w(parser+0x38,kind)
  self.w(token+0x18,self.string(list(map(ord,raw))));self.w(token+0x10,7)
  self.invoke(0x5b9e20 if kind==4 else 0x5bb0b0,parser,[token]);return node
 def settings(self):
  settings=self.alloc(0x80);self.invoke(0x4b2ce0,settings,[]);return settings
 def variable(self,settings):
  assert self.r(settings+12)==1
  return self.r(self.r(settings+8))
def main():
 rows=[];strings=[];pins={}
 for image_name,image in [('original',P),('v14',V)]:
  for precision,cw in PCS.items():
   for name,authored in SPECS:
    for sequence in [[authored],[authored,'2.25','-1.5'],['-0','0.1%','16777217']]:
     m=Machine(cw,image);settings=m.settings();var=m.alloc(0x2c)
     # Flag0 registered variable; constructor default is a diagnostic sentinel,
     # NEVER substituted into the production source binding.
     m.invoke(0x401710,var,[m.string(list(map(ord,name))),0x3dcccccd,0]);m.invoke(0x4b3470,settings,[0,var]);identity=m.variable(settings);states=[]
     for raw in sequence:
      m.invoke(0x4b3c10,settings,[m.make(name,raw,4)]);v=m.variable(settings)
      states.append({'raw':raw,'bits':m.r(v),'kind':m.r(v+0x24),'flags':m.r(v+0x28),'same_identity':v==identity,'control_word':m.u.reg_read(UC_X86_REG_FPCW)})
     assert all(s['kind']==2 and s['flags']==0 and s['same_identity'] and s['control_word']==cw for s in states)
     rows.append({'image':image_name,'precision':precision,'name':name,'states':states})
     for a,n in m.visited.items():
      if P.base<=a<P.base+P.image_size and a not in m.seam_entries and a not in LEAF_ENTRIES:
       raw=P.range(a,n);assert raw==V.range(a,n);pins[a]=raw
   for sequence,modes in [(['null','NULL'],[0,0]),(['null','cv.csv'],[0,0]),(['cv.csv','null'],[0,0]),(['first.csv','second.csv','NULL'],[0,0,0]),(['null','cv.csv'],[0,3]),(['cv.csv','null'],[0,3])]:
    m=Machine(cw,image);settings=m.settings();states=[];identity=None
    for raw,load_flags in zip(sequence,modes):
     m.w(settings+0x24,load_flags&1)
     m.invoke(0x4b3c10,settings,[m.make('CombatValueCSV',raw,6)]);v=m.variable(settings)
     if identity is None:identity=v
     text=m.r(v+0x10);length=m.r(text-12);units=list(struct.unpack('<%dH'%length,m.u.mem_read(text,length*2)))
     states.append({'raw':raw,'load_flags':load_flags,'value_utf16':units,'kind':m.r(v+0x24),'flags':m.r(v+0x28),'same_identity':v==identity,'control_word':m.u.reg_read(UC_X86_REG_FPCW)})
    assert all(s['kind']==4 and s['flags']==0 and s['same_identity'] and s['control_word']==cw for s in states)
    strings.append({'image':image_name,'precision':precision,'states':states})
    for a,n in m.visited.items():
     if P.base<=a<P.base+P.image_size and a not in m.seam_entries and a not in LEAF_ENTRIES:
      raw=P.range(a,n);assert raw==V.range(a,n);pins[a]=raw
 out={'schema':1,'passed':True,'scope':__doc__,'original_sha256':sha(P.b),'v14_sha256':sha(V.b),'producer_sha256':sha(Path(__file__).read_bytes()),'numeric_cases':rows,'csv_cases':strings,'instructions':[{'address':hex(a),'bytes':b.hex()} for a,b in sorted(pins.items())],'limitations':['Constructor values and ordered source field documents are explicit fixture inputs, not native startup/profile discovery.','No actual user profile, saved world, full source-load context capture or completed combat value is supplied.','Numeric variables use real native float2/flags0 registration followed by typed fixed source updates; unknown hardcoded/locked/bounded variable policies remain outside this source binder.','CSV String source variable is created on first declaration; later inputs overwrite SAME variable identity. Nonempty stream file operations remain unsupported.']}
 (HERE/'native-context.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8');print('passed',len(rows),'numeric',len(strings),'CSV',len(pins),'pins')
if __name__=='__main__':main()
