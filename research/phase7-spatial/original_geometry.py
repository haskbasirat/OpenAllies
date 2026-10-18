"""Original placed-shape bounds, inclusive tile extents and blocker predicate."""
from pathlib import Path
import json,random,struct,sys,hashlib
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
H=lambda b:hashlib.sha256(b).hexdigest()
bits=lambda n:struct.unpack('<I',struct.pack('<f',n))[0]
W=lambda *v:struct.pack('<'+'I'*len(v),*v)
class Geometry(Machine):
    PLACEMENT=0x2003000;SHAPE=0x2004000;OUT=0x2005000
    def __init__(self,cw):
        super().__init__(cw);self.visited={};self.u.hook_add(UC_HOOK_CODE,self.hook)
    def hook(self,u,pc,size,_):
        if original.base<=pc<original.base+original.image_size:self.visited[pc]=size
    def prepare(self,grid,p):
        self.reset(0);s=p['shape'];self.u.mem_write(self.SHAPE,W(s['kind'],s['radius_bits'],*s['bounds']))
        self.u.mem_write(self.PLACEMENT,W(self.SHAPE,*p['facing'],*p['position'],0))
        self.w(self.STATE+4,grid['coordinate_shift']);self.w(self.STATE+8,grid['cell_size_bits'])
        self.w(self.STATE+28,grid['dimensions'][0]);self.w(self.STATE+32,grid['dimensions'][1])
    def evaluate(self,grid,p):
        self.prepare(grid,p);self.w(self.STACK+4,self.OUT);self.w(self.STACK+8,self.PLACEMENT)
        self.u.emu_start(0x5534f0,self.STOP,count=100000);assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+12
        rect=list(struct.unpack('<4i',self.u.mem_read(self.OUT,16)));orientation=self.r(self.PLACEMENT+20)
        self.prepare(grid,p);self.w(self.STACK+4,self.OUT);self.u.reg_write(UC_X86_REG_ECX,self.PLACEMENT)
        self.u.emu_start(0x46e540,self.STOP,count=100000);assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
        bounds=list(struct.unpack('<4I',self.u.mem_read(self.OUT,16)))
        return {'rect':{'min':rect[:2],'max':rect[2:]},'orientation_cache':orientation,'bounds':bounds}

def main():
    rng=random.Random(0x5534f0);cases=[];windows={};inputs=[]
    facings=[[0.,-1.],[0.,1.],[1.,0.],[-1.,0.],[0.6,0.8],[-0.70710677,0.70710677],[1e-5,-1.],[1e-4,-1.],[0.,0.]]
    for kind in range(4):
        for facing in facings:
            for position in [[0.,0.],[1.9999,2.],[19.75,23.9],[-3.2,60.]]:
                inputs.append({'shape':{'kind':kind,'radius_bits':bits(0. if kind==0 else 1.125),
                    'bounds':list(map(bits,[-2.75,-1.125,2.75,1.125]))},'facing':list(map(bits,facing)),'position':list(map(bits,position))})
    for _ in range(120):
        hx,hy=rng.uniform(0,8),rng.uniform(0,8)
        inputs.append({'shape':{'kind':rng.randrange(4),'radius_bits':bits(rng.uniform(0,8)),
            'bounds':list(map(bits,[-hx,-hy,hx,hy]))},'facing':list(map(bits,[rng.uniform(-1,1),rng.uniform(-1,1)])),
            'position':list(map(bits,[rng.uniform(-10,30),rng.uniform(-10,40)]))})
    grid={'dimensions':[10,12],'coordinate_shift':1,'cell_size_bits':bits(2.)}
    for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
        m=Geometry(cw)
        for p in inputs:cases.append({'grid':grid,'placement':p,'precision':pc,'native':m.evaluate(grid,p)})
        windows.update(m.visited)
    pins=[]
    for a,n in sorted(windows.items()):
        raw=original.range(a,n);assert raw==modified.range(a,n),hex(a)
        pins.append({'va':hex(a),'bytes':raw.hex()})
    report={'schema':1,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(original.b),'v14_sha256':H(modified.b),
        'scope':'Original5534F0/552550 and46E540/46DAB0 with native CRT integer conversion and exact x87 precision. No decision hooks.',
        'cases':cases,'executed_windows':pins}
    path=HERE/'native-geometry.json';path.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'cases':len(cases),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
