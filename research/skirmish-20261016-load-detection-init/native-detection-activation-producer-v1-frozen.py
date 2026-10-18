"""Real56EC90->56EB40->4D69A0/516640->515F40/552640/515970.
Actual2696/6477 boundary inputs and explicitly labelled field probes. Stops at
the genuine World+248 read515A20, or required release entry514CE0; it never
supplies a fake solver result. Allocation is the only returning seam. Each PE
executes independently at all three x87 precision controls.
"""
from pathlib import Path
import json,hashlib,sys,copy,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_EBX,UC_X86_REG_EDI
H=lambda b:hashlib.sha256(b).hexdigest()
LAYOUT=ROOT/'research/skirmish-20261016-battle-terrain/actual-map-inputs.json'
rows=[m for m in json.loads(LAYOUT.read_text())['maps'] if m['source']=='Maps/Campaign/tutorial02.RMP']
assert len(rows)==3 and all(r['base_layout']==rows[0]['base_layout'] for r in rows)
g=rows[0]['grid'];extent=rows[0]['base_layout']['extent_i32']
GEOMETRY={'shift':g['coordinate_shift'],'tile_size_bits':g['cell_size'],'extent_bits':[struct.unpack('<I',struct.pack('<f',x))[0] for x in extent],'width':g['width'],'height':g['height']}

class Machine(h.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   self.u.mem_write(image.base,image.b[:0x1000])
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
  self.actor=self.alloc(0x180);self.detection=self.alloc(0x18);self.world=self.alloc(0x300);self.map=self.alloc(0x60)
  self.u.mem_write(self.actor,b'\xa5'*0x180);self.u.mem_write(self.detection,b'\xa5'*0x18)
  self.u.mem_write(self.world,b'\xa5'*0x300);self.u.mem_write(self.map,b'\xa5'*0x60)
  self.w32(self.actor,0xa25b28,0);self.w32(self.actor+8,c['flags']);self.w32(self.actor+0x14,c['owner'])
  self.w32(self.actor+0x20,*c['position_bits']);self.w32(self.actor+0x100,c['state_low']);self.w32(self.actor+0x138,0)
  self.w32(self.actor+0x148,0,0,9);self.w32(self.actor+0xb8,self.detection)
  self.w32(self.detection,0xa3c568,self.actor,0xa3c540,0,0,c['radius'])
  self.w32(0xb36534,self.world);self.w32(self.world+0x30,self.map)
  self.w32(self.map+4,GEOMETRY['shift'],GEOMETRY['tile_size_bits'],*GEOMETRY['extent_bits'])
  self.w32(self.map+0x1c,GEOMETRY['width'],GEOMETRY['height'])
  self.stop=None;self.allocations=[]
  if c['existing']:
   vector=self.alloc(36);self.zone=self.alloc(24)
   self.w32(self.actor+0x148,vector,9,9);self.w32(vector+24,self.zone)
   self.w32(self.zone,0xa3229c,self.actor,c['existing_radius'],*c['existing_xy']);self.u.mem_write(self.zone+0x14,struct.pack('<HB',0x40,2))
  self.seams[0x6f7810]=lambda m:m.allocate()
 def allocate(self):
  n=self.arg();assert n in (36,24),n;self.allocations.append(n)
  p=self.alloc(n);self.u.mem_write(p,b'\xa5'*n);self.ret(value=p)
 def _hook(self,u,pc,size,_):
  if pc in (0x515a20,0x514ce0):
   h.VISITED[pc]=bytes(u.mem_read(pc,size)).hex();assert h.P.range(pc,size)==h.V.range(pc,size)
   self.stop={'entry':hex(pc)}
   if pc==0x515a20:self.stop['tile']=[u.reg_read(UC_X86_REG_EBX),u.reg_read(UC_X86_REG_EDI)]
   u.emu_stop();return
  super()._hook(u,pc,size,_)
 def run(self):
  try:self.call(0x56ec90,self.detection)
  except AssertionError:
   if not self.stop:raise
  assert self.u.reg_read(UC_X86_REG_EIP)==h.STOP or self.stop
  pointer,size,capacity=self.r32(self.actor+0x148,3)
  zone=None
  if size>6:
   z=self.r32(pointer+24)
   if z:
    assert self.r32(z+4)==self.actor
    zone={'radius_bits':self.r32(z+8),'x':self.r32(z+12),'y':self.r32(z+16),'flags':int.from_bytes(self.u.mem_read(z+20,2),'little'),'state':self.r8(z+22)}
  return {'outcome':'complete' if not self.stop else 'required'+self.stop['entry'],'stop':self.stop,'vector_size':size,'vector_capacity':capacity,'zone6':zone,'allocation_sizes':self.allocations,'detection_radius':self.r32(self.detection+20),'detection_reveal':self.r32(self.detection+16)}

def inputs():
 out=[];pins=[]
 for owner in (2696,6477):
  p=HERE/'boundary-01'/f'actual-{owner}-before-56ec90.json';raw=p.read_bytes();a=json.loads(raw);pins.append({'path':str(p.relative_to(ROOT)).replace('\\','/'),'sha256':H(raw)})
  assert a['actor_words']['312']==0 and a['actor_state']['low']&0x200000==0
  c={'owner':owner,'flags':a['thing_flags'],'state_low':a['actor_state']['low'],'position_bits':a['position_bits'],'radius':a['detection']['radius'],'existing':False,'existing_radius':0xbf800000,'existing_xy':[0,0]}
  out.append({'label':f'actual{owner}_fresh','actual':True,'input':copy.deepcopy(c)})
  for label,changes in [('inactive_existing',{'existing':True}),('active_same',{'existing':True,'existing_radius':c['radius']}),('active_different',{'existing':True,'existing_radius':0x3f800000}),('zero',{'radius':0}),('negative',{'radius':0xbf800000}),('dead',{'flags':c['flags']|1}),('clamp_low',{'position_bits':[0xbf800000,0,0]}),('clamp_high',{'position_bits':[0x43c80000,0x43c80000,0]})]:
   v=copy.deepcopy(c);v.update(changes);out.append({'label':f'probe{owner}_{label}','actual':False,'input':v})
 return out,pins
def main():
 cases,pins=inputs();rows=[]
 for c in cases:
  for pc in h.PRECISION:
   v=[Machine(pc,image,c['input']).run() for image in (h.P,h.V)];assert v[0]==v[1]
   rows.append({**c,'precision':pc,'native':v[0]})
 boundary={0x515a20,0x514ce0};seams={0x6f7810}
 d={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'actual_boundary_files':pins,'geometry':GEOMETRY,'geometry_source':{'path':str(LAYOUT.relative_to(ROOT)).replace('\\','/'),'sha256':H(LAYOUT.read_bytes())},'rows':rows,'fresh_original_v14_machines':len(rows)*2,'address_byte_pins':[{'va':hex(a),'bytes':b,'kind':'required_entry' if a in boundary else 'allocator_entry' if a in seams else 'executed'} for a,b in sorted(h.VISITED.items())]}
 path=HERE/'native-detection-activation.json'
 if '--write' in sys.argv:path.write_text(json.dumps(d,indent=1)+'\n')
 else:assert json.loads(path.read_text())==d
 print(json.dumps({'rows':len(rows),'machines':len(rows)*2,'executed_pins':sum(p['kind']=='executed' for p in d['address_byte_pins']),'all_pins':len(d['address_byte_pins']),'fixture_sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
