"""Native76F240 scalar constructor and76DCE0 reset. Sync constructors,
CString/callback registration, allocation and diagnostics are explicit seams.
Time words are observed from nonzero fresh allocations, never assumed zero.
Finite positive rate and finite reset inputs; no full Node/Session receipt.
"""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from unicorn.x86_const import UC_X86_REG_ECX
ORIGINAL,V14=h.P,h.V
class Constructor(h.Machine):
    def __init__(self,c,pc):
        super().__init__(b'',pc);self.c=c;self.events=[]
        self.sim=self.alloc(0x48,c['fill'])
        svar=self.alloc(0x200);rate=self.alloc(4)
        self.w(rate,c['rate']);self.w(svar+0x20,rate);self.w(0xb39ab0,svar)
        def callback(m):m.events.append('register_callback');m.ret(4,m.u.reg_read(UC_X86_REG_ECX))
        self.seam(0x8acd80,callback)
        def sync(m):
            assert m.arg(0)==0x10000;m.events.append('construct_sync')
            m.ret(4,m.u.reg_read(UC_X86_REG_ECX))
        self.seam(0x77ccb0,sync)
    def run(self):
        self.call(0x76f240,self.sim,[])
        return dict(outcome=self.outcome,paused=self.b(self.sim+0x28),frozen=self.b(self.sim+0x29),
            world=self.r(self.sim+0x3c),field34=self.r(self.sim+0x34),avi=self.r(self.sim+0x38),
            speed=self.r(self.sim+0x20),maximum=self.r(self.sim+0x24),rate=self.r(self.sim+0x2c),
            times=[self.r(self.sim+off) for off in [0x10,0x14,0x18,0x1c]],events=self.events)
class Reset(h.Machine):
    def __init__(self,c,pc):
        super().__init__(b'',pc);self.c=c;self.sim=self.alloc(0x48,0xa5)
        self.world=self.alloc(0x2a0);self.w(self.world+0xe0,c['world_begin']);self.w(0xb36534,self.world)
    def run(self):
        self.call(0x76dce0,self.sim,[self.c['admin']])
        return dict(outcome=self.outcome,times=[self.r(self.sim+off) for off in [0x10,0x14,0x18,0x1c]],
            maximum=self.r(self.sim+0x24),frozen=self.b(self.sim+0x29))
def main():
    ctors=[];resets=[]
    zero_tails=[]
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.P=image
        for vs,r,size,off in image.sections:
            word=0xb40904-image.base-r
            if 0<=word<vs:
                assert size<=word and word+8<=vs
                zero_tails.append(dict(build=build,section_rva=r,virtual_size=vs,raw_size=size,word_offset=word,addresses=[0xb40904,0xb40908],initial_words=[0,0]))
        for pc in h.CW:
            for fill in [0,0xa5,0xff]:
                for rate in [0x41800000,0x40400000,0x40e00000,0x3eaaaaab]:
                    c=dict(fill=fill,rate=rate);n=Constructor(c,pc).run()
                    assert n['outcome'] is None,n
                    assert n['times']==[int.from_bytes(bytes([fill])*4,'little')]*4,n
                    assert (n['paused'],n['frozen'],n['world'],n['field34'],n['avi'],n['speed'],n['maximum'])==(0,1,0,0,0,0,0x42800000)
                    ctors.append(dict(build=build,precision=pc,input=c,native=n))
            for admin,world in [(0,0),(0x3f800000,0x40000000),(0x3eaaaaab,0x80000000),(0x4b800001,0x3f800000),(0xbf000000,0x41200000)]:
                c=dict(admin=admin,world_begin=world);n=Reset(c,pc).run();assert n['outcome'] is None,n
                resets.append(dict(build=build,precision=pc,input=c,native=n))
    h.P=ORIGINAL
    excluded={0x6f7810,0x6f7430,0x856370,0x401b00,0x8acd80,0x77ccb0}
    pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a not in excluded}
    for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
    assert len(zero_tails)==2
    out=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),machine_sha256=h.H(Path(h.__file__).read_bytes()),constructors=ctors,resets=resets,pins=pins,process_counter_zero_tail=zero_tails)
    p=HERE/'native-live-sim.json'
    if '--write' in sys.argv:p.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(p.read_text()),'native Sim replay differs'
    print(json.dumps(dict(constructors=len(ctors),resets=len(resets),pins=len(pins),sha256=h.H(p.read_bytes()))))
if __name__=='__main__':main()
