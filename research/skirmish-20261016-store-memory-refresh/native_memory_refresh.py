"""4EC4A0 memory refresh with actual six class vtables and actual CString/leaf bodies.
Fixture fields are explicit guard probes, not saved source. Required-stop rows
stop at actual entries after all prior record stores; no returning fake virtual.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ECX
H=lambda b:hashlib.sha256(b).hexdigest()
VT={2:0xa2a2a0,3:0xa273f0,4:0xa299d8,6:0xa28a00,7:0xa28b88,9:0xa28e90}
NAMES={2:0x4ef270,3:0x4d99d0,4:0x4e5340,6:0x4db840,7:0x4dbf90,9:0x4ef270}
class Machine(h.Machine):
 def __init__(self,pc,image,c):
  super().__init__(b'',pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   self.u.mem_write(image.base,image.b[:0x1000])
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+raw])
  self.c=c;self.required=None;self.events=[];self.nstats=0
  self.actor=self.alloc(0x600);self.definition=self.alloc(0x600);self.record=self.alloc(0x38)
  self.world=self.alloc(0x300);self.w(0xb36534,self.world);self.w(self.world+0xe0,c['time'])
  self.w(self.actor,VT[c['class']]);self.w(self.actor+4,self.definition);self.w(self.actor+8,c['flags'])
  self.w(self.actor+0x20,c['position'][0]);self.w(self.actor+0x24,c['position'][1]);self.w(self.actor+0x40,c['variant'])
  self.w(self.definition,0xa0f6e0 if c['class']==9 else 0);self.w(self.definition+0x10,self.cstring(c['display']));self.wb(self.definition+0x108,int(c['data_variations']))
  self.w(self.actor+0xe8,c['owner']);self.w(self.actor+0x100,0);self.w(self.actor+0x15c,0xffffffff)
  self.w(self.actor+0x170,0);self.w(self.actor+0xdc,self.cstring(c['key']));self.w(self.actor+0xe0,self.cstring(c['text']))
  self.w(self.actor+0x198,self.cstring(c['kingdom']));self.w(self.actor+0x80,self.cstring(c['area']));self.w(self.actor+0x64,self.cstring(c['camera']))
  #Area/Camera labels share Actor component words only in distinct constructor classes.
  if c['class']in(3,4):
   for off in(0x60,0x8c,0x98):self.w(self.actor+off,0)
  self.w(self.record,0xbf800000);self.w(self.record+4,self.cstring('old'));self.w(self.record+8,0);self.w(self.record+0xc,0)
  for off in range(0x10,0x38,4):self.w(self.record+off,0xa5a5a5a5)
  for entry,label in[(NAMES[c['class']],'name'),(0x4eaaa0,'aspect')]:
   self.probes[entry]=lambda m,label=label:m.enter(label)
  for entry in {int.from_bytes(h.P.range(VT[c["class"]]+off,4),"little")for off in(0x10c,0x110,0x114,0x118,0x11c)}:
   self.probes[entry]=lambda m:m.stat()
 def enter(self,label):
  self.events.append(label)
  if self.c['stop']==label:self.required=label;self.u.emu_stop()
 def stat(self):
  self.nstats+=1;self.events.append('stat'+str(self.nstats))
  if self.c['stop']=='stat'+str(self.nstats):self.required='stat'+str(self.nstats);self.u.emu_stop()
 def run(self):
  try:self.call(0x4ec4a0,self.actor,[self.record])
  except AssertionError:
   if not self.required:raise
  assert self.u.reg_read(UC_X86_REG_EIP)==h.STOP or self.required
  return {'outcome':self.required or'complete','time':self.r(self.record),'name':self.cstring_text(self.r(self.record+4)),'position':[self.r(self.record+8),self.r(self.record+0xc)],'definition':self.r(self.record+0x10)==self.definition,'definition_raw':0 if self.r(self.record+0x10)==self.definition else self.r(self.record+0x10),'owner':self.r(self.record+0x14),'aspect':self.r(self.record+0x18),'stats':[self.r(self.record+off)for off in(0x1c,0x20,0x24,0x28,0x2c)],'byte30':self.b(self.record+0x30),'flags':self.r(self.record+0x34),'events':self.events}
def main():
 rows=[]
 for cls in VT:
  for saved in(False,True):
   for stop in('none','name','aspect','stat1','stat3','stat5'):
    c={'class':cls,'time':0x40500000,'flags':0x40a12400,'position':[0x3fc00000,0xc0200000],'variant':17,'owner':0x30011,'display':'Pine \u00e9','key':'saved'if saved else'','text':'Saved unit','kingdom':'Allies','area':'Front area','camera':'Camera A','stop':stop}
    for variations in ([True,False]if cls==9 else[True]):
     c=dict(c,data_variations=variations)
     for pc in h.CW:
      values=[Machine(pc,image,c).run()for image in(h.P,h.V)];assert values[0]==values[1]
      rows.append({'pc':pc,'input':c,'native':values[0]})
 pins=[]
 stopped={NAMES[c['input']['class']]for c in rows if c['input']['stop']=='name'}|{0x4eaaa0,0x4db6b0,0x4cec50,0x4cec90,0x4cece0}
 for a,b in sorted(h.VISITED.items()):pins.append({'va':hex(a),'bytes':b,'kind':'executed_or_explicit_required_probe_entry'if a in stopped else'executed'})
 tables=[{'class':cls,'vtable':hex(vt),'slots':{hex(o):h.P.range(vt+o,4).hex()for o in(0x24,0xb4,0x108,0x10c,0x110,0x114,0x118,0x11c)}}for cls,vt in VT.items()]
 for t in tables:
  for o,b in t['slots'].items():assert h.V.range(int(t['vtable'],16)+int(o,16),4).hex()==b
 assert h.P.range(0x9f32dc,4)==h.V.range(0x9f32dc,4)==b'\0'*4
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(h.P.b),'v14_sha256':H(h.V.b),'source_scope':'Saved variation17 and Data.byte108 are explicit probe fields; Doodad actual Data vtable40 true is also executed with byte108false. Source producer is separately guarded in Rust','native_zero_stat':{'va':'0x9f32dc','bytes':'00000000'},'tables':tables,'rows':rows,'machines':len(rows)*2,'pins':pins}
 p=HERE/'native-memory-refresh.json'
 if'--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':doc['machines'],'pins':len(pins),'fixture_sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
