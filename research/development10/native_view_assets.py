"""Execute43F1D4..43F948 asset-lookup segment of real43F0A0.
Actual outer prologue/prefix stores run;213-UVar loop is an explicit skipped
composition boundary. Environment43EA50, current alias410CB0, texture5A77E0,
FX4BDB70 and diagnostic404EA0 are typed child seams. CString cleanup executes
with the established allocator transport. No graphics-backend or AD claim.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE/'map-resource-mode'));import native_tail_constructor as base
h=base.h
from unicorn.x86_const import UC_X86_REG_EIP
TEXTURES=['TextureCircle','TextureSquare','TextureHollowCircle','TextureHollowSquare']
FX=['VFXMoveCommand','VFXSelectFacing','VFXHealthBar','VFXMoraleBar','VFXSupplyStockpileBar','VFXAttackEfficiencyBar','VFXGhosting','VFXIllegalPlacement','VFXCircleBrush','VFXSquareBrush','VFXProjectedBoxTexture','VFXProjectedCircleTexture','VFXLocationMarkerEditor','VFXRecruitMarkerRallyPoint','VFXText','VFXTextUnscaled','VFXDamageNumbers','VFXHealNumbers']
OFFSETS=[0x1c0,0x1cc,0x1d0,0x1d4,0x1d8,0x1dc,0x1e0,0x1e4,0x1e8,0x1ec,0x1f0,0x1f4,0x1f8,0x1fc,0x200,0x204,0x1c4,0x1c8]
class Machine(h.Machine):
 def __init__(self,pc,c):
  super().__init__(b'',pc);self.c=c;self.events=[];self.outcome='running';self.tags={0:None,0x11223344:'old'}
  self.view=self.alloc(0x300,0);self.last=None
  for offset in [0x11c,0x120,0x124,0x128]+OFFSETS:self.w(self.view+offset,0x11223344)
  world=self.alloc(0x100);gmap=self.alloc(0x100);creator=self.alloc(0x100)
  self.w(0xb36534,world);self.w(world+0x30,gmap);self.w(gmap+0x24,creator)
  screen=self.alloc(0x40);self.w(0xb400dc,screen)
  for va,pop in[(0x746e30,0),(0x5ae0a0,4),(0x404f60,0)]:self.seam(va,lambda m,pop=pop:m.ret(pop))
  self.seam(0x6f7810,lambda m:m.ret(0,0))
  self.probes[0x43f156]=lambda m:m.u.reg_write(UC_X86_REG_EIP,0x43f1b2)
  self.seam(0x43ea50,lambda m:(m.event('environment'),m.ret(0)))
  self.seam(0x410cb0,self.alias);self.seam(0x5a77e0,self.texture);self.seam(0x4bdb70,self.fx)
  self.seam(0x404ea0,self.warning)
  self.probes[0x43f948]=lambda m:m.stop('segment_complete')
 def stop(self,text):self.outcome=text;raise h.Throw()
 def event(self,text):
  if self.c['stop']==text:self.stop('error:'+text)
  self.events.append(text)
 def literal(self,p):
  words=[]
  for i in range(1024):
   word=int.from_bytes(self.u.mem_read(p+i*2,2),'little')
   if word==0:return bytes().join(w.to_bytes(2,'little')for w in words).decode('utf-16-le')
   words.append(word)
  raise AssertionError('unterminated native literal')
 def alias(self,m):
  key=m.literal(m.arg(1));assert key in TEXTURES+FX,key
  self.event('lookup:'+key);self.last=key;destination=m.arg(0)
  self.w(destination,self.cstring(key));self.ret(8,destination)
 def texture(self,m):
  key=self.cstring_text(self.r(m.arg(0)));assert key==self.last and m.arg(1)==0
  self.event('texture:'+key);p=0 if key in self.c['missing']else self.alloc(4)
  if p:self.tags[p]=key
  self.ret(0,p)
 def fx(self,m):
  key=self.cstring_text(m.arg(0));assert key==self.last
  self.event('fx:'+key);p=0 if key in self.c['missing']else self.alloc(0x40)
  if p:
   descriptor=self.alloc(0x20);self.w(p+0x18,descriptor)
   self.w(descriptor+0x18,3 if key in self.c['wrong_kind']else 12);self.tags[p]=key
  self.ret(4,p)
 def warning(self,m):
  text=self.literal(m.arg(4));self.event('warning:'+text);self.ret(0)
 def run(self):
  self.call(0x43f0a0,self.view,[123])
  assert not self.null_accesses,self.null_accesses
  assert self.outcome=='segment_complete'or self.outcome.startswith('error:'),self.outcome
  return dict(outcome=self.outcome,events=self.events,textures=[self.tags[self.r(self.view+o)]for o in[0x11c,0x120,0x124,0x128]],fx=[self.tags[self.r(self.view+o)]for o in OFFSETS],ad=self.b(self.view+0xad))
def main():
 cases=[dict(stop=None,missing=[],wrong_kind=[])]
 for key in TEXTURES+FX:cases.append(dict(stop=None,missing=[key],wrong_kind=[]))
 for key in FX[-2:]:cases.append(dict(stop=None,missing=[],wrong_kind=[key]))
 full=Machine('Double53',cases[0]).run()['events']
 for event in full:cases.append(dict(stop=event,missing=[],wrong_kind=[]))
 for key,warning in[(FX[0],'VFXMoveCommand VFX not found'),(FX[-2],'VFXDamageNumbers must point to a VFXTEXT'),(FX[-1],'VFXHealNumbers must point to a VFXTEXT')]:
  cases.append(dict(stop='warning:'+warning,missing=[key],wrong_kind=[]))
 h.VISITED.clear();rows=[]
 for pc,c in itertools.product(h.CW,cases):
  h.P=base.ORIGINAL;a=Machine(pc,c).run();h.P=base.V14;b=Machine(pc,c).run();assert a==b
  rows.append(dict(precision=pc,input=c,output=a))
 h.P=base.ORIGINAL
 excluded={0x746e30,0x5ae0a0,0x404f60,0x6f7810,0x43f156,0x43ea50,0x410cb0,0x5a77e0,0x4bdb70,0x404ea0,0x43f948}
 pins={hex(a):v for a,v in sorted(h.VISITED.items())if 0x43f1d4<=a<0x43f946 and a not in excluded}
 for a,b in pins.items():assert base.ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==base.V14.range(int(a,16),len(bytes.fromhex(b)))
 doc=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),rows=rows,pins=pins,seams=[hex(a)for a in sorted(excluded)])
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-view-assets.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=h.H(data))))
if __name__=='__main__':main()
