"""Exact single native43FF36 AD store; continuation stops at43FF3D.
No whole43F0A0 success/children/GPU claim. Neighbor AC/AE/AF bytes retained.
"""
from pathlib import Path
import sys,json,itertools,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(h.Machine):
 def run(self,before):
  p=self.alloc(0x200);self.u.mem_write(p+0xac,bytes(before))
  stub=h.STUBS+0x10000
  code=b'\x56\xbe'+struct.pack('<I',p)+b'\xe8'+struct.pack('<i',0x43ff36-(stub+11))+b'\x5e\xc3'
  self.u.mem_write(stub,code);self.seams[0x43ff3d]=lambda m:m.ret()
  self.call(stub);return list(self.u.mem_read(p+0xac,4))
def main():
 h.VISITED.clear();rows=[]
 for pc,ac,ad,neighbor in itertools.product(h.PRECISION,[0,1,255],[0,1,255],[0,165]):
  before=[ac,ad,neighbor,neighbor^255]
  h.P=ORIGINAL;a=Machine(pc).run(before);h.P=V14;b=Machine(pc).run(before)
  assert a==b==[ac,1,neighbor,neighbor^255]
  rows.append(dict(precision=pc,before=before,after=a))
 h.P=ORIGINAL;pins={hex(a):v for a,v in sorted(h.VISITED.items())if a==0x43ff36}
 assert len(pins)==1
 for a,v in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(v)))==V14.range(int(a,16),len(bytes.fromhex(v)))
 data=(json.dumps(dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),rows=rows,pins=pins),indent=1)+'\n').encode()
 path=HERE/'native-completion.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=H(data))))
ORIGINAL=h.P;V14=h.V
if __name__=='__main__':main()
