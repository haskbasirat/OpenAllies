"""Actual76B780 prefix with explicit external memory-read intervention at
76B7A6 EndAdmin read. This is an observation boundary, not an application
callback or native C++ exception claim. It proves both End words are read
before either Begin word is stored. Original/v14 atPC24/53/64.
"""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-tick'))
import native_tick_driver as h
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_EIP
ORIGINAL,V14=h.NATIVE,h.ACCEPTED
PINS={}
class Stop(Exception):pass
class Machine(h.Machine):
    def __init__(self,pc):
        super().__init__(pc)
        self.set_sim(dict(begin_game=h.bits(5),begin_admin=h.bits(6),end_game=h.bits(10),end_admin=h.bits(20),frozen=0,paused=0))
        self.u.hook_add(UC_HOOK_MEM_READ,self.read_boundary)
    def hook(self,u,pc,size,data):
        if ORIGINAL.base<=pc<ORIGINAL.base+ORIGINAL.image_size:PINS[pc]=bytes(u.mem_read(pc,size)).hex()
        super().hook(u,pc,size,data)
    def read_boundary(self,u,access,address,size,value,data):
        if address==self.sim+0x1c and self.u.reg_read(UC_X86_REG_EIP)==0x76b7a6:raise Stop()
    def run(self):
        before=self.sim_state()
        try:self.call(0x76b780,self.sim,[h.bits(21)])
        except Stop:pass
        else:raise AssertionError('required read boundary not reached')
        after=self.sim_state();assert after==before
        return dict(before=before,after=after,events=self.events,boundary=hex(self.u.reg_read(UC_X86_REG_EIP)))
def main():
    rows=[]
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.NATIVE=image
        for pc in [24,53,64]:rows.append(dict(build=build,precision=pc,native=Machine(pc).run()))
    h.NATIVE=ORIGINAL
    pins={hex(a):v for a,v in sorted(PINS.items()) if a not in {0x77d9d0}}
    for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
    out=dict(schema=1,scope=__doc__,producer_sha256=h.digest(Path(__file__).read_bytes()),machine_sha256=h.digest(Path(h.__file__).read_bytes()),rows=rows,pins=pins)
    p=HERE/'native-admin-end-read-prefix.json'
    if '--write' in sys.argv:p.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(p.read_text()),'read-prefix replay differs'
    print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=h.digest(p.read_bytes()))))
if __name__=='__main__':main()
