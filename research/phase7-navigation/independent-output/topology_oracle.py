"""Original serialized region load, tile publication, adjacency and point lookup.
No invented edges; only allocator and byte/version transport are fixture leaves.
"""
from pathlib import Path
import hashlib,json,struct,sys,time
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase6-mapheader'))
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
import native_header as base
from original_rng import original,modified
from inspect_header import read_chunks
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE,UC_HOOK_BLOCK
from unicorn.x86_const import *
H=lambda b:hashlib.sha256(b).hexdigest()

class Machine(base.Machine):
 def __init__(self,data,layout,flags,precision='Double53'):
  self.r=base.Reader(data);self.u=Uc(UC_ARCH_X86,UC_MODE_32);self.u.mem_map(0,4096)
  self.u.mem_map(base.P.base,(base.P.image_size+4095)&~4095)
  for vs,r,size,off in base.P.sections:self.u.mem_write(base.P.base+r,base.P.b[off:off+size])
  self.u.mem_map(0x2000000,0x1000000);self.u.mem_map(0x4000000,0x8000000)
  self.heap=0x4000000;self.executed={};self.stack=0x2f08000;self.stop=0x2f10000
  self.stream=self.alloc(0x100);table=self.alloc(0x100);self.write(self.stream,table);self.write(table,0x2e00000)
  for pc in [0x2e00000,0x8589b0,0x6f7810,0x6f7430]:self.u.hook_add(UC_HOOK_CODE,self.hook,begin=pc,end=pc)
  self.u.hook_add(UC_HOOK_BLOCK,self.block)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[precision])
  self.graph=self.alloc(0x100);world=self.alloc(0x100);self.grid=self.alloc(0x100)
  self.write(0xb36534,world);self.write(world+0x30,self.grid);self.write(0xb3652c,self.graph)
  self.write(self.grid+4,layout['coordinate_shift']);self.write(self.grid+8,layout['cell_size_bits'])
  self.write(self.grid+0xc,struct.unpack('<I',struct.pack('<f',layout['extent_i32'][0]))[0]);self.write(self.grid+0x10,struct.unpack('<I',struct.pack('<f',layout['extent_i32'][1]))[0])
  self.write(self.grid+0x1c,layout['width']);self.write(self.grid+0x20,layout['height'])
  self.tiles=self.alloc(layout['width']*layout['height']*48);self.write(self.grid+0x28,self.tiles)
  variants=[]
  for kind in range(3):
   v=self.alloc(0x40);d=self.alloc(0x80);self.write(v+0x1c,d);self.write(d+0x1c,kind);variants.append(v)
  for i,flag in enumerate(flags):self.write(self.tiles+i*48,variants[(flag>>4)&7]);self.write(self.tiles+i*48+0x20,flag)
 def alloc(self,n):
  if not hasattr(self,'heap'):return super().alloc(n)
  p=self.heap;self.heap+=(n+15)&~15;assert self.heap<0xbf00000
  if n:self.u.mem_write(p,b'\0'*n)
  return p
 def block(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:self.executed[pc]=size
 def hook(self,u,pc,size,_):
  ecx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x2e00000:
   dest,n=self.arg(),self.arg(1);u.mem_write(dest,self.r.take(n));self.ret(8);return
  if pc==0x8589b0:
   version=self.r.u32();assert version<=self.arg(1);self.write(ecx,version);self.ret(8,value=ecx);return
  if pc==0x6f7810:self.ret(value=self.alloc(self.arg()));return
  if pc==0x6f7430:self.ret();return
 def call(self,pc,ecx,args=()):
  self.write(self.stack,self.stop)
  for i,value in enumerate(args):self.write(self.stack+4+i*4,value)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,ecx)
  self.u.emu_start(pc,self.stop,count=50000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop,hex(self.u.reg_read(UC_X86_REG_EIP))
 def run(self):
  self.call(0x7aa9b0,self.graph,[self.stream-0x3c,self.grid]);consumed=self.r.at
  count=self.read(self.graph+0x18);regions=[]
  for i in reversed(range(count)):
   pointer=self.read(self.read(self.graph+0x14)+i*4);self.call(0x7bca60,pointer)
  for i in range(count):
   p=self.read(self.read(self.graph+0x14)+i*4);rectangles=[];n=self.read(p+4)
   while n:rectangles.append(self.read(self.read(n)));n=self.read(n+4)
   self.call(0x7b8fb0,p);category=self.u.reg_read(UC_X86_REG_EAX)
   self.call(0x7b9900,p);area=self.u.reg_read(UC_X86_REG_EAX)
   neighbors=[self.read(self.read(self.read(p+0x74)+j*4))for j in range(self.read(p+0x70))]
   cache={}
   for bucket in range(self.read(p+0x60)):
    entry=self.read(self.read(p+0x5c)+bucket*4)
    while entry:
     cache.setdefault(str(self.read(entry)),self.read(entry+4));entry=self.read(entry+8)
   regions.append({'position_bits':[self.read(p+0x54),self.read(p+0x58)],'category':category,'rectangles':rectangles,'neighbors':neighbors,'area':area,'distance_cache':cache})
  rectangle_ids=[]
  for i in range(self.read(self.grid+0x1c)*self.read(self.grid+0x20)):
   p=self.read(self.tiles+i*48+0x10);rectangle_ids.append(self.read(p)if p else 0xffffffff)
  lookups=[];width=self.read(self.grid+0x1c);shift=self.read(self.grid+4)
  selected=[i for i,r in enumerate(rectangle_ids)if r!=0xffffffff]
  points=[[-1.,0.],[0.,-1.],[float(width<<shift),0.],[0.,float(self.read(self.grid+0x20)<<shift)]]
  for i in selected[::max(1,len(selected)//12)][:12]:points.append([float(((i%width)<<shift))+.25,float(((i//width)<<shift))+.25])
  for point in points:
   bits=[struct.unpack('<I',struct.pack('<f',v))[0]for v in point];self.call(0x7aa3d0,self.graph,bits)
   pointer=self.u.reg_read(UC_X86_REG_EAX);lookups.append({'point_bits':bits,'region':self.read(pointer)if pointer else None})
  return {'consumed':consumed,'regions':regions,'tile_rectangles_sha256':H(struct.pack('<'+'I'*len(rectangle_ids),*rectangle_ids)),'primary_island':self.read(self.read(self.graph+0x2c))if self.read(self.graph+0x2c)else None,'diagonal_bits':self.read(self.graph+0x30),'lookups':lookups}

def main():
 source=json.loads((ROOT/'research/phase6-spatial/native-real-map-terrain.json').read_text())
 sys.path.insert(0,str(ROOT/'research/phase6-spatial'));from original_map_terrain import parse
 types_path=ROOT/'research/engine-baseline/terrain-variation-metadata.json';meta_path=ROOT/'research/phase6-terrain/terrain-render-metadata.json'
 types=json.loads(types_path.read_text())['variations'];meta=json.loads(meta_path.read_text());results={};visited={};started=time.monotonic()
 for path,row in source['maps'].items():
  chunks=read_chunks(ROOT.parent/'reconstruction/assets/original/Data'/path);raw=chunks['TERRAIN'][0]
  version,resolution,biome,palette,records,start,end,_=parse(raw)
  previous=meta['variants'][meta['biome_defaults'][biome]['variation_id']];flags=[]
  for index,flag,region,byte in records:
   d=meta['variants'].get(palette[index],previous);previous=d;kind=types[d['canonical']]['native_type'];f=(flag&~0x70)|(kind<<4)
   flags.append(f|0x300 if f&1 else (f&~0x300)|(0 if d['land']else 0x100)|(0 if d['water']else 0x200))
  assert H(struct.pack('<'+'I'*len(flags),*flags))==row['flags_sha256']
  m=Machine(raw[end:],row['layout'],flags)
  try:got=m.run()
  except Exception:
   print('native error',path,hex(m.u.reg_read(UC_X86_REG_EIP)),'offset',m.r.at);raise
  results[path]={'terrain_sha256':H(raw),'suffix_offset':end,'suffix_sha256':H(raw[end:]),'layout':row['layout'],'world_version':struct.unpack_from('<I',chunks['GWORLD'][0])[0],**got};visited.update(m.executed)
  print('validated',len(results),path,'regions',len(got['regions']),flush=True)
 pins=[]
 for a,n in sorted(visited.items()):
  raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
 report={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(original.b),'v14_sha256':H(modified.b),'types_metadata_sha256':H(types_path.read_bytes()),'render_metadata_sha256':H(meta_path.read_bytes()),'precision':'Double53','maps':results,'native_basic_block_pins':pins,'seams':['allocation/free','version framing','byte stream transport'],'scope':'Complete7AA9B0 region load, real constructors/tile publication/postload island selection, then original7BCA60 adjacency,7B9900 area,7B8FB0 category and7AA3D0/7B9300 lookup. Basic-block pins include interrupted seam blocks; they do not imply execution past hooks. Not full7ABC30 bottleneck/cache setup, old-world static analysis or full-world start.'}
 (OUT/'native-topology.json').write_text(json.dumps(report,indent=2)+'\n');print({'passed':True,'maps':len(results),'seconds':time.monotonic()-started})
if __name__=='__main__':main()
