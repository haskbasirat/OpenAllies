"""Native selected-map confirmation68F629..68F68D after real selector guards.
Lab primitive dialog+12A=false and actual-current-state word exercise the reached
tail; this is not whole68EF50, UI navigation, SetLaunchSettings order transport,
Host, or readiness. Real40F7D0/7841E0 and784130 execute; only the empty CString
pool allocation primitive is transported. Separate7840B0 staging opener shown.
"""
from pathlib import Path
import sys,struct,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
H=lambda b:hashlib.sha256(b).hexdigest();ORIGINAL=h.P;V14=h.V
class Machine(h.Machine):
 def run(self,op,current):
  manager=self.alloc(4);vt=self.alloc(16);leaf=h.STUBS+0x20000;self.w32(manager,vt);self.w32(vt+12,leaf)
  def pool(m):p=m.alloc(20);m.w32(p,manager,0,0,1);m.ret(0,p)
  self.seams[0x856370]=lambda m:m.ret(0,manager);self.seams[leaf]=pool
  if op=='staging':
   p=self.alloc(0x28);self.u.mem_write(p,b'\xa5'*0x28);self.call(0x40f7d0,p);self.call(0x7840b0,p,[1,0,0]);return dict(launch=self.state(p))
  dialog=self.alloc(0x140);self.w8(dialog+0x12a,0);self.w32(0xb3979c,current);out={}
  stub=h.STUBS+0x21000
  code=b'\x53\x56\x57\x55\x81\xec\x40\x01\x00\x00\xbb'+struct.pack('<I',dialog)
  callpc=stub+len(code);code+=b'\xe8'+struct.pack('<i',0x68f629-(callpc+5));self.u.mem_write(stub,code)
  def stop(m):
   p=m.esp()+0x44;out.update(launch=m.state(p),game_type=m.u.reg_read(h.UC_X86_REG_EDI));m.u.emu_stop()
  self.seams[0x68f68d]=stop
  try:self.call(stub)
  except AssertionError:assert self.u.reg_read(h.UC_X86_REG_EIP)==0x68f68d
  return out
 def state(self,p):return dict(kind=self.r32(p),flag04=self.r8(p+4),text_length=self.r32(self.r32(p+8)-12),field0c=self.r32(p+12),field10=self.r32(p+16),flag14=self.r8(p+20),flag15=self.r8(p+21),flag16=self.r8(p+22),field18=self.r32(p+24),field1c=self.r32(p+28),flag20=self.r8(p+32),field24=self.r32(p+36))
def main():
 h.VISITED.clear();rows=[]
 for pc,(op,current)in itertools.product(h.PRECISION,[('confirm',0),('confirm',2),('confirm',7),('confirm',12),('staging',12)]):
  h.P=ORIGINAL;a=Machine(pc).run(op,current);h.P=V14;b=Machine(pc).run(op,current);assert a==b;rows.append(dict(pc=pc,op=op,current=current,output=a))
 h.P=ORIGINAL;pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in [0x68f68d,0x856370]}
 for a,v in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(v)))==V14.range(int(a,16),len(bytes.fromhex(v)))
 data=(json.dumps(dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),rows=rows,pins=pins,stop_entries=['0x68f68d'],seams=['0x856370','synthetic CString pool vf0C']),indent=1)+'\n').encode();p=HERE/'native-selected-confirm.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),executed_pins=len(pins),sha256=H(data))))
if __name__=='__main__':main()
