"""Execute original heightfield initialization and chunk input instructions.

Only allocation and memory-backed stream reads are supplied leaves. Native CRT
ceil/truncation, constructor loops and read order execute. No game process runs.
"""
from pathlib import Path
import hashlib, json, struct, sys
ROOT=Path(__file__).resolve().parents[2]; OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine, original, modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def sha(b):return hashlib.sha256(b).hexdigest()

class BindingMachine(Machine):
    WORLD=0x2003000; SVARS=0x2004000; VALUES=0x2004100
    STREAM=0x2005000; VTABLE=0x2005200; READ=0x200f100; MODE=0x200f110; WRITE=0x200f120
    def __init__(self):
        super().__init__(0x27f)
        self.u.mem_map(0,0x1000)
        self.u.mem_map(0x4000000,0x1000000)
        self.u.hook_add(UC_HOOK_CODE,self.hook)
        self.heap=0x4000000;self.allocations=[];self.reads=[];self.data=b'';self.at=0
        self.visited={}
        self.w(0xb36534,self.WORLD);self.w(0xb39ab0,self.SVARS)
        self.w(self.SVARS,self.VALUES);self.w(self.SVARS+12,self.VALUES+4);self.w(self.SVARS+16,self.VALUES+8)
        self.w(self.STREAM+0x3c,self.VTABLE)
        self.w(self.VTABLE,self.READ);self.w(self.VTABLE+4,self.WRITE);self.w(self.VTABLE+12,self.MODE)
    def ret(self,pop,value=0):
        sp=self.u.reg_read(UC_X86_REG_ESP);dest=self.r(sp)
        self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_ESP,sp+4+pop);self.u.reg_write(UC_X86_REG_EIP,dest)
    def hook(self,u,pc,size,_):
        if pc==0x6f7810:
            size=self.r(u.reg_read(UC_X86_REG_ESP)+4);address=self.heap
            self.heap+=(size+15)&~15
            assert self.heap<0x5000000
            self.allocations.append(size);self.ret(0,address)
        elif pc==self.MODE:self.ret(0,0)
        elif pc==self.WRITE:raise AssertionError('native write attempted during height input')
        elif pc==self.READ:
            sp=u.reg_read(UC_X86_REG_ESP);dst,size=self.r(sp+4),self.r(sp+8)
            assert self.at+size<=len(self.data),(self.at,size,len(self.data))
            u.mem_write(dst,self.data[self.at:self.at+size]);self.reads.append({'offset':self.at,'size':size})
            self.at+=size;self.ret(8,size)
        elif original.base<=pc<original.base+original.image_size:self.visited[pc]=size
    def initialize(self,extent,shift,minimum,maximum,default):
        self.reset(0);self.heap=0x4000000;self.allocations=[]
        self.w(self.WORLD+0x3c,extent[0]);self.w(self.WORLD+0x40,extent[1])
        self.w(self.VALUES,shift);self.w(self.VALUES+4,minimum);self.w(self.VALUES+8,maximum)
        self.w(self.STACK+4,default)
        regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
        for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
        self.u.emu_start(0x5519a0,self.STOP,count=2000000)
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
        assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
        return self.snapshot()
    def snapshot(self):
        count=self.r(self.STATE+12)*self.r(self.STATE+16)
        return {'extent_i32':[self.r(self.STATE+4),self.r(self.STATE+8)],
            'width':self.r(self.STATE+12),'height':self.r(self.STATE+16),'coordinate_shift':self.r(self.STATE+20),
            'cell_size_bits':self.r(self.STATE+24),'index_bias':self.r(self.STATE+28),
            'minimum_bits':self.r(self.STATE+72),'maximum_bits':self.r(self.STATE+76),'default_bits':self.r(self.STATE+80),
            'height_bits':list(struct.unpack('<'+'I'*count,self.u.mem_read(self.r(self.STATE+64),count*4))),
            'flags':list(self.u.mem_read(self.r(self.STATE+68),count)),'owns_storage':bool(self.u.mem_read(self.STATE+84,1)[0])}
    def load(self,data):
        self.reset(0);self.data=data;self.at=0;self.reads=[]
        self.w(self.STACK+4,self.STREAM)
        self.u.emu_start(0x551e60,self.STOP,count=100000)
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
        return {'state':self.snapshot(),'read_calls':self.reads,'consumed':self.at,'remaining':list(data[self.at:])}
    def sample(self,method,position,precision):
        self.reset(0)
        self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[precision])
        output=0x2007000;stub_address=0x300f100
        stub=b'\xdb\x3d'+struct.pack('<I',output)+b'\xdb\x2d'+struct.pack('<I',output)+b'\xd9\x1d'+struct.pack('<I',output+16)
        self.u.mem_write(stub_address,stub);self.w(self.STACK,stub_address)
        self.w(self.STACK+4,position[0]);self.w(self.STACK+8,position[1])
        self.u.emu_start(0x550190 if method=='Nearest' else 0x550210,stub_address+len(stub),count=100000)
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+12
        sig,exp=struct.unpack('<QH',self.u.mem_read(output,10))
        return {'height_bits':self.r(output+16),'extended_significand':sig,'extended_sign_exponent':exp}

def main():
    machine=BindingMachine();construct=[];loads=[]
    for extent in [[0,0],[1,1],[2,6],[1.1,7.9],[16.0,24.0],[15.999,23.001],[448,448]]:
        for shift in [1,2,3,4]:
            e=list(map(bits,extent));minimum,maximum,default=bits(-10),bits(10),bits(2.25)
            state=machine.initialize(e,shift,minimum,maximum,default)
            construct.append({'input':{'extent_bits':e,'resolution_bits':shift,'minimum_bits':minimum,'maximum_bits':maximum,'default_bits':default},'output':state,'allocations':machine.allocations})
    for flags_bias in [0,127]:
        for extent in [[0,0],[2,6],[8,4]]:
            e=list(map(bits,extent));state=machine.initialize(e,1,bits(-10),bits(10),0)
            count=state['width']*state['height'];heights=[bits((i%7-3)*0.125)for i in range(count)];flags=[flags_bias+i%4 for i in range(count)]
            header=[1,bits(-20),bits(30),bits(1.5),1,count]
            data=struct.pack('<'+'I'*(6+count),*header,*heights)+bytes(flags)+b'\x7f'
            loads.append({'extent_bits':e,'resolution_bits':1,'bytes':list(data),'output':machine.load(data)})
    ranges=[]
    for pc,size in sorted(machine.visited.items()):
        if ranges and ranges[-1][1]==pc:ranges[-1][1]=pc+size
        else:ranges.append([pc,pc+size])
    pins=[]
    for start,end in ranges:
        b=original.range(start,end-start);assert b==modified.range(start,end-start)
        pins.append({'start':hex(start),'end':hex(end),'sha256':sha(b)})
    reader=original.range(0x551e60,0xfe);assert reader==modified.range(0x551e60,0xfe)
    result={'schema':1,'passed':True,'method':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'constructor':construct,'loads':loads,'executed_ranges':pins,'reader_full_range':{'start':'0x551e60','end':'0x551f5e','sha256':sha(reader)},'seams':['6F7810 allocation','stream virtual read+0 and mode+0C; virtual write+4 is a failing trap'],'limits':['Ordinary finite nonnegative world extents and positive representable resolution only.','Native reader551E60 checks serialized resolution/count against preconstructed layout. Writer5500F0 is not used as a loader.','Version0 invokes5507C0 to rebuild flags from world tile state; this branch remains unsupported in Rust rather than consuming nonexistent flag bytes.']}
    (OUT/'native-height-binding.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'constructors':len(construct),'loads':len(loads),'sha256':sha((OUT/'native-height-binding.json').read_bytes())}))
if __name__=='__main__':main()
