"""Bounded native World44/48 ctor stores, current-map Load and started store.
Loader proof admits versions16..28 only; legacy zone verification is excluded.
Actual stream reads are explicit value seams, preserving exact destination/order.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
ORIGINAL=h.P
def run(c,image):
 h.P=ORIGINAL if image=='original' else h.V;m=h.Machine(c['precision']);world=m.alloc(0x2a0)
 m.w32(world+0x44,0xa5a5a5a5,0xffffffff)
 m.u.reg_write(UC_X86_REG_ESI,world);m.u.reg_write(UC_X86_REG_EBX,0)
 def stop(mm):mm.u.reg_write(UC_X86_REG_EIP,h.STOP)
 m.seams[0x4f7d79]=stop;m.u.emu_start(0x4f7d73,h.STOP,count=16)
 assert [m.r32(world+0x44),m.r32(world+0x48)]==[0,0]
 resource=m.alloc(0xb4);vt=m.alloc(4);stub=h.STUBS+0x180
 m.w32(resource+0x3c,vt);m.w32(vt,stub);reads=[]
 def read(mm):
  dest,n=mm.arg(),mm.arg(1);assert n==4
  offset=dest-world;assert offset in [0x44,0x48]
  reads.append(offset);mm.w32(dest,c['word44'] if offset==0x44 else c['word48']);mm.ret(8,4)
 m.seams[stub]=read;m.seams[0x4fc7df]=stop
 m.u.reg_write(UC_X86_REG_EBX,resource);m.u.reg_write(UC_X86_REG_EDI,world);m.u.reg_write(UC_X86_REG_EBP,0)
 m.u.reg_write(UC_X86_REG_ESP,h.STACK);m.w32(h.STACK+0x14,c['version'])
 m.u.emu_start(0x4fc76c,h.STOP,count=128)
 assert reads==[0x44,0x48]
 loaded=[m.r32(world+0x44),m.r32(world+0x48)]
 m.seams[0x4fdff8]=stop;m.u.reg_write(UC_X86_REG_EBX,world)
 m.u.emu_start(0x4fdff1,h.STOP,count=16)
 return dict(constructed=[0,0],reads=reads,loaded=loaded,started=m.r32(world+0x48))
def main():
 rows=[]
 for precision,version,words in itertools.product(h.PRECISION,[16,27,28],[(0,0),(2,1),(0xffffffff,128)]):
  c=dict(precision=precision,version=version,word44=words[0],word48=words[1])
  a=run(c,'original');b=run(c,'v14');assert a==b;rows.append(dict(input=c,output=a))
 out=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  instructions=[dict(address=hex(a),bytes=b) for a,b in sorted(h.VISITED.items())])
 path=HERE/'native-mission-fields.json'
 if '--write' in sys.argv:path.write_text(json.dumps(out,indent=1)+'\n')
 else:assert json.loads(path.read_text())==out
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
