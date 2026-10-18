"""Execute original region stream loops and saved water cache reader.

Heap/vector allocation, version framing and byte I/O are seams. Region-grid
publication/postload rebuilding are skipped after all serialized reads.
"""
from pathlib import Path
import sys,json,struct,hashlib,time
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-mapheader'))
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
import native_header as base
from original_rng import original,modified,sha
from unicorn.x86_const import *
from inspect_header import Reader,read_chunks

class Machine(base.Machine):
 def __init__(self,data,width):
  super().__init__(data)
  self.graph=self.alloc(0x100);self.cache=self.alloc(0x100)
  world=self.alloc(0x100);self.grid=self.alloc(0x100);self.write(0xb36534,world);self.write(world+0x30,self.grid);self.write(self.grid+0x1c,width)
  self.write(self.cache+0x28,self.alloc(4096*4));self.write(self.cache+0x2c,4096)
 def hook(self,u,pc,size,user):
  if pc not in {0x50b530,0x45d8a0,0x4a6ad0,0x7ba4f0,0x96d18d,0x7bae77,0x7b99b0,0x7aaac6,0x7aac74,0x2e00000,0x8589b0,0x6f7810,0x6f7430,0x4849f0}:return
  ecx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x50b530:
   self.vector(ecx,self.arg(),4,[]);self.write(ecx+4,self.arg());self.ret(4);return
  if pc==0x45d8a0:self.vector(ecx,self.arg(),8,[]);self.ret(4);return
  if pc in (0x4a6ad0,):self.ret();return
  if pc==0x7ba4f0:
   self.write(ecx,self.arg());self.write(ecx+0x5c,self.alloc(64*4));self.write(ecx+0x60,64);self.ret(4,value=ecx);return
  if pc==0x96d18d:
   #The allocated list-array backing is already zero, matching empty list ctors.
   self.ret(20);return
  if pc==0x7bae77:u.reg_write(UC_X86_REG_EIP,0x7baebe);return
  if pc==0x7b99b0:self.ret(4);return
  if pc in (0x7aaac6,0x7aac74):u.emu_stop();return
  if pc==0x2e00000:
   dest,n=self.arg(),self.arg(1);u.mem_write(dest,self.r.take(n));self.ret(8);return
  if pc==0x8589b0:
   v=self.r.u32();assert v<=self.arg(1);self.write(ecx,v);self.ret(8,value=ecx);return
  if pc==0x6f7810:self.ret(value=self.alloc(self.arg()));return
  if pc==0x6f7430:self.ret();return
  if pc==0x4849f0:self.vector(ecx,self.arg(),4,[]);self.ret(4);return
 def call(self,pc,ecx,args):
  self.write(self.stack,self.stop)
  for i,value in enumerate(args):self.write(self.stack+4+i*4,value)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,ecx)
  self.u.emu_start(pc,self.stop,count=20000000)
 def run_cache(self):
  self.call(0x7aa9b0,self.graph,[self.stream-0x3c,self.grid])
  assert self.u.reg_read(UC_X86_REG_EIP)in(0x7aaac6,0x7aac74)
  start=self.r.at
  self.call(0x55c540,self.cache,[self.stream-0x3c])
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
  vertices=bytes(self.u.mem_read(self.read(self.cache+0x1c),self.read(self.cache+0x20)*8))
  indices=bytes(self.u.mem_read(self.read(self.cache+0x10),self.read(self.cache+0x14)*4))
  entries=[]
  for bucket in range(4096):
   entry=self.read(self.read(self.cache+0x28)+4*bucket)
   while entry:
    entries.append(list(struct.unpack('<5I',self.u.mem_read(entry,20))));entry=self.read(entry+20)
  entries.sort()
  return {'cache_offset':start,'consumed':self.r.at,'vertex_count':len(vertices)//8,'index_count':len(indices)//4,'cell_count':len(entries),'vertices_sha256':sha(vertices),'indices_sha256':sha(indices),'entries_sha256':sha(json.dumps(entries,separators=(',',':')).encode()),'entries':entries}

def parse(data):
 r=Reader(data);assert r.u32()==0
 if r.byte():
  for _ in range(r.count()):
   assert r.u32()==0;r.take(8);r.take(r.count()*8)
  for _ in range(r.count()):
   assert r.u32()==0;r.take(8)
   for _ in range(4):
    r.take(2)
    for _ in range(r.count()):assert r.u32()==0;r.take(12)
 start=r.at;assert r.u32()==0;vertices=r.take(r.count()*8);indices=r.take(r.count()*4)
 entries=[list(struct.unpack('<4I',r.take(16)))for _ in range(r.count())]
 assert r.at==len(data)
 return start,vertices,indices,entries

def main():
 rows={};started=time.monotonic()
 source=json.loads((ROOT/'research/phase6-spatial/native-real-map-terrain.json').read_text())
 for path,row in source['maps'].items():
  d=read_chunks(ROOT.parent/'reconstruction/assets/original/Data'/path)['TERRAIN'][0]
  assert sha(d)==row['terrain_sha256']
  suffix=d[row['suffix_offset']:];start,vertices,indices,entries=parse(suffix)
  m=Machine(suffix,row['layout']['width']);got=m.run_cache()
  assert got['cache_offset']==start and got['consumed']==len(suffix),path
  assert got['vertices_sha256']==sha(vertices)and got['indices_sha256']==sha(indices)
  assert got['entries']==sorted([[x+y*row['layout']['width'],x,y,a,b]for x,y,a,b in entries])
  #Actual lookup executes original55B9E0 including collision-chain search.
  lookup=[]
  for cell in (entries[:2]+[[row['layout']['width']-1,row['layout']['height']-1,0,0]]):
   xy=m.alloc(8);out=m.alloc(16);m.write(xy,cell[0]);m.write(xy+4,cell[1]);m.call(0x55b9e0,m.cache,[xy,out]);exists=bool(m.u.reg_read(UC_X86_REG_EAX)&255)
   expected=next((e for e in reversed(entries)if e[:2]==cell[:2]),None)
   assert exists==(expected is not None)
   if exists:assert list(struct.unpack('<4I',m.u.mem_read(out,16)))==expected
   lookup.append({'cell':cell[:2],'exists':exists})
  del got['entries'];rows[path]={'terrain_sha256':sha(d),'suffix_offset':row['suffix_offset'],'width':row['layout']['width'],'height':row['layout']['height'],**got,'lookups':lookup}
  if len(rows)%10==0:print('validated',len(rows),flush=True)
 ranges={'region_reader':(0x7aa9b0,0x7aac90),'region_node':(0x7bbaf0,0x7bbc68),'region_cell':(0x7bacf0,0x7baed3),'region_side':(0x7b94a0,0x7b956c),'cache_reader':(0x55c540,0x55c665),'cache_lookup':(0x55b9e0,0x55ba49),'cache_hash_lookup':(0x55b410,0x55b463),'cache_vtable':(0xa3af1c,0xa3af24)}
 for a,b in ranges.values():assert original.range(a,b-a)==modified.range(a,b-a)
 report={'schema':1,'passed':True,'producer_sha256':sha(Path(__file__).read_bytes()),'original_sha256':sha(original.b),'accepted_sha256':sha(modified.b),'ranges':{n:{'start':hex(a),'end':hex(b),'sha256':sha(original.range(a,b-a))}for n,(a,b)in ranges.items()},'maps':rows,'seconds':time.monotonic()-started,'scope':'Original region serialized loops and complete55C540 cache reader with real hash insertion; full80source maps. Allocation/vector reserve, version helper and I/O are seams. Region object constructors create empty containers; grid publication and postload region linking are skipped after byte reads. No runtime shore regeneration or height/water shading claim.'}
 (HERE/'native-water-geometry.json').write_text(json.dumps(report,indent=2));print('passed',len(rows),report['seconds'])
if __name__=='__main__':main()
