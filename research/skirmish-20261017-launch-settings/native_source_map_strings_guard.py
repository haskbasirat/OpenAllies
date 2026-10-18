"""Exact5A19F7..5A1A1E source MapStrings guard. Actual4981A0 enum
flags initialization and511860 execute; enum registration storage and reached
47EA40 service are explicit seams. Nonnull is the actual SourceResource
argument from5A2B0E, not an inference from a Header label or Session mode.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-mission-start'));from native_creator_constructor import Machine,h
from unicorn.x86_const import *
def run(image,pc,mode,present):
 m=Machine(image,pc,dict(width=192,height=192,independent=0,resource_count=3));arr=m.alloc(28);m.w(0xb3b198,arr)
 def register():
  obj=m.alloc(12);m.w(obj+8,m.arg(2));m.w(arr+4*m.arg(0),obj);m.ret(12,obj)
 m.seams[0x4976b0]=register;m.call(0x4981a0,[],this=m.alloc(32));header=m.alloc(0x68);rules=m.alloc(32);m.w(header+4,rules);m.w(rules+4,mode)
 resource=m.alloc(0xb4)if present else 0;file=m.alloc(4);m.cs(file,'Maps/explicit.RMP');calls=[]
 def load():
  assert m.arg(0)==resource and m.arg(1)==file;calls.append('47EA40');m.ret(8)
 m.seams[0x47ea40]=load;m.w(m.stack+0x38,resource);m.w(m.stack+0x3c,file);m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_EDI,header)
 m.u.emu_start(0x5a19f7,0x5a1a1e,count=10000);assert m.u.reg_read(UC_X86_REG_EIP)==0x5a1a1e
 return dict(calls=calls),m.visited,m.intercepted

def main():
 rows=[];visited={};seams=set()
 for pc,mode,present in itertools.product(h.PRECISION,range(7),[False,True]):
  a,pa,sa=run('original',pc,mode,present);b,pb,sb=run('v14',pc,mode,present);assert a==b
  assert bool(a['calls'])==(present and mode in [0,1,3]);visited.update(pa);visited.update(pb);seams|=sa|sb;rows.append(dict(precision=pc,mode=mode,resource_present=present,output=a))
 pins=[]
 for va,n in sorted(visited.items()):
  if va in seams or not 0x401000<=va<0x9ef000:continue
  raw=h.P.range(va,n);assert raw==h.V.range(va,n);pins.append(dict(va=hex(va),bytes=raw.hex()))
 data=(json.dumps(dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,executions=len(rows)*2,instructions=pins,seams=[hex(a)for a in sorted(seams)]),indent=1)+'\n').encode();path=HERE/'native-source-map-strings-guard.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
