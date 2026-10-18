"""Real4D1350 ->589A60 ->565140 on actual post-AfterInit2696 fields.
Three captured managed loads are each replayed on original/v14 at all3PC.
State descriptor/mask fields are projected from the earlier native477FC0
registration fixture. 4B2E60 is only source-SVar lookup transport, returning
the exact captured AllowSellUnderSiege bool cell; it is not a fake gameplay
effect. Actual unchanged actions skip4CFCC0; changed-mask publication is reused
from earlier action proofs. Positive UI/renderer entries remain required.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import UC_X86_REG_EIP
H=lambda b:hashlib.sha256(b).hexdigest()
REG=ROOT/'research/phase7-actors/native-actor-states.json';registration=json.loads(REG.read_text())['registration']
class Machine(h.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   self.u.mem_write(image.base,image.b[:0x1000])
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
  self.c=c;self.events=[];self.required=None;self.lookups=[]
  self.actor=self.alloc(0x200);definition=self.alloc(0x600);self.manager=self.alloc(0x400);self.table=self.alloc(42*4)
  self.w32(0xb36530,self.manager);self.w32(self.manager+0x2e4,self.table)
  for r in registration['records']:
   d=self.alloc(0x40);self.w32(self.table+r['index']*4,d);self.w32(d+0xc,r['index']);self.w32(d+0x2c,r['group']);self.w8(d+0x30,int(r['exclusive']))
  for i,m in enumerate(registration['masks']):self.w32(0xb3ac40+i*8,m);self.w32(0xb3ac44+i*8,m>>32)
  assert h.P.range(0x4d6dbd,6)==h.V.range(0x4d6dbd,6)==bytes.fromhex('c706f073a200')
  self.w32(self.actor,0xa273f0,definition,c['flags'],c['thing_word_c'])
  self.w8(self.actor+0x4c,c['object4c']);self.w32(self.actor+0xe8,self.alloc(0x400) if c['owning_kingdom'] else 0)
  self.w32(self.actor+0x100,c['state']);self.w32(self.actor+0x104,c['state']>>32)
  self.w32(self.actor+0xf0,c['actions']);self.w32(self.actor+0xf4,c['actions']>>32)
  assert c['thing_word_c']==0,'actual renderer fields required if nonnull'
  assert set(c['slots'])=={0,6,9,13,15,22},c['slots']
  for slot in c['slots']:
   component=self.alloc(0x80);self.w32(component+4,self.actor);self.w32(self.actor+0x60+slot*4,component)
  structure=self.r32(self.actor+0x94);assert c['structure']['owner']==c['owner'] and c['structure']['parent']==0 and c['structure']['queue_count']==0
  self.w32(structure+0x10,0);self.w32(structure+0x18,c['structure']['queue_count'])
  self.w32(self.r32(self.actor+0x78)+0x1c,c['properties_flags'])
  d=c['construction'];self.w32(definition+0x4c0,d['upgrade_count']);self.w8(definition+0x4e4,int(d['can_sell']));self.w8(definition+0x4e5,int(d['can_raze']))
  self.cell=self.alloc(4);self.w8(self.cell,int(c['allow_sell_under_siege']));self.w32(0xb3d65c,0)
  self.seams[0x4b2e60]=lambda m:m.lookup()
 def lookup(self):
  p=self.arg();s=bytearray()
  while bytes(self.u.mem_read(p,2))!=b'\0\0':s+=bytes(self.u.mem_read(p,2));p+=2
  assert s.decode('utf-16le')=='AllowSellUnderSiege';self.lookups.append({'name':s.decode('utf-16le'),'value':self.c['allow_sell_under_siege']});self.ret(value=self.cell,pop=8)
 def _hook(self,u,pc,n,d):
  if pc in (0x4d1350,0x589a60,0x565140,0x4cfcc0):self.events.append(hex(pc))
  if pc in (0x76c7f0,0x4cfd0e):
   h.VISITED[pc]=bytes(u.mem_read(pc,n)).hex();assert h.P.range(pc,n)==h.V.range(pc,n);self.required=hex(pc);u.emu_stop();return
  super()._hook(u,pc,n,d)
 def run(self):
  try:self.call(0x4d1350,self.actor)
  except AssertionError:
   if not self.required:raise
  assert self.u.reg_read(UC_X86_REG_EIP)==h.STOP or self.required
  return {'outcome':self.required or 'complete','actions':self.r32(self.actor+0xf0)|(self.r32(self.actor+0xf4)<<32),'events':self.events,'source_svar_lookups':self.lookups}
def main():
 actual_path=HERE/'actual-farm-action-inputs.json';actual=json.loads(actual_path.read_text());assert len(actual['rows'])==3
 rows=[]
 for index,capture in enumerate(actual['rows']):
  for pc in h.PRECISION:
   values=[Machine(pc,image,capture['before']).run() for image in(h.P,h.V)];assert values[0]==values[1];assert values[0]['outcome']=='complete'
   assert values[0]['actions']==capture['after']['actions']
   rows.append({'capture_index':index,'precision':pc,'input':capture['before'],'native':values[0]})
 caller=h.P.range(0x4d7330,0xe);assert caller==h.V.range(0x4d7330,0xe)
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'actual_inputs_sha256':H(actual_path.read_bytes()),'registration_projection_sha256':H(REG.read_bytes()),'caller_static_byte_pin':{'va':'0x4d7330','bytes':caller.hex()},'rows':rows,'machines':len(rows)*2,'address_pins':[{'va':hex(a),'bytes':b,'kind':'source_svar_transport_entry' if a==0x4b2e60 else 'executed'} for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-farm-action-refresh.json'
 if '--write' in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':len(rows)*2,'pins':len(doc['address_pins']),'fixture_sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
