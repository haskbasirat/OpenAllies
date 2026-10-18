"""Unchanged4F0E50 plus real550190/550210 over all80 raw map heightfields.

Runtime captures supply source-bound grid layouts and actual resolved local
TerrainData scalar fields. Height samples come directly from pinned raw maps.
No height callback, mountain-noise answer, viewer map or rendering metadata is
seamed. Native height construction/stream binding have separate existing proofs.
"""
from pathlib import Path
import gc,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
sys.path.insert(0,str(GAME/'reconstruction/tools'))
from map_extract import read_chunks
ORIGINAL=h.P
class Ground(h.Machine):
 def __init__(self,image,pc):
  h.P=ORIGINAL if image=='original' else h.V;super().__init__(pc)
  self.u.mem_map(0x4000000,0x4000000);self.data_next=0x4001000;self.return_next=h.STUBS
 def allocate(self,size):
  p=(self.data_next+15)&~15;self.data_next=p+max(size,4)
  assert self.data_next<0x8000000,'oracle data allocation extent'
  self.u.mem_write(p,b'\0'*max(size,4));return p
 def prepare(self,row,chunks):
  self.data_next=0x4001000;self.world=self.allocate(0x200);self.grid=self.allocate(0x100)
  grid=row['grid'];self.width=grid['width'];self.height=grid['height']
  self.tiles=self.allocate(self.width*self.height*0x30)
  self.w32(self.world+0x30,self.grid);self.w32(self.grid+4,grid['coordinate_shift'],grid['cell_size'])
  self.w32(self.grid+0x1c,self.width,self.height);self.w32(self.grid+0x28,self.tiles)
  for at,name,layout in ((0x34,'HEIGHT_MAP',row['base_layout']),(0x38,'MOUNTAIN_HEIGHT_MAP',row['overlay_layout'])):
   raw=chunks[name][0];assert struct.unpack_from('<I',raw)[0]==1
   count=layout['width']*layout['height'];assert struct.unpack_from('<II',raw,16)==(layout['coordinate_shift'],count)
   field=self.allocate(0x80);values=self.allocate(count*4)
   self.u.mem_write(values,raw[24:24+count*4]);self.w32(field+0xc,layout['width'],layout['height'],layout['coordinate_shift'],layout['cell_size_bits'],layout['index_bias']);self.w32(field+0x40,values);self.w32(self.world+at,field)
 def sample(self,sample):
  for row in sample['neighborhood']:
   x,y=row['xy'];cell=row['cell'];variation=self.allocate(0x30);definition=self.allocate(0x80)
   self.w32(self.tiles+(y*self.width+x)*0x30,variation);self.w32(variation+0x1c,definition)
   self.w32(definition+0x1c,cell['overlay_kind']);self.w32(definition+0x68,cell['offset_min'],cell['offset_max'])
  xy=self.allocate(8);result=self.allocate(4);self.w32(xy,*sample['position'])
  #Every return-capture stub gets a fresh address: no stale translated SMC.
  stub=self.return_next;self.return_next+=32
  self.u.mem_write(stub,b'\xd9\x1d'+struct.pack('<I',result)+b'\xe9'+struct.pack('<i',h.STOP-(stub+11)))
  self.w32(h.STACK,stub,xy,sample['flags']);self.u.reg_write(h.UC_X86_REG_ESP,h.STACK);self.u.reg_write(h.UC_X86_REG_ECX,self.world)
  keep=[(h.UC_X86_REG_EBX,0x11223344),(h.UC_X86_REG_ESI,0x33445566),(h.UC_X86_REG_EDI,0x55667788),(h.UC_X86_REG_EBP,0x778899aa)]
  for r,value in keep:self.u.reg_write(r,value)
  self.u.emu_start(0x4f0e50,h.STOP,count=200000)
  assert self.u.reg_read(h.UC_X86_REG_EIP)==h.STOP
  assert self.esp()==h.STACK+12 and all(self.u.reg_read(r)==value for r,value in keep)
  assert self.fpu_top()==0
  return dict(height_bits=self.r32(result),abi_preserved=True)
def main():
 captured=json.loads((HERE/'actual-map-inputs.json').read_text())['maps'];samples=[]
 source=GAME/'reconstruction/assets/original/Data'
 for image in ('original','v14'):
  for pc in h.PRECISION:
   machine=Ground(image,pc)
   for row in captured:
    if row['precision']!=pc:continue
    path=source/row['source'];assert hashlib.sha256(path.read_bytes()).hexdigest()==row['map_sha256']
    chunks=read_chunks(path);machine.prepare(row,chunks)
    for sample in row['samples']:
     output=machine.sample(sample)
     assert output['height_bits']==sample['height_bits'],(image,pc,row['source'],sample,output)
     samples.append(dict(source=row['source'],precision=pc,image=image,input=sample,output=output))
   del machine;gc.collect()
 document=dict(schema=1,scope='Native4F0E50 and real height samplers over all80 raw maps with actual source-bound TerrainData metadata',original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  inputs_sha256=hashlib.sha256((HERE/'actual-map-inputs.json').read_bytes()).hexdigest(),samples=samples,
  instructions=[dict(va=hex(va),bytes=raw) for va,raw in sorted(h.VISITED.items())])
 path=HERE/'native-battle-terrain.json'
 if '--write' in sys.argv:path.write_text(json.dumps(document,indent=1)+'\n')
 else:assert json.loads(path.read_text())==document,'frozen native terrain evidence differs'
 print(json.dumps(dict(maps=80,samples=len(samples),instructions=len(h.VISITED),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
