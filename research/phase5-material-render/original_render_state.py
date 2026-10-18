"""Execute original alpha/depth property decisions; intercept only state emission.

Native constructor instructions populate the enum lookup tables. No original
game process is started and no graphics device/property decision is mocked.
The leaf SetRenderState logger replaces the GPU driver side effect only.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32

RANGES={'alpha':(0x73a5b0,0x73a6f0),'enum_constructor':(0x913b91,0x913d36),
        'constructor_register_preconditions':(0x913a50,0x913b91),
        'depth':(0x913680,0x9136df),'renderer_getter':(0x85cc90,0x85cc96),
        'depth_default_inline_constructor':(0x71e22f,0x71e242),
        'depth_type_getter':(0x6fe850,0x6fe856),'depth_rtti_init':(0x9e7e50,0x9e7e6b)}
for a,b in RANGES.values(): assert original.range(a,b-a)==modified.range(a,b-a)

class RenderMachine(Machine):
    ROOT_OBJECT=0x2000000; PASS=0x2004000; PROPERTIES=0x2005000; PROPERTY=0x2006000
    VTABLE=0x2007000; SINK=0x300f100
    def __init__(self):
        super().__init__()
        self.calls=[]
        self.u.hook_add(UC_HOOK_CODE,self.hook)
        self.reset(0)
        for register,value in [(UC_X86_REG_ESI,self.STATE),(UC_X86_REG_EBP,2),(UC_X86_REG_EDI,3),(UC_X86_REG_EBX,0)]:
            self.u.reg_write(register,value)
        self.u.emu_start(0x913b91,0x913d36,count=1000)
        self.tables={'blend':[self.r(self.STATE+0x28+i*4)for i in range(11)],
                     'alpha_compare':[self.r(self.STATE+0x54+i*4)for i in range(8)],
                     'depth_compare':[self.r(self.STATE+0x110+i*4)for i in range(8)]}
        self.w(self.STATE,self.VTABLE); self.w(self.VTABLE+0x70,self.SINK)
        self.w(0xb43bcc,self.STATE)
        self.w(self.ROOT_OBJECT+0x38,self.PASS); self.w(self.PASS+0x38,self.STATE)
        self.w(self.PROPERTIES+8,self.PROPERTY)
    def hook(self,u,address,size,user):
        if address not in (0x8a20c0,self.SINK):return
        sp=self.u.reg_read(UC_X86_REG_ESP)
        self.calls.append([self.r(sp+4),self.r(sp+8),self.r(sp+12)])
        self.u.reg_write(UC_X86_REG_ESP,sp+16)
        self.u.reg_write(UC_X86_REG_EIP,self.r(sp))
    def alpha(self,flags,threshold):
        self.reset(self.VTABLE);self.calls=[]
        self.u.mem_write(self.PROPERTY+0x20,struct.pack('<HB',flags,threshold))
        self.u.reg_write(UC_X86_REG_ECX,self.ROOT_OBJECT)
        self.w(self.STACK+4,0);self.w(self.STACK+8,self.PROPERTIES)
        self.u.emu_start(0x73a5b0,self.STOP,count=1000)
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+12
        return list(self.calls)
    def depth(self,flags,function):
        self.reset(self.VTABLE);self.calls=[]
        self.w(self.PROPERTY+0x20,flags);self.w(self.PROPERTY+0x24,function)
        self.w(self.STACK+4,self.PROPERTY)
        self.u.emu_start(0x913680,self.STOP,count=1000)
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
        return list(self.calls)

def main():
    machine=RenderMachine()
    flags={193,4845,4844,237,4333,4097,236}
    flags.update(1|(src<<1)|(dst<<5)for src in range(11)for dst in range(11))
    flags.update(236|512|(compare<<10)for compare in range(8))
    alpha=[{'flags':f,'threshold':t,'calls':machine.alpha(f,t)}for f in sorted(flags)for t in [0,96,150,255]]
    depth=[{'flags':f,'function':c,'calls':machine.depth(f,c)}for f in range(4)for c in range(8)]
    sha=lambda b:hashlib.sha256(b).hexdigest()
    result={'schema':1,'method':__doc__,
            'preconditions':{'enum_constructor':'Starts913B91 with EBX0 from913A52, EBP2 from913B16, EDI3 from913B1B. Allocation/reference registration before this region is not executed.',
                             'depth_mode':'state+10C=0: ordinary Z-buffer, not optional W-buffer mode.',
                             'leaf_hook':'73A5B0 calls8A20C0 and913680 calls virtual+70. Only these SetRenderState emissions are captured.'},
            'original_sha256':sha(original.b),'modified_sha256':sha(modified.b),
            'ranges':{k:{'start':hex(a),'end':hex(b),'sha256':sha(original.range(a,b-a))}for k,(a,b)in RANGES.items()},
            'tables':machine.tables,'alpha':alpha,'depth':depth}
    out=HERE/'native-render-state-vectors.json';out.write_text(json.dumps(result,indent=2)+'\n')
    dis=Cs(CS_ARCH_X86,CS_MODE_32)
    (HERE/'native-render-state-assembly.txt').write_text('\n\n'.join(k+'\n'+'\n'.join(f'{i.address:08x} {i.mnemonic} {i.op_str}'for i in dis.disasm(original.range(a,b-a),a))for k,(a,b)in RANGES.items()))
    print(json.dumps({'alpha_cases':len(alpha),'depth_cases':len(depth),'tables':machine.tables,'sha256':sha(out.read_bytes())}))

if __name__=='__main__':main()
