"""World ctor4F8072..4F809C revision increment; PE zero-tail initialization.
Only that bounded instruction interval is executed, not the full World ctor.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP
ORIGINAL=h.P
def section(image):
 rva=0xb3cc94-image.base
 for vs,r,raw,off in image.sections:
  if r<=rva<r+max(vs,raw):
   assert raw<=rva-r and rva-r+4<=vs
   return dict(virtual_size=vs,rva=r,raw_size=raw,raw_offset=off,word_offset=rva-r,
               initialization='PE loader zero-filled section tail')
 raise AssertionError('revision not in image')
def run(value,precision,image):
 h.P=ORIGINAL if image=='original' else h.V;m=h.Machine(precision)
 assert m.r32(0xb3cc94)==0
 m.w32(0xb3cc94,value)
 def stop(mm):
  mm.u.reg_write(UC_X86_REG_EIP,h.STOP)
 m.seams[0x4f809c]=stop
 m.u.reg_write(UC_X86_REG_ESP,h.STACK)
 m.u.emu_start(0x4f8072,h.STOP,count=64)
 assert m.u.reg_read(UC_X86_REG_EIP)==h.STOP
 assert m.r32(m.esp()+20)==(value+1)&0xffffffff
 return m.r32(0xb3cc94)
def main():
 rows=[]
 for precision,value in itertools.product(h.PRECISION,[0,1,2,0x7fffffff,0x80000000,0xfffffffe,0xffffffff,0x12345678]):
  a=run(value,precision,'original');b=run(value,precision,'v14');assert a==b
  rows.append(dict(precision=precision,before=value,after=a))
 a=section(ORIGINAL);b=section(h.V);assert a==b
 output=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,zero_tail=a,
  original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  instructions=[dict(address=hex(a),bytes=b) for a,b in sorted(h.VISITED.items())])
 path=HERE/'native-world-revision.json'
 if '--write' in sys.argv:path.write_text(json.dumps(output,indent=1)+'\n')
 else:assert json.loads(path.read_text())==output
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
