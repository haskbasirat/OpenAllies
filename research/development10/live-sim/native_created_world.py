"""Actual76AA90 entry through76AB1F (before second diagnostic/RNG/Init).
Fresh constructor-null Sim and successful World constructor result are explicit
typed-object inputs. Allocation/World constructor/diagnostic callbacks are
named seams. Error/null rows delimit unsupported producer domains, never imply
that a missing World became a successful loaded map.
"""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from unicorn.x86_const import UC_X86_REG_ECX
ORIGINAL,V14=h.P,h.V
class Machine(h.Machine):
    def __init__(self,stop,pc):
        super().__init__(b'',pc);self.stop=stop;self.events=[]
        self.sim=self.alloc(0x48);self.w(self.sim+0x34,0);self.w(self.sim+0x3c,0)
        self.settings=self.alloc(0xb4);self.creator=self.alloc(0x54);self.world=0
        def diagnostic(m):
            m.events.append('diagnostic')
            if stop=='diagnostic':m.outcome='diagnostic';raise h.Throw()
            m.ret(0)
        self.seam(0x404f60,diagnostic)
        def allocate(m):
            assert m.arg(0)==0x29c;assert m.r(m.sim+0x34)==0;m.events.append('allocate')
            if stop=='allocate':m.outcome='allocate';raise h.Throw()
            m.world=0 if stop=='null_allocation' else m.alloc(0x29c)
            m.ret(0,m.world)
        self.seam(0x6f7810,allocate)
        def constructor(m):
            assert m.u.reg_read(UC_X86_REG_ECX)==m.world;assert m.arg(0)==m.settings
            m.events.append('world_constructor')
            if stop=='world_constructor':m.outcome='world_constructor';raise h.Throw()
            m.ret(4,m.world)
        self.seam(0x4f7d00,constructor)
        def cutoff(m):m.outcome='post_store';raise h.Throw()
        self.probes[0x76ab1f]=cutoff
    def run(self):
        self.call(0x76aa90,self.sim,[self.creator,self.settings,0x12345678])
        value=self.r(self.sim+0x3c)
        return dict(outcome=self.outcome,field34=self.r(self.sim+0x34),world='new' if self.world and value==self.world else value,events=self.events)
def main():
    rows=[]
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.P=image
        for pc in h.CW:
            for stop in ['post_store','diagnostic','allocate','world_constructor','null_allocation']:
                n=Machine(stop,pc).run();assert n['field34']==0
                assert n['world']==('new' if stop=='post_store' else 0),n
                rows.append(dict(build=build,precision=pc,input=dict(stop=stop),native=n))
    h.P=ORIGINAL
    excluded={0x404f60,0x6f7810,0x4f7d00,0x76ab1f}
    pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a not in excluded}
    for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
    out=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),machine_sha256=h.H(Path(h.__file__).read_bytes()),rows=rows,pins=pins)
    p=HERE/'native-created-world.json'
    if '--write' in sys.argv:p.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(p.read_text()),'created World replay differs'
    print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=h.H(p.read_bytes()))))
if __name__=='__main__':main()
