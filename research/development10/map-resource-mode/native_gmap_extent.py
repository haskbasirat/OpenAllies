"""Actual556638..55666B GMap constructor integer->binary32 extent stores.
The earlier native ceil/conversion producer is existing TerrainGridLayout;
these inputs are its explicit signed integer results, not grid cell counts.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from unicorn.x86_const import *
ORIGINAL,V14=h.P,h.V
class Machine(h.Machine):
    def __init__(self,c,pc):
        super().__init__(b'',pc);self.map=self.alloc(0x30,0xa5);self.c=c
        self.w(self.map+0x14,c[0]);self.w(self.map+4,2)
        def cutoff(m):raise h.Throw()
        self.probes[0x55666b]=cutoff
    def run(self):
        self.u.reg_write(UC_X86_REG_ESP,h.STACK);self.u.reg_write(UC_X86_REG_ESI,self.map);self.u.reg_write(UC_X86_REG_EAX,self.c[1])
        try:self.u.emu_start(0x556638,h.STOP,count=1000)
        except h.Throw:pass
        return dict(map_extent_bits=[self.r(self.map+0xc),self.r(self.map+0x10)],grid_cells=[self.r(self.map+0x1c),self.r(self.map+0x20)])
def main():
    rows=[]
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.P=image
        for pc in h.CW:
            for c in [[0,1],[640,640],[1000,1001],[16777217,16777219],[2147483647,12345]]:
                rows.append(dict(build=build,precision=pc,input_extents=c,native=Machine(c,pc).run()))
    h.P=ORIGINAL
    pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a!=0x55666b}
    for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
    out=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),machine_sha256=h.H(Path(h.__file__).read_bytes()),rows=rows,pins=pins,seams=['supplied containing frame/ESI and integer results before556638; stop55666B before logging/allocation; no whole GMap constructor or positive-grid-domain claim'])
    p=HERE/'native-gmap-extent.json'
    if '--write' in sys.argv:p.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(p.read_text()),'native GMap extent replay differs'
    print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=h.H(p.read_bytes()))))
if __name__=='__main__':main()
