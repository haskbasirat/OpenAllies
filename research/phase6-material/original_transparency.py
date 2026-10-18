"""Original alpha-accumulator policy, depth keys, quicksort and reverse submission.

Execute unchanged x86. Only linked-list allocation and final geometry submission
are intercepted. Stop the flush after submissions and before list cleanup.
"""
from pathlib import Path
import sys,json,struct,random,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
RANGES={'queue':(0x896070,0x8960da),'append':(0x8960e0,0x896120),
        'depth_keys':(0x895ed0,0x896011),'pivot':(0x8da6a0,0x8da72b),
        'sort':(0x8da890,0x8da93d),'submit_until_cleanup':(0x8da9c0,0x8daa1b),
        'accumulator_constructor':(0x895df0,0x895e0a),'game_init':(0x71032e,0x71043a)}
for a,b in RANGES.values(): assert original.range(a,b-a)==modified.range(a,b-a)
f32=lambda x:struct.unpack('<f',struct.pack('<f',x))[0]
bits=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]

class AlphaMachine(Machine):
    CAMERA=0x2002000;OBJECTS=0x2003000;NODES=0x2007000
    ARRAY=0x2008000;KEYS=0x2009000;PROPS=0x200a000;ALPHA=0x200a200
    VT=0x200b000;POOL_VT=0x200b100;ALLOC=0x300f100;DRAW=0x300f200
    def __init__(self,pc=0x37f):
        super().__init__(pc); self.drawn=[];self.next_node=self.NODES
        self.u.hook_add(UC_HOOK_CODE,self.hook)
        self.w(self.VT+0x6c,self.DRAW);self.w(self.POOL_VT+4,self.ALLOC)
    def hook(self,u,a,size,user):
        if a not in (self.ALLOC,self.DRAW):return
        sp=u.reg_read(UC_X86_REG_ESP)
        if a==self.ALLOC:
            u.reg_write(UC_X86_REG_EAX,self.next_node);self.next_node+=16;args=0
        else:
            self.drawn.append((u.reg_read(UC_X86_REG_ECX)-self.OBJECTS)//256);args=4
        u.reg_write(UC_X86_REG_EIP,self.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4+args)
    def queued(self,flags,respect):
        self.reset(0xabb7b8);self.next_node=self.NODES
        self.u.mem_write(self.STATE+0x10,b'\0'*40)
        self.w(self.STATE+0x10,self.POOL_VT)
        self.u.mem_write(self.STATE+0x34,bytes([respect]))
        self.w(self.OBJECTS+0xc4,self.PROPS);self.w(self.PROPS+4,1);self.w(self.PROPS+8,self.ALPHA)
        self.u.mem_write(self.ALPHA+0x20,struct.pack('<H',flags))
        self.w(self.STACK+4,self.OBJECTS)
        self.u.emu_start(0x896070,self.STOP,count=1000)
        return bool(self.u.reg_read(UC_X86_REG_EAX)&255)
    def sort(self,centers,radii,direction,subtract):
        self.reset(0xabb7b8);self.drawn=[];n=len(centers)
        self.w(self.STATE+0xc,self.CAMERA);self.w(self.STATE+0x14,self.NODES if n else 0)
        self.w(self.STATE+0x1c,n);self.w(self.STATE+0x24,n)
        self.w(self.STATE+0x28,self.ARRAY);self.w(self.STATE+0x2c,self.KEYS)
        self.u.mem_write(self.STATE+0x35,bytes([subtract]))
        self.u.mem_write(self.CAMERA+0x114,struct.pack('<3f',*direction))
        for i,(center,radius) in enumerate(zip(centers,radii)):
            obj=self.OBJECTS+256*i;node=self.NODES+16*i
            self.w(obj,self.VT);self.u.mem_write(obj+0x28,struct.pack('<4f',*center,radius))
            self.w(node,self.NODES+16*(i+1)if i+1<n else 0);self.w(node+8,obj)
        self.u.emu_start(0x8da9c0,0x8daa1b,count=1000000)
        keys={((self.r(self.ARRAY+i*4)-self.OBJECTS)//256):self.r(self.KEYS+i*4)for i in range(n)}
        return [keys[i]for i in range(n)],self.drawn

def main():
    rng=random.Random(0xAA2004)
    inputs=[[],[[0,0,0]],[[0,0,0]]*2,[[0,0,0]]*7,
            [[0,0,float(x)]for x in range(15)],[[0,0,float(x)]for x in reversed(range(15))]]
    inputs.extend([[[rng.choice([-2,-1,0,0,0,1,2]),rng.uniform(-1000,1000),rng.uniform(-1000,1000)]for _ in range(n)]for n in range(2,38)])
    # Exact-depth ties and cancellation/rounding boundaries challenge both keys
    # and the nonstable native median-of-three partition algorithm.
    inputs.extend([[[1e6,1,-1e6],[1e6,2,-1e6],[-1e6,1,1e6],[0,0,0]],
                   [[x,0,0]for x in [0.,-0.,0.,-0.,1.,-1.]]])
    directions=[[0,0,1],[1,1,1],[0.1,-0.7,0.3],[-1,0,0]]
    rows=[]
    for precision,pc in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
        machine=AlphaMachine(pc)
        for centers in inputs:
            centers=[[f32(x)for x in c]for c in centers]
            radii=[f32((i%5)*0.25)for i in range(len(centers))]
            for direction in directions:
                direction=list(map(f32,direction))
                for subtract in [False,True]:
                    keys,order=machine.sort(centers,radii,direction,subtract)
                    rows.append({'precision':precision,'centers':centers,'radii':radii,'direction':direction,
                                 'subtract_radius':subtract,'key_bits':keys,'draw_order':order})
    machine=AlphaMachine()
    policy=[{'flags':flags,'respect_no_sort':respect,'queued':machine.queued(flags,respect)}
            for flags in [0,1,236,237,193,4333,4844,4845,8192,8193,0xffff]for respect in [False,True]]
    sha=lambda b:hashlib.sha256(b).hexdigest()
    report={'schema':1,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'method':__doc__,
            'ranges':{name:{'start':hex(a),'end':hex(b),'sha256':sha(original.range(a,b-a))}for name,(a,b)in RANGES.items()},
            'policy':policy,'sort':rows}
    output=HERE/'native-transparency-vectors.json';output.write_text(json.dumps(report,separators=(',',':'))+'\n')
    dis=Cs(CS_ARCH_X86,CS_MODE_32)
    (HERE/'native-transparency-assembly.txt').write_text('\n\n'.join(name+'\n'+'\n'.join(f'{i.address:08x} {i.mnemonic} {i.op_str}'for i in dis.disasm(original.range(a,b-a),a))for name,(a,b)in RANGES.items()))
    print(json.dumps({'sort_cases':len(rows),'policy_cases':len(policy),'sha256':sha(output.read_bytes())}))

if __name__=='__main__':main()
