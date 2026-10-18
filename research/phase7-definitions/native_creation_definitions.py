"""Original View aspect iteration and per-append total-weight stores."""
from pathlib import Path
import json,sys,struct
from probe_lists import Machine as Base
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import original,modified,sha
from unicorn.x86_const import *
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
class Machine(Base):
 def __init__(self,enabled,weights,cw):
  super().__init__();self.enabled=enabled;self.weights=weights;self.loaded=[];self.aspect_nodes={};self.u.reg_write(UC_X86_REG_FPCW,cw)
  self.view=self.node('View');self.object=self.alloc(0x180);self.w(self.object+0x118,0xffffffff)
  self.prepare('AspectNormal',-1,bits(.25))
  for i,w in enumerate(weights):self.prepare('Aspect',i,w)
 def prepare(self,name,tag,weight):
  node=self.node(name,16 if name=='Aspect'else 0,ordinal=tag&0xffffffff);self.add(self.view,node)
  aspect=self.alloc(0x70);self.w(aspect+12,self.string([ord(c)for c in f'aspect{tag}']));self.w(aspect+16,1);self.w(aspect+0x2c,weight);self.aspect_nodes[node]=(aspect,tag)
 def hook(self,u,pc,n,z):
  if pc==0x5b5380:self.ret(int(self.enabled),pop=4)
  elif pc==0x49c670:
   aspect,tag=self.aspect_nodes[self.arg()];self.loaded.append(tag);self.w(u.reg_read(UC_X86_REG_ESP)+4,aspect);u.reg_write(UC_X86_REG_EIP,0x49c360)
  else:super().hook(u,pc,n,z)
 def run(self):
  self.w(self.stack,self.stop)
  for reg,value in [(UC_X86_REG_ESP,self.stack),(UC_X86_REG_EBP,self.object),(UC_X86_REG_ESI,self.view),(UC_X86_REG_EBX,0)]:self.u.reg_write(reg,value)
  self.u.emu_start(0x49e309,0x49e3a3,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==0x49e3a3
  assert self.loaded==([]if self.enabled else[-1])+list(range(len(self.weights)))
  return {'enabled':self.enabled,'weights':self.weights,'normal_weight':bits(.25),'loaded_tags':self.loaded,'stored_count':self.r(self.object+0x110),'stored_total_bits':self.r(self.object+0x128),'normal_index':self.r(self.object+0x118)}
def main():
 rows=[]
 weights=[[],[bits(1)],[bits(.1),bits(.2)],[bits(16777216),bits(1),bits(1)],[bits(1),bits(1),bits(16777216)],[bits(.2),bits(.7),bits(13),bits(.000001)]]
 for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  for enabled in[False,True]:
   for w in weights:rows.append({'precision':pc,**Machine(enabled,w,cw).run()})
 ranges={'aspect_iteration':(0x49e309,0x49e3a3),'aspect_append':(0x49c360,0x49c49a),'weight_read_validation':(0x46aa69,0x46aaa9),'normal_default_ctor':(0x49c760,0x49ca90)}
 for a,b in ranges.values():assert original.range(a,b-a)==modified.range(a,b-a)
 p=Path(__file__).with_name('native-creation-definitions.json');p.write_text(json.dumps({'schema':1,'passed':True,'producer_sha256':sha(Path(__file__).read_bytes()),'machine_sha256':sha(Path(__file__).with_name('probe_lists.py').read_bytes()),'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'cases':rows,'ranges':{n:{'start':hex(a),'end':hex(b),'sha256':sha(original.range(a,b-a))}for n,(a,b)in ranges.items()},'seams':['Authored flag read returns supplied bool. Model loading in49C670 is replaced by the matching prepared AspectData and then tail-enters original49C360.','Original list lookup/iteration and native append/duplicate-ID check/float total accumulation execute. Memory allocation/free and generated list-name formatting use the documented probe_lists.py seams.','This does not claim asset file/model loading or full ObjectData initialization.']},indent=2));print('passed',len(rows))
if __name__=='__main__':main()
