"""Trusted local Windows x64 x87 reference; no SDK code or game modification.

The small generated leaf preserves the caller's full FXSAVE64 state, FNINITs,
loads an explicit control word and raw80 input, executes exactly FSIN or FCOS,
stores raw80/status/control, and restores the saved state. No Unicorn or host
libm result is treated as transcendental ground truth. Results describe THIS
CPU, not every x87 implementation and not the whole Settlement fit.
"""
from pathlib import Path
import ctypes, json, platform, struct, sys
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine, original, modified, sha, STACK, PRECISIONS
from unicorn.x86_const import UC_X86_REG_EBP, UC_X86_REG_ESP, UC_X86_REG_FPCW
from capstone import Cs,CS_ARCH_X86,CS_MODE_64
HERE=Path(__file__).resolve().parent

class HardwareTrig:
    def __init__(self):
        assert platform.system()=='Windows' and struct.calcsize('P')==8
        self.k=ctypes.WinDLL('kernel32',use_last_error=True)
        self.k.VirtualAlloc.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_ulong,ctypes.c_ulong]
        self.k.VirtualAlloc.restype=ctypes.c_void_p
        self.k.VirtualProtect.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_ulong,ctypes.POINTER(ctypes.c_ulong)]
        self.k.VirtualProtect.restype=ctypes.c_int
        self.k.VirtualFree.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_ulong]
        self.k.GetCurrentProcess.restype=ctypes.c_void_p
        self.k.FlushInstructionCache.argtypes=[ctypes.c_void_p,ctypes.c_void_p,ctypes.c_size_t]
        self.k.FlushInstructionCache.restype=ctypes.c_int
        self.buffer=self.k.VirtualAlloc(None,4096,0x3000,4)
        assert self.buffer and self.buffer%16==0
        self.code_pages=[];self.calls={};self.instructions={}
        for name,opcode in [('fsin',b'\xd9\xfe'),('fcos',b'\xd9\xff')]:
            code=(b'\x48\x0f\xae\x81\x00\x02\x00\x00' #fxsave64[rcx+512]
                  b'\xdb\xe3'                               #fninit
                  b'\xd9\x29'                               #fldcw[rcx]
                  b'\xd9\x79\x02'                           #fnstcw[rcx+2]
                  b'\xdb\x69\x10'+opcode+                   #fld tbyte[rcx+16]; trig
                  b'\xdb\x79\x20'                           #fstp tbyte[rcx+32]
                  b'\xdd\x79\x30'                           #fnstsw[rcx+48]
                  b'\x48\x0f\xae\x89\x00\x02\x00\x00'   #fxrstor64[rcx+512]
                  b'\xc3')
            address=self.k.VirtualAlloc(None,4096,0x3000,4);assert address
            ctypes.memmove(address,code,len(code));old=ctypes.c_ulong()
            assert self.k.VirtualProtect(address,4096,0x20,ctypes.byref(old))
            assert self.k.FlushInstructionCache(self.k.GetCurrentProcess(),address,len(code))
            self.code_pages.append(address)
            self.calls[name]=ctypes.CFUNCTYPE(None,ctypes.c_void_p)(address)
            self.instructions[name]={'bytes':code.hex(),'sha256':sha(code),'instructions':[f'{i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_64).disasm(code,0)]}
    def trig(self,name,cw,raw80):
        assert len(raw80)==10
        ctypes.memset(self.buffer,0,128)
        ctypes.memmove(self.buffer,struct.pack('<H',cw),2)
        ctypes.memmove(self.buffer+16,raw80,10)
        self.calls[name](self.buffer)
        data=ctypes.string_at(self.buffer,64)
        assert struct.unpack_from('<H',data,2)[0]==cw
        return {'raw80':data[32:42].hex(),'status_word':struct.unpack_from('<H',data,48)[0],'control_word':struct.unpack_from('<H',data,2)[0]}
    def close(self):
        for p in self.code_pages+[self.buffer]:assert self.k.VirtualFree(p,0,0x8000)

def angle(pc,sides):
    # Original460C07..460C18 executes FILD/FDIVR/FLD, then stops BEFORE
    # Unicorn's FSIN. Thus raw80 input is original arithmetic, not host float.
    m=Machine(pc);actor=m.alloc(0x500);m.w(actor+0x470,sides)
    m.u.reg_write(UC_X86_REG_EBP,actor);m.u.reg_write(UC_X86_REG_ESP,STACK)
    m.u.reg_write(UC_X86_REG_FPCW,PRECISIONS[pc])
    m.u.emu_start(0x460c07,0x460c18,count=50)
    value=m.st0_live();m.verify_v14();assert not m.null_accesses
    return struct.pack('<QH',value['extended_significand'],value['extended_sign_exponent']),m.visited

def main():
    reference=HardwareTrig();rows=[];pins={}
    try:
        for pc,cw in PRECISIONS.items():
            for sides in [3,4,5,6,7,8,9,12,16,24,32,64,128,1024,1000000,2147483647]:
                raw,visited=angle(pc,sides);pins.update(visited)
                for operation in ['fsin','fcos']:
                    value=reference.trig(operation,cw,raw)
                    assert value==reference.trig(operation,cw,raw),'repeat stability'
                    # These finite |angle|<pi operands must not report C2
                    # range-reduction failure, invalid, divzero or overflow.
                    assert value['status_word'] & (0x400|1|4|8)==0
                    rows.append({'precision':pc,'sides':sides,'operation':operation,'native_angle_raw80':raw.hex(),'result':value})
        # Identity anchors prove the expected instruction/store mapping.
        zero=bytes(10)
        assert reference.trig('fsin',0x37f,zero)['raw80']==zero.hex()
        assert reference.trig('fcos',0x37f,zero)['raw80']==struct.pack('<QH',1<<63,0x3fff).hex()
        out={'schema':1,'passed':True,'scope':'Trusted local x64 hardware FSIN/FCOS reference only. Not a software implementation, universal CPU accuracy proof,460940 fit or source-load precision observation.','cpu':platform.processor(),'platform':platform.platform(),'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'shim':reference.instructions,'cases':rows,'native_angle_instructions':[{'address':hex(a),'bytes':b.hex()} for a,b in sorted(pins.items())]}
        (HERE/'hardware-trig.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8')
        print('passed',len(rows),'hardware cases, repeated, plus sin0/cos0 anchors; CPU:',platform.processor())
    finally:reference.close()

if __name__=='__main__':main()
