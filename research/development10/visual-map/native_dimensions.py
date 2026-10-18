"""Actual723E00 dimension slice7241CE..724205, including CRTceil96D4C0
and _ftol96CC4C. No child callback seams; only execution continuation stops
at724205 before next unrelated World+30 read. Inputs finite signed32 domain.
"""
from pathlib import Path
import sys,json,struct,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(h.Machine):
 def run(self,extents):
  world=self.alloc(0x50);terrain=self.alloc(0x158);self.w32(0xb36534,world);self.w32(world+0x3c,*extents)
  self.w32(0xb4b340,1);self.u.reg_write(h.UC_X86_REG_MXCSR,0x1f80)
  self.w32(terrain+0x74,0xa5a5a5a5,0xa5a5a5a5)
  stub=h.STUBS+0x10000;code=b'\x55\xbd'+struct.pack('<I',terrain)+b'\xe8'+struct.pack('<i',0x7241ce-(stub+11))+b'\x5d\xc3'
  self.u.mem_write(stub,code);self.seams[0x724205]=lambda m:m.ret()
  self.call(stub)
  assert self.fpu_top()==0
  return [v if v<0x80000000 else v-0x100000000 for v in self.r32(terrain+0x74,2)]
def main():
 h.VISITED.clear();rows=[]
 values=[0,0x80000000,1,0x80000001,0x3eaaaaab,0xbeaaaaab,0x3fc00000,0xbfc00000,0x3f800001,0xbf800001,0x42ffffff,0xc2ffffff,0x44800000,0x44bfffff,0x4a000001,0x4effffff,0xcf000000]
 pairs=list(zip(values,values[::-1]))+[[h.f2b(a),h.f2b(b)]for a,b in[(256.,512.),(512.,512.),(1536.,2048.),(2047.5,1023.125)]]
 for pc,extents in itertools.product(h.PRECISION,pairs):
  h.P=ORIGINAL;a=Machine(pc).run(extents);h.P=V14;b=Machine(pc).run(extents);assert a==b,(pc,extents,a,b)
  rows.append(dict(precision=pc,extents=extents,dimensions=a))
 h.P=ORIGINAL
 pins={hex(a):raw for a,raw in sorted(h.VISITED.items())if a!=0x724205}
 for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
 data=(json.dumps(dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),rows=rows,pins=pins),indent=1)+'\n').encode();p=HERE/'native-dimensions.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=H(data))))
ORIGINAL=h.P;V14=h.V
if __name__=='__main__':main()
