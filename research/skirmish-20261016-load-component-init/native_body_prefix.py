"""Original562290 Body Init healthy/construction skip and required-child prefixes.

Explicit finite field probes, actual vtable, native565070 and4CF110 bodies.
561D40/561E50 stop at entry; never provide fake health/modifier effects.
Default mode immutably replays the fixture; --write creates it before freeze.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_MEM_WRITE

H=lambda b:hashlib.sha256(b).hexdigest()
VT=0xa3b0e8
class Required(Exception):pass

class Machine(h.Machine):
    def __init__(self,c,image):
        super().__init__(c['precision']);self.c=c
        if image is not h.P:
            a=(h.P.image_size+0xfff)&~0xfff;b=(image.image_size+0xfff)&~0xfff
            if b>a:self.u.mem_map(image.base+a,b-a)
            for vs,r,raw,off in image.sections:
                if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
        self.actor=self.alloc(0x200);self.body=self.alloc(0x38)
        self.w32(self.actor,0xa273f0);self.w32(self.actor+0x100,c['state_low'])
        self.w32(self.body,VT,self.actor,0xa3b0c0)
        self.w32(self.body+0x10,c['health_bits'],c['maximum_bits'])
        self.w32(self.actor+0x60,self.body)
        self.target=self.alloc(0x20)
        if c['construction'] is not None:
            construction=self.alloc(0x2c);self.w32(construction+4,self.actor)
            self.w32(construction+0x28,self.target if c['construction'] else 0)
            self.w32(self.actor+0x9c,construction)
        self.properties=self.alloc(0xdc) if c['properties'] else 0
        self.w32(self.actor+0x78,self.properties)
        self.writes=[];self.boundary=None
        self.u.hook_add(UC_HOOK_MEM_WRITE,self.write,begin=self.body,end=self.body+0x37)
        self.seams[0x561d40]=self.maximum_required
        self.seams[0x561e50]=self.health_required
    def write(self,u,access,address,size,value,_):
        assert address==self.body+0x10 and size==4
        self.writes.append({'offset':0x10,'before':self.r32(address),'after':value&0xffffffff})
    def maximum_required(self,_):
        assert self.ecx()==self.body
        assert self.arg(0)==(self.properties+0x1c if self.properties else 0)
        assert self.arg(1)==self.target and self.arg(2)==1
        self.boundary={'address':'561D40','modifier_context_present':bool(self.properties),
            'construction_target_present':True,'preserve_ratio':1}
        raise Required()
    def health_required(self,_):
        assert self.ecx()==self.body
        assert self.r32(self.esp()+4,4)==[self.c['health_bits'],0,1,0]
        self.boundary={'address':'561E50','requested_bits':self.arg(0),'token':0,'notify':1,'source':0}
        raise Required()
    def run(self,image):
        assert image.u32(VT+8)==0x562290 and image.u32(VT+0x14)==0x4b2830
        try:
            value=self.call(0x562290,self.body)&255
            assert value==1 and self.final_esp==h.STACK+4 and self.fpu_top()==0
            outcome='complete'
            before=list(self.writes)
            self.call(0x4b2830,self.body)
            assert self.writes==before
        except Required:
            outcome='required_'+self.boundary['address']
        assert self.r32(self.actor+0x100)==self.c['state_low']
        return {'outcome':outcome,'health_bits':self.r32(self.body+0x10),
            'maximum_bits':self.r32(self.body+0x14),'writes':self.writes,'boundary':self.boundary}

def main():
    rows=[]
    for pc,health,state,construction,properties in itertools.product(
        h.PRECISION,[100.,50.,0.],[0,1<<21,1<<29,(1<<21)|(1<<29)],
        [None,False,True],[False,True]):
        c={'precision':pc,'health_bits':h.f2b(health),'maximum_bits':h.f2b(100.),
            'state_low':state,'construction':construction,'properties':properties}
        result=[Machine(c,image).run(image) for image in (h.P,h.V)]
        assert result[0]==result[1]
        rows.append({'scope':'explicit finite owned-field/native leaf probe; not a whole actor initialization',
            'input':c,'native':result[0]})
    for address in (0x562290,0x565070,0x4cf110,0x561d40,0x561e50,0x4b2830):assert address in h.VISITED,hex(address)
    pins=[{'va':hex(a),'bytes':b} for a,b in sorted(h.VISITED.items())]
    doc={'schema':1,'scope':'Actual Body562290, Construction565070 and modifier-context4CF110 over finite explicit fields. Complete construction-skip/healthy paths; required561D40/561E50 entries stop before child effects. Damaged prefix retains health=max store before561E50. No full health service or wholeActorInit claim.',
        'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
        'fresh_original_v14_machines':len(rows)*2,'rows':rows,'address_byte_pins':pins,
        'required_entry_seams':['561D40','561E50'],
        'vtable_byte_pins':[{'va':hex(VT+off),'bytes':h.P.range(VT+off,4).hex()} for off in (8,0x14)]}
    path=HERE/'native-body-prefix.json';raw=(json.dumps(doc,indent=1)+'\n').encode()
    if '--write' in sys.argv:path.write_bytes(raw)
    else:assert json.loads(path.read_text())==doc
    from collections import Counter
    print(json.dumps({'rows':len(rows),'fresh_original_v14_machines':len(rows)*2,
        'outcomes':dict(Counter(r['native']['outcome'] for r in rows)),
        'address_byte_pins':len(pins),'sha256':H(raw),'immutable':'--write' not in sys.argv}))
if __name__=='__main__':main()
