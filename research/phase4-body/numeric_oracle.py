"""Synthetic x87 arithmetic oracle; isolated Unicorn, no game/process execution.

These small x86 instructions test the shared numeric primitive, not game logic.
Game-specific callers have separate full-original-instruction fixtures.
"""
from pathlib import Path
import hashlib,json,struct,sys
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT.parent/'reconstruction/tools/python-packages'))
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *


def operand(kind,bits):return {'kind':kind,'bits':bits}
def f32(value):return operand('f32',struct.unpack('<I',struct.pack('<f',value))[0])
def f64(bits):return operand('f64',bits)


def run(mode,op,values):
 u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(0x100000,0x4000)
 code=b''
 for i,value in enumerate(values):
  p=0x101000+16*i;kind=value['kind'];n=value['bits']
  if kind=='f32':u.mem_write(p,struct.pack('<I',n));code+=b'\xd9\x05'+struct.pack('<I',p)
  elif kind=='f64':u.mem_write(p,struct.pack('<Q',n));code+=b'\xdd\x05'+struct.pack('<I',p)
  elif kind=='i32':u.mem_write(p,struct.pack('<i',n));code+=b'\xdb\x05'+struct.pack('<I',p)
  else:
   # FILD signed64 is the exact input value of the public u32 constructor.
   u.mem_write(p,struct.pack('<q',n));code+=b'\xdf\x2d'+struct.pack('<I',p)
  if op=='reciprocal_mul' and i==1:code+=b'\xde\xf9'
 code+= {'add':b'\xde\xc1','sub':b'\xde\xe9','mul':b'\xde\xc9','div':b'\xde\xf9','reciprocal_mul':b'\xde\xc9'}[op]
 code+=b'\xd9\x1d'+struct.pack('<I',0x102000)
 u.mem_write(0x100000,code);u.reg_write(UC_X86_REG_FPCW,mode)
 u.emu_start(0x100000,0x100000+len(code),count=100)
 result,=struct.unpack('<I',u.mem_read(0x102000,4))
 return result,code.hex()


def main():
 pairs=[(f32(1),f32(3)),(f32(-1),f32(3)),(f32(0),f32(-0.0)),(f32(-0.0),f32(-0.0)),
        (f32(16777216),f32(1)),(f32(16777218),f32(1)),(f32(1),operand('f32',0x33800000)),
        (operand('f32',1),f32(0.5)),(operand('f32',3),f32(0.5)),
        (operand('f32',0x7f7fffff),f32(2)),(operand('u32',0xffffffff),f64(0x3df0000000100000)),
        (operand('i32',-2147483648),f64(0x41f0000000000000)),
        (f64(0x3ff0000000000001),f64(0x3cb0000000000000)),
        (f64(1),f64(0x3ff0000000000000))]
 rows=[]
 for name,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  for left,right in pairs:
   for op in ['add','sub','mul','div']:
    if op=='div' and right['bits'] in [0,0x80000000] and right['kind']=='f32':continue
    result,code=run(cw,op,[left,right]);rows.append(dict(precision=name,operation=op,values=[left,right],result=result,instructions=code))
  for maximum in [3.0,7.0,100.0,257.0,0.0001,99999.0]:
   for current in [1.0,2.0,0.1,maximum]:
    values=[f32(1),f32(maximum),f32(current)]
    result,code=run(cw,'reciprocal_mul',values);rows.append(dict(precision=name,operation='reciprocal_mul',values=values,result=result,instructions=code))
 p=Path(__file__).with_name('numeric-native-oracle.json')
 report={'schema':1,'passed':True,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'scope':'Synthetic x87 instructions executed in isolated Unicorn, PC24/53/64 nearest-even; finite input, binary32 stores. Not game control-flow evidence.'}
 p.write_text(json.dumps(report,indent=2)+'\n');print(len(rows),hashlib.sha256(p.read_bytes()).hexdigest())


if __name__=='__main__':main()
