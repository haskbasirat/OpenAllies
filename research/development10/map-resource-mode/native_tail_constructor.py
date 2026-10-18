"""Original/v14 World ctor4F7D00 prefix through4F8048 establishes actual
EBX=0 and the direct camera22C/230 and started274 stores. Other embedded
constructor services are explicit bounded nonalias seams. Not full GWorld.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
ORIGINAL,V14=h.P,h.V
class Machine(h.Machine):
    def __init__(self,fill,pc):
        super().__init__(b'',pc);self.world=self.alloc(0x2a0,fill)
        for a in [0x511df0,0x4ee0a0,0x85d870]:self.seam(a,lambda m:m.ret(0))
        for a in [0x50aba0,0x50b3a0]:self.seam(a,lambda m:m.ret(4))
        def cutoff(m):m.outcome='ConstructedTailFields';raise h.Throw()
        self.probes[0x4f8048]=cutoff
    def run(self):
        try:self.call(0x4f7d00,self.world,[0x2468ace0])
        except h.Throw:pass
        return dict(outcome=self.outcome,look_at=[self.r(self.world+0x22c),self.r(self.world+0x230)],started_274=self.b(self.world+0x274),global_same=self.r(0xb36534)==self.world,settings_same=self.r(self.world+0x28)==0x2468ace0)
def main():
    rows=[]
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.P=image
        for pc in h.CW:
            for fill in [0,0xa5,0xff]:rows.append(dict(build=build,precision=pc,initial_fill=fill,native=Machine(fill,pc).run()))
    h.P=ORIGINAL
    excluded={0x511df0,0x4ee0a0,0x85d870,0x50aba0,0x50b3a0,0x6f7810,0x4f8048}
    pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a not in excluded}
    for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
    out=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),machine_sha256=h.H(Path(h.__file__).read_bytes()),rows=rows,pins=pins,
       seams=['511DF0 VWorldState,4EE0A0 Sun,85D870 embedded constructors supplied nonalias returns; their own fields are outside this prefix proof',
              '50ABA0/50B3A0 embedded scheduler constructors supplied bounded nonalias returns',
              '6F7810 shared bump allocator (A5 fill) ignores reuse/failure; actual explicit field stores execute',
              'cutoff4F8048 after direct274/275 stores; no full World constructor or subsequent Load success'])
    p=HERE/'native-tail-constructor.json'
    if '--write' in sys.argv:p.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(p.read_text()),'native tail constructor replay differs'
    print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=h.H(p.read_bytes()))))
if __name__=='__main__':main()
