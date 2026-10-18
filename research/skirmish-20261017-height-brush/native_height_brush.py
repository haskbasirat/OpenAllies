"""Whole551140 same-endpoint mode0/1 through real raster, geometry, pending
records, slope propagation and550A10 stores. Real550940 constructor fields.
Only allocator/free transport and protected diagnostic presentation are seams;
Worldphase6 makes actual50C310 return without renderer callbacks. Explicit
synthetic source point-orientation, Terrain slopes and caller+1C are inputs.
No generic mode2/swept-brush or source-loader/renderer claim.
"""
from pathlib import Path
import sys,json,struct,itertools,copy
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_ECX
ORIGINAL,V14=h.P,h.V
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
class Machine(h.Machine):
    def __init__(self,c,pc):
        super().__init__(b'',pc);self.c=c;self.events=[]
        self.field=self.alloc(0x58,0xa5);self.call(0x550940,self.field,[])
        self.constructor=dict(round=self.b(self.field+0x20),dirty=[self.r(self.field+o)for o in [0x30,0x34,0x38,0x3c]])
        g=c['grid'];n=g['width']*g['height'];self.samples=self.alloc(n*4);flags=self.alloc(n)
        for i,v in enumerate(g['heights']):self.w(self.samples+i*4,v)
        self.u.mem_write(flags,bytes(g['flags']))
        for o,v in {0xc:g['width'],0x10:g['height'],0x14:g['shift'],0x18:g['cell'],0x1c:g['bias'],0x40:self.samples,0x44:flags,0x48:c['limits'][0],0x4c:c['limits'][1]}.items():self.w(self.field+o,v)
        self.world=self.alloc(0x134);self.w(0xb36534,self.world);self.w(self.world+0xd8,6)
        self.manager=self.alloc(8);self.w(self.world+0x130,self.manager);self.w(self.manager+4,0)
        m=self.alloc(0x100);self.w(self.world+0x30,m);self.w(m+4,g['shift']);self.w(m+0xc,bits((g['width']-1)*(1<<g['shift'])));self.w(m+0x10,bits((g['height']-1)*(1<<g['shift'])))
        self.w(m+0x1c,g['width']);self.w(m+0x20,g['height']);cells=self.alloc(n*0x30);self.w(m+0x28,cells)
        self.slope_fields={}
        for i in range(n):
            v=self.alloc(0x70);self.w(v+0x5c,c['slopes'][i]);variation=self.alloc(0x30);self.w(variation+0x1c,v);self.w(cells+i*0x30,variation);self.slope_fields[v+0x5c]=i
        self.orientation=self.alloc(0x80);source=self.alloc(16);self.w(0xb39990,self.orientation);self.w(self.orientation+0x64,source)
        for i,v in enumerate(c['orientation']):self.w(source+4+i*4,v)
        self.shape=self.alloc(24)
        for i,v in enumerate(c['shape']):self.w(self.shape+4*i,v)
        self.direction=self.alloc(8);self.start=self.alloc(8)
        for i,v in enumerate(c['facing']):self.w(self.direction+4*i,v)
        for i,v in enumerate(c['xy']):self.w(self.start+4*i,v)
        self.settings=self.alloc(0x24,0xa5)
        for i,v in enumerate(c['settings']):self.w(self.settings+i*4,v)
        self.wb(self.settings+0x14,c['protect']);self.wb(self.settings+0x15,c['smooth']);self.w(self.settings+0x18,c['strength']);self.w(self.settings+0x1c,c['falloff']);self.wb(self.settings+0x20,c['area'])
        def log(m):
            self.events.append(dict(kind='protected',cell=[m.arg(5),m.arg(6)],area=m.arg(7),requested=m.arg(8)))
            m.ret(0)
        self.seam(0x404f90,log)
        self.notifications=[]
        self.probes[0x50c310]=lambda m:self.notifications.append([m.arg(i)for i in range(3)])
        def reads(u,access,a,size,value,_):
            if a==self.orientation+0x64:self.events.append(dict(kind='prepare',value=c['orientation']))
            if a in self.slope_fields:self.events.append(dict(kind='slope',index=self.slope_fields[a],value=self.r(a)))
        self.u.hook_add(UC_HOOK_MEM_READ,reads)
    def run(self):
        result=self.call(0x551140,self.field,[self.shape,self.direction,self.start,self.start,self.settings],count=5000000)
        assert self.outcome is None,(self.outcome,self.null_accesses)
        assert not self.null_accesses
        g=self.c['grid'];n=g['width']*g['height']
        return dict(returned=bool(result&255),heights=[self.r(self.samples+i*4)for i in range(n)],dirty=[self.r(self.field+o)for o in [0x30,0x34,0x38,0x3c]],events=self.events,notifications=self.notifications)
def cases():
    g=dict(width=5,height=5,shift=1,cell=bits(2),bias=1,heights=[bits(float((i*7)%6))for i in range(25)],flags=[0]*25)
    base=dict(name='rectangle',grid=g,limits=[bits(-100),bits(100)],shape=[3,bits(2),bits(-1),bits(-1),bits(1),bits(1)],facing=[0,bits(1)],xy=[bits(4),bits(4)],settings=[0,bits(3.25),bits(.5),0xff7fffff,0x7f7fffff],protect=1,smooth=1,strength=bits(1),falloff=0,area=0,slopes=[bits(.35)]*25,orientation=[0,bits(1),0])
    rows=[]
    def add(name,**kw):
        c=copy.deepcopy(base);c.update(kw);c['name']=name;rows.append(c)
    add('rectangle')
    add('circle',shape=[1,bits(2.4),0,0,0,0])
    add('point',shape=[0,0,0,0,0,0])
    add('rotated',facing=[bits(.6),bits(.8)])
    add('fractional',xy=[bits(3.17),bits(2.41)])
    add('edge',xy=[bits(-.2),bits(6.25)])
    add('notify-extended-y',grid={**g,'heights':[0]+[bits(3.25)]*24},xy=[bits(4),bits(7.56111478805542)],shape=[3,bits(16),bits(-8),bits(-8),bits(8),bits(8)],settings=[0,bits(3.25),bits(100),0xff7fffff,0x7f7fffff])
    add('protected-raster',grid={**g,'flags':[1 if i==12 else 0 for i in range(25)]})
    add('protected-slope',grid={**g,'flags':[1 if i==0 else 0 for i in range(25)]})
    add('own-area',grid={**g,'flags':[7]*25},area=7)
    add('no-protection',grid={**g,'flags':[7]*25},protect=0)
    add('no-edge',smooth=0)
    add('half-strength',strength=bits(.5))
    add('flat-no-change',grid={**g,'heights':[bits(3.25)]*25})
    add('physical-clamp',limits=[bits(1),bits(2)])
    add('descriptor-clamp',settings=[0,bits(8),bits(.5),bits(-2),bits(2)])
    add('additive',settings=[1,bits(.75),bits(.5),0xff7fffff,0x7f7fffff])
    add('invalid-mode',settings=[3,bits(.75),bits(.5),0xff7fffff,0x7f7fffff])
    add('zero-slope-source',slopes=[0]*25)
    for v in [bits(-1),bits(.01),bits(2),bits(10),bits(1000),1086590007]:add('falloff-'+hex(v),falloff=v)
    return rows
def main():
    h.VISITED.clear();rows=[];construct=[]
    for pc,c in itertools.product(h.CW,cases()):
        pair=[]
        for pe in [ORIGINAL,V14]:
            h.P=pe;m=Machine(c,pc);pair.append(m.run());construct.append(m.constructor)
        assert pair[0]==pair[1]
        rows.append(dict(precision=pc,input=c,native=pair[0]))
    h.P=ORIGINAL
    assert all(c==dict(round=0,dirty=[0,0,0,0])for c in construct)
    excluded={0x6f7810,0x6f7430,0x404f90}
    pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in excluded}
    out=dict(schema=1,scope=__doc__,rows=rows,executions=len(rows)*2,pins=pins,
        constructor_executions=len(construct),constructor_fields=dict(round=0,dirty=[0,0,0,0]),
        producer_sha256=h.H(Path(__file__).read_bytes()),machine_sha256=h.H(Path(h.__file__).read_bytes()),
        seams=['6F7810 A5 allocator and6F7430 free transport, no allocator failure/reuse parity','404F90 protected-area log presentation, recorded then return'],
        boundaries=['Mode2/swept-endpoint excluded','Worldphase6 executes real50C310 early return','Source fields are supplied explicit inputs; no source loader or actual map brush execution claimed'])
    path=HERE/'native-height-brush.json'
    if '--write' in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(path.read_text()),'immutable native brush replay differs'
    print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=h.H(path.read_bytes()))))
if __name__=='__main__':main()
