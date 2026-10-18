"""Original fixed-function RepColor stage setup, isolated x86; no game/GPU writes."""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1];GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/engine-baseline'))
sys.path.insert(0,str(GAME/'reconstruction/tools/python-packages'))
from audit_baseline import PE
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
native=PE(GAME/'AA.exe');accepted=PE(GAME/'reconstruction/native-game/runtime-expanded-v14/AA.exe')
h=lambda b:hashlib.sha256(b).hexdigest()
assert h(native.b)=='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
RANGES={'base_stage':(0x73a880,0x73a9fa),'diffuse_stage':(0x739b40,0x739bbb),
 'stage_state_writer':(0x89ff80,0x8a0097),'effect_enable':(0x7396b0,0x7396b4),
 'effect_color_pointer':(0x7396c0,0x7396c4),'effect_constructor':(0x739710,0x73978c),
 'effect_rgb_to_rgba':(0x716870,0x7169c0),'stage_order':(0x73adf0,0x73aea0),
 'repcolor_preprocess':(0x87a31a,0x87a453),'inherited_context_init':(0x879a90,0x879b98),
 'child_context_dispatch':(0x87a790,0x87a820),'tag_test':(0x871cc0,0x871d50),
 'recognized_user_tags':(0x877c50,0x877df0),'user_property_parse':(0x8790b0,0x8792f0)}

class Machine:
 def __init__(self,enabled,tag,texture,effect=True):
  self.u=Uc(UC_ARCH_X86,UC_MODE_32);self.u.mem_map(native.base,(native.image_size+4095)&~4095)
  for vs,r,size,off in native.sections:self.u.mem_write(native.base+r,native.b[off:off+size])
  self.u.mem_map(0x2000000,0x20000);self.u.mem_map(0x3000000,0x10000)
  self.renderer=0x2000000;self.effect=0x2001000 if effect else 0;self.prop=0x2002000
  self.count=0x2003000;self.passobj=0x2004000;self.texprop=0x2005000;self.table=0x2006000
  self.stack=0x3008000;self.stop=0x300f000;self.stages={};self.render_states={};self.calls=[];self.tag=tag
  self.w(self.renderer+0x38,self.passobj);self.w(self.passobj+0x38,0x2007000)
  if self.effect:self.u.mem_write(self.effect+0x11,bytes([enabled]))
  self.w(self.prop+0x28,self.texprop);self.w(self.texprop+0x2c,self.table)
  if texture:self.w(self.table,0x2008000);self.w(0x2008004,0x2009000)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def w(self,p,v):self.u.mem_write(p,struct.pack('<I',v))
 def r(self,p):return struct.unpack('<I',self.u.mem_read(p,4))[0]
 def arg(self,n=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+4*n)
 def ret(self,n=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+n)
 def hook(self,u,pc,size,data):
  if pc==0x871cc0:self.calls.append('tag-query');self.ret(value=int(self.tag))
  elif pc==0x8a4780:
   assert self.arg()==0;i=self.arg(1);stage=0x2010000+i*0x100;states=0x2018000+i*0x400
   self.w(stage+12,states);self.stages[i]=(stage,states);self.ret(8,stage)
  elif pc==0x73a7c0:self.calls.append('bind-texture');self.ret(8)
  elif pc==0x702440:
   assert self.arg()==self.effect+0x14;self.calls.append('pack-effect-rgba');self.ret(value=0xff336699)
  elif pc==0x8a20c0:
   assert self.arg(2)==1;self.render_states[self.arg()]=self.arg(1);self.ret(12)
  elif pc==0x8a0830:assert self.arg()==0;self.ret(4)
 def call(self,pc,args):
  self.w(self.stack,self.stop)
  for i,a in enumerate(args):self.w(self.stack+4+4*i,a)
  self.u.reg_write(UC_X86_REG_ECX,self.renderer);self.u.reg_write(UC_X86_REG_ESP,self.stack)
  regs={UC_X86_REG_EBX:11,UC_X86_REG_ESI:12,UC_X86_REG_EDI:13,UC_X86_REG_EBP:14}
  for r,v in regs.items():self.u.reg_write(r,v)
  self.u.emu_start(pc,self.stop,count=50000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
  assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+4+4*len(args)
  for r,v in regs.items():assert self.u.reg_read(r)==v
 def states(self):return {str(i):{str(s):self.r(p+12+s*4) for s in range(1,7)} for i,(_,p) in self.stages.items()}

def main():
 rows=[]
 for name,enabled,tag,texture,effect in [
  ('repcolor_with_texture',True,True,True,True),('repcolor_without_texture_descriptor',True,True,False,True),
  ('disabled_effect_plain_texture',False,True,True,True),('missing_tag_plain_texture',True,False,True,True),
  ('null_effect_plain_texture',False,False,True,False),('plain_missing_texture',False,False,False,True)]:
  m=Machine(enabled,tag,texture,effect);m.call(0x73a880,[m.effect,m.prop,m.count,0x200a000]);before=m.states()
  rep=enabled and tag and effect
  if rep:assert before['0']=={'1':13,'2':2,'3':3,'4':3,'5':2,'6':3};assert m.render_states=={60:0xff336699}
  elif texture:assert before['0']=={'1':2,'2':2,'3':3,'4':2,'5':2,'6':3};assert not m.render_states
  else:assert not before
  index=m.r(m.count);m.call(0x739b40,[m.effect,m.prop,m.count]);after=m.states()
  assert after[str(index)]=={'1':4,'2':1,'3':0,'4':4,'5':1,'6':0}
  rows.append({'name':name,'base_stages':before,'after_diffuse_stage':after,'render_states':m.render_states,'boundary_calls':m.calls})
 evidence={};cs=Cs(CS_ARCH_X86,CS_MODE_32)
 for name,(a,z) in RANGES.items():
  b=native.range(a,z-a);assert b==accepted.range(a,z-a)
  evidence[name]={'start_va':hex(a),'end_exclusive':hex(z),'sha256':h(b),'unchanged_in_v14':True}
  (HERE/f'{name}.asm.txt').write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}' for i in cs.disasm(b,a))+'\n')
 report={'schema':1,'passed':True,'source_sha256':h(native.b),'accepted_sha256':h(accepted.b),'producer_sha256':h(Path(__file__).read_bytes()),'cases':rows,'case_count':len(rows),'native_ranges':evidence,
 'executed':'Full73A880 +739B40, real89FF80 stage-state writes and7396B0/C0 accessors, preserved stack/callee-saved registers.',
 'seams':['Tag result, stage allocation, texture binding, color RGBA packing and render-state sink are explicit boundary stubs. No pixel rendering or D3D driver call.'],
 'formula':{'base_rgb':'texture.rgb * texture.a + team.rgb * (1 - texture.a)','base_alpha':'team.a','diffuse_rgb':'base_rgb * diffuse.rgb','diffuse_alpha':'team.a * diffuse.a','ordinary_team_alpha':'716870 sets1 before739710 copiesRGBA','alpha_policy':'87A3 preprocessing disables RepColor alpha blending/testing; texture.a is a color mask, not cutout opacity.'},
 'official_references':['https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dtextureop','https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dta','https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dtexturestagestatetype'],
 'limits':['Plain base+diffuse stage contract only; later optional decal/detail/glow stages and original lighting/color-space pipeline are not reconstructed.','Team selection in the inspector is explicit preview state, not recovered kingdom ownership.','Opaque target alpha1 is a renderer convention; native diffuse output alpha is not proven universally1.','SG/RepColor tag inheritance is context-dependent; nongeometry node reset at879B72 prevents blindly ORing every ancestor tag.']}
 p=HERE/'repcolor-native-state-oracle.json';p.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'cases':len(rows),'sha256':h(p.read_bytes())}))
if __name__=='__main__':main()
