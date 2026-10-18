"""Actual510580 file-load closed30 retention and4A4C00 copy.
Closed30 is not in file formats11..13; old retained storage is deliberately
poisoned0xA5 in half of these source-load probes, proving no implicit reset.
Fresh source binding is authorized separately by actual50F4C0 ctor rows.
Header parser transport/source lookup/environment boundaries are inherited
and explicit; no whole5A19C0 localization/host-side effect claim.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-mapheader'));import native_header as H
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-mission-start'));from native_creator_constructor import Machine as C,h
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ECX,UC_X86_REG_FPCW
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from null_guard import fs_prefixed
class Header(H.Machine):
 def __init__(self,data,image,pc,before):
  super().__init__(data);self.before=before;self.arrays=[];self.actual={};self.intercepts=set()
  if image=='v14':
   retained={a:self.read(a)for a in [0xb43b40,0xb388a0,0xb36530]}
   old=(H.P.image_size+4095)&~4095;new=(H.V.image_size+4095)&~4095
   if new>old:self.u.mem_map(H.P.base+old,new-old)
   for vs,r,size,off in H.V.sections:self.u.mem_write(H.V.base+r,H.V.b[off:off+size])
   for a,v in retained.items():self.write(a,v)
  self.u.reg_write(UC_X86_REG_FPCW,h.PRECISION[pc])
  def guard(u,access,address,size,value,user):
   if not(address+size<=4 and fs_prefixed(u,u.reg_read(UC_X86_REG_EIP))):raise RuntimeError(f"illegal page0 {address:X} PC{u.reg_read(UC_X86_REG_EIP):X}")
  self.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,guard,begin=0,end=0xfff)
 def hook(self,u,pc,size,user):
  ecx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x513910:self.creator=ecx
  super().hook(u,pc,size,user)
  if pc==0x4a4ce0:
   data=self.read(ecx);n=self.read(ecx+8)
   for i in range(n):self.u.mem_write(data+i*0x6c+0x30,bytes([self.before]))
   self.arrays.append((data,n))
  if u.reg_read(UC_X86_REG_EIP)==pc:self.actual[pc]=size
  else:self.intercepts.add(pc)
def run_source(image,pc,path,before):
 data=H.read_chunks(ROOT.parent/'reconstruction/assets/original/Data/Maps'/path)['MAP_HEADER'][0]
 m=Header(data,image,pc,before);m.run();counts=[]
 # Only active source/Header and appended-Creator record counts, not capacity.
 record_counts=[]
 for params in [m.params,m.creator+4]:
  record_counts.append(m.read(params+0x14))
  counts.extend(m.u.mem_read(m.read(params+0x10)+i*0x6c+0x30,1)[0]for i in range(m.read(params+0x14)))
 assert all(v==before for v in counts),(path,before,counts)
 return dict(count=len(counts),record_counts=record_counts,closed30=counts,payload_sha256=hashlib.sha256(data).hexdigest()),m.actual,m.intercepts

def run_copy(image,pc,before):
 m=C(image,pc,dict(width=192,height=192,independent=0,resource_count=3));src=m.alloc(0x6c);dst=m.alloc(0x6c)
 m.call(0x50f4c0,[1],this=src,callee_pop=4);m.call(0x50f4c0,[0],this=dst,callee_pop=4)
 m.b(src+0x30,before);m.b(src+0x31,0x5a);m.b(src+0x32,0x81)
 m.call(0x4a4c00,[src],this=dst,callee_pop=4)
 out=dict(closed30=m.rb(dst+0x30),remove31=m.rb(dst+0x31),sai32=m.rb(dst+0x32));assert out==dict(closed30=before,remove31=0x5a,sai32=0x81)
 return out,m.visited,m.intercepted

def main():
 rows=[];visited={};seams=set()
 for pc,path,before in itertools.product(h.PRECISION,['Campaign/tutorial02.RMP','DeveloperMaps/(3v3)Firefight.RMP','(2v2)Berlin.RMP'],[0,0xa5]):
  a,pa,sa=run_source('original',pc,path,before);b,pb,sb=run_source('v14',pc,path,before);assert a==b
  visited.update(pa);visited.update(pb);seams|=sa|sb;rows.append(dict(kind='source',precision=pc,path=path,before=before,output=a))
 for pc,before in itertools.product(h.PRECISION,[0,1,0xa5,255]):
  a,pa,sa=run_copy('original',pc,before);b,pb,sb=run_copy('v14',pc,before);assert a==b
  visited.update(pa);visited.update(pb);seams|=sa|sb;rows.append(dict(kind='copy',precision=pc,before=before,output=a))
 pins=[]
 for va,n in sorted(visited.items()):
  if va in seams or not 0x401000<=va<0x9ef000:continue
  raw=h.P.range(va,n);assert raw==h.V.range(va,n);pins.append(dict(va=hex(va),bytes=raw.hex()))
 data=(json.dumps(dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,executions=len(rows)*2,instructions=pins,seams=[hex(a)for a in sorted(seams)]),indent=1)+'\n').encode();path=HERE/'native-closed30.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()


