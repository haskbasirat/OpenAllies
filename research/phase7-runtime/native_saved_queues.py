"""Original50BFF0/50BF10/50BD70 saved queue load on original map chunks."""
from pathlib import Path
import sys,struct,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-queues'))
import native_queues as q
sys.path.insert(0,str(ROOT.parent/'reconstruction/tools'))
from map_extract import read_chunks
from unicorn.x86_const import *
class Machine(q.Machine):
 def __init__(self,data):
  super().__init__({'counts':[1,16,80,80,80],'seed':123,'precision':53,'clock':{'begin_bits':0,'end_bits':0,'step_bits':q.bits(.0625)}})
  self.data=data;self.at=4;self.bound=[]
  vt=self.alloc(32);self.write(vt,0x23f0100);self.write(vt+12,0x23f0110);self.write(self.world+0x3c,vt)
  count=struct.unpack_from('<i',data)[0];self.table=self.alloc(12);values=self.alloc(count*4);self.write(self.table,count);self.write(self.table+4,values);self.write(self.world+0x284,self.table)
  for i in range(count):self.write(values+i*4,self.owner(i))
 def hook(self,u,pc,size,arg):
  if pc==0x23f0100:
   dest,n=self.arg(),self.arg(1);out=self.data[self.at:self.at+n];assert len(out)==n,(self.at,n,len(self.data))
   u.mem_write(dest,out);self.at+=n;self.ret(8);return
  if pc==0x23f0110:self.ret(value=0);return
  if pc==0x404f60:self.ret();return
  if pc in [0x858ad0,0x4f0110,0x410c40]:raise RuntimeError(('error',hex(pc),self.at))
  super().hook(u,pc,size,arg)
 def load(self,version):
  for i in [3,4]+([0,1,2]if version>=22 else[]):
   self.call(0x50bf10 if i<3 else 0x50bff0,self.queues[i],[self.world]);self.bound.append([i,self.at])
  assert self.at==len(self.data),(self.at,len(self.data))
def main():
 rows=[]
 for path in sorted((ROOT.parent/'reconstruction/assets/original/Data/Maps').rglob('*.rmp')):
  chunks=read_chunks(path);data=chunks['UPDATABLES'][0];version=struct.unpack_from('<I',chunks['GWORLD'][0])[0]
  m=Machine(data)
  try:m.load(version)
  except Exception:print(path,hex(m.u.reg_read(UC_X86_REG_EIP)),m.at,m.bound);raise
  rows.append({'map':path.relative_to(ROOT.parent).as_posix(),'world_version':version,'payload':data.hex(),'sha256':hashlib.sha256(data).hexdigest(),'boundaries':m.bound,'snapshot':m.snapshot()})
 out={'passed':True,'maps':rows,'original_sha256':hashlib.sha256(q.ORIGINAL.b).hexdigest(),'v14_sha256':hashlib.sha256(q.ACCEPTED.b).hexdigest(),'visited':[{'va':hex(a),'bytes':b}for a,b in sorted(q.VISITED.items())],
 'scope':'Whole original queue Load methods and list-node construction execute. Every source ID is bound to a fixture Updatable pointer to expose all original list entries; this does not establish actor/component restoration or live battle readiness. Allocator/free/logger and byte-stream I/O are fixture services. Runtime must supply actual successfully restored Updatables and discard null slots exactly.'}
 p=HERE/'native-saved-queues.json';p.write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),hashlib.sha256(p.read_bytes()).hexdigest())
if __name__=='__main__':main()
