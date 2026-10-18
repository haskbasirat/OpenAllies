"""442C00 direct-field constructor prefix through442D0E and43F0A0 startup
through43F1D4. Screen7040E0 executes. User reset, source-cache, VGrid constructor
and diagnostic are declared child seams;213-UVar reset is an explicit in-body
composition boundary43F156..43F1B2, separately proved by aa-ui view_variables.
No texture/environment/terrain/VObject/finalAD work or readiness is claimed.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE/'map-resource-mode'))
import native_tail_constructor as base
h=base.h
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ECX
class Machine(h.Machine):
 def __init__(self,pc,fill):
  super().__init__(b'',pc);self.view=self.alloc(0x300,fill);self.events=[];self.outcome='running'
 def fields(self):return dict(revision=self.r(self.view+0xb8),flags=[self.b(self.view+o)for o in[0xac,0xad,0xae]],words=[self.r(self.view+o)for o in[0x84,0x88,0x8c,0x90,0xec]],grid=bool(self.r(self.view+0x94)))
 def stop(self,name):self.outcome=name;raise h.Throw()
 def ctor(self):
  for va,pop in[(0x5f1d00,8),(0x424b90,0),(0x42e730,4)]:self.seam(va,lambda m,pop=pop:m.ret(pop))
  self.probes[0x442d0e]=lambda m:self.stop('constructor_fields')
  try:self.call(0x442c00,self.view,[0x12345678])
  except h.Throw:pass
  assert self.outcome=='constructor_fields'
  return self.fields()
 def init(self,c):
  for o,v in zip([0x84,0x88,0x8c,0x90,0xec],c['words']):self.w(self.view+o,v)
  for o,v in zip([0xac,0xad,0xae],c['flags']):self.u.mem_write(self.view+o,bytes([v]))
  self.w(self.view+0xb8,c['old_revision']);self.w(self.view+0x94,0x1234 if c['old_grid'] else 0)
  screen=self.alloc(0x40,0);self.w(0xb400dc,screen)
  world=self.alloc(0x100,0);gmap=self.alloc(0x100,0);creator=self.alloc(0x100,0)
  self.w(0xb36534,world);self.w(world+0x30,gmap);self.w(gmap+0x24,creator)
  def event(name):
   if c['stop']==name:self.stop('error:'+name)
   self.events.append(name)
  self.probes[0x7040e0]=lambda m:event('screen')
  for va,pop,name in[(0x746e30,0,'user'),(0x5ae0a0,4,'resource'),(0x404f60,0,'diagnostic')]:
   self.seam(va,lambda m,pop=pop,name=name:(event(name),m.ret(pop)))
  def allocate(m):
   assert m.arg(0)==0x110;event('allocate_grid');m.ret(0,0 if c['null_grid']else m.alloc(0x110,0))
  def grid(m):event('grid');m.ret(0,m.u.reg_read(UC_X86_REG_ECX))
  def uvars(m):event('uvars');m.u.reg_write(UC_X86_REG_EIP,0x43f1b2)
  self.seam(0x6f7810,allocate);self.seam(0x430290,grid)
  self.probes[0x43f156]=uvars;self.probes[0x43f1d4]=lambda m:self.stop('prefix_complete')
  try:self.call(0x43f0a0,self.view,[c['revision']])
  except h.Throw:pass
  return dict(fields=self.fields(),events=self.events,screen_byte24=self.b(screen+0x24),outcome=self.outcome)
def main():
 rows=[];ctors=[];h.VISITED.clear()
 cases=[dict(words=[0x11111111+i for i in range(5)],flags=[0x80,0,0xff],old_revision=99,revision=r,old_grid=old,null_grid=null,stop=stop)
  for r,old,null,stop in itertools.product([0,0xffffffff],[False,True],[False,True],[None,'screen','user','resource','allocate_grid','grid','diagnostic','uvars'])if not(null and stop=='grid')]
 for image,pe in[('original',base.ORIGINAL),('v14',base.V14)]:
  h.P=pe
  for pc in h.CW:
   for fill in[0,0xa5,0xff]:
    out=Machine(pc,fill).ctor();assert out==dict(revision=0,flags=[0,0,0],words=[0]*5,grid=False)
    ctors.append(dict(image=image,precision=pc,fill=fill,output=out))
   for c in cases:rows.append(dict(image=image,precision=pc,input=c,output=Machine(pc,0xa5).init(c)))
 h.P=base.ORIGINAL
 for pc in h.CW:
  originals=[r['output']for r in rows if r['image']=='original'and r['precision']==pc]
  assert originals==[r['output']for r in rows if r['image']=='v14'and r['precision']==pc]
 excluded={0x5f1d00,0x424b90,0x42e730,0x442d0e,0x746e30,0x5ae0a0,0x404f60,0x6f7810,0x430290,0x43f156,0x43f1d4}
 pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in excluded}
 for a,b in pins.items():assert base.ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==base.V14.range(int(a,16),len(bytes.fromhex(b)))
 doc=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),machine_sha256=h.H(Path(h.__file__).read_bytes()),constructor_rows=ctors,rows=rows,pins=pins,seams=[hex(a)for a in sorted(excluded)])
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-vworld-prefix.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),constructors=len(ctors),pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
