"""Synthetic finite x87 FPREM loop, keeping the full extended result."""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT.parent/'reconstruction/tools/python-packages'))
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_FPCW
f32=lambda n:struct.unpack('<I',struct.pack('<f',n))[0]
f64=lambda n:struct.unpack('<Q',struct.pack('<d',n))[0]

def main():
 cases=[];instructions={}
 for name,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  for kind,pack,sign,edges in [('f32',f32,1<<31,[1,3,0x7fffff,0x800000,0x7f7fffff]),('f64',f64,1<<63,[1,3,0xfffffffffffff,0x10000000000000,0x7fefffffffffffff])]:
   values=sorted(set([pack(x) for x in [0.0,0.1,0.5,1,2,3,7.25,100,1e10]]+edges))
   values += [v|sign for v in values]
   divisors=sorted(set([pack(x) for x in [0.1,0.5,1,3,7.25,16]]+edges))
   divisors += [v|sign for v in divisors]
   u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(0x100000,0x4000)
   u.reg_write(UC_X86_REG_FPCW,cw);context=u.context_save()
   load=b'\xd9\x05' if kind=='f32' else b'\xdd\x05'
   code=load+struct.pack('<I',0x101008)+load+struct.pack('<I',0x101000)
   # FPREM; FNSTSW AX; TEST AH,4; JNZ FPREM. No early result store.
   code+=b'\xd9\xf8\xdf\xe0\xf6\xc4\x04\x75\xf7'
   code+=b'\xd9\xc0\xdb\x3d'+struct.pack('<I',0x102000)
   code+=b'\xd9\x1d'+struct.pack('<I',0x102010)+b'\xdd\xd8'
   u.mem_write(0x100000,code);instructions[kind]=code.hex()
   for x in values:
    for y in divisors:
     u.context_restore(context);u.mem_write(0x101000,struct.pack('<QQ',x,y))
     u.emu_start(0x100000,0x100000+len(code),count=10000)
     cases.append({'precision':name,'kind':kind,'inputs':[x,y],
                   'result80':bytes(u.mem_read(0x102000,10)).hex(),
                   'result32':struct.unpack('<I',u.mem_read(0x102010,4))[0]})
 report={'schema':1,'scope':'Synthetic complete FPREM loop for finite binary32/binary64 values; no game control-flow claim.',
         'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'instructions':instructions,'cases':cases}
 path=HERE/'remainder-vectors.json';path.write_text(json.dumps(report,indent=2),encoding='utf-8')
 print(json.dumps({'cases':len(cases),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
