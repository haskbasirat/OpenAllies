"""Actual World ctor4F7D00 prefix (reused frozen harness) and World Init
tail4F2BE4..4F2C49, including actual539F30 thirteen-word constructor.
Tail's ESI/EBP/EBX/frame are declared entry inputs after the earlier Init
prefix. GameSpeed+278 read/store executes, using an explicit actual SVar cell.
Allocation/destructor/free are seams. Not full World Init or destructor proof.
"""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
sys.path.insert(0,str(ROOT/'research/development10/map-resource-mode'))
import native_machine as h
from native_tail_constructor import Machine as Constructor
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_ECX
ORIGINAL,V14=h.P,h.V
class Tail(h.Machine):
    def __init__(self,c,pc):
        super().__init__(b'',pc);self.c=c;self.events=[];self.new=0
        self.world=self.alloc(0x2a0);self.old=self.alloc(52) if c['old'] else 0
        self.w(self.world+4,self.old);self.u.mem_write(self.world+0x275,b'\x80')
        self.w(self.world+0x278,0x12345678)
        svar=self.alloc(0x200);cell=self.alloc(4);self.w(cell,c['speed'])
        self.w(0xb39ab0,svar);self.w(svar+0x114,cell)
        def enter(m):
            m.u.reg_write(UC_X86_REG_ESI,m.world);m.u.reg_write(UC_X86_REG_EBP,0)
            m.u.reg_write(UC_X86_REG_EBX,0xffffffff)
        self.probes[0x4f2be4]=enter
        def allocation(m):
            assert m.arg(0)==52;m.events.append(['allocate',52])
            if c['error']=='allocate':m.outcome='AllocationServiceError';raise h.Throw()
            m.new=0 if c['null'] else m.alloc(52)
            m.ret(0,m.new)
        self.seam(0x6f7810,allocation)
        def destroy(m):
            assert m.u.reg_read(UC_X86_REG_ECX)==m.old;m.events.append(['destroy_old'])
            if c['error']=='destroy':m.outcome='DestructorServiceError';raise h.Throw()
            m.ret(0)
        self.seam(0x53a780,destroy)
        def free(m):
            assert m.arg(0)==m.old;m.events.append(['free_old'])
            if c['error']=='free':m.outcome='FreeServiceError';raise h.Throw()
            m.ret(0)
        self.seam(0x6f7430,free)
        def cutoff(m):m.outcome='InitializedAuxiliaryPrefix';raise h.Throw()
        self.probes[0x4f2c49]=cutoff
    def run(self):
        self.call(0x4f2be4,0,[])
        pointer=self.r(self.world+4)
        return dict(outcome=self.outcome,fog_275=self.b(self.world+0x275),speed_278=self.r(self.world+0x278),
            pointer='new' if self.new and pointer==self.new else 'old' if self.old and pointer==self.old else pointer,
            new_words=[self.r(self.new+4*i) for i in range(13)] if self.new else None,events=self.events)
def main():
    constructors=[];rows=[]
    base=dict(old=False,null=False,error=None,speed=0x3f800000)
    cases=[('fresh',{}),('fresh-null',dict(null=True)),('fresh-error',dict(error='allocate')),
           ('replace',dict(old=True)),('replace-null',dict(old=True,null=True)),
           ('replace-dtor-error',dict(old=True,error='destroy')),('replace-free-error',dict(old=True,error='free')),
           ('raw-speed-word',dict(speed=0x80000000))]
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.P=image
        for pc in h.CW:
            for fill in [0,0xa5,0xff]:
                m=Constructor(fill,pc);m.run();n=dict(auxiliary_4=m.r(m.world+4),fog_275=m.b(m.world+0x275))
                assert n==dict(auxiliary_4=0,fog_275=0)
                constructors.append(dict(build=build,precision=pc,fill=fill,native=n))
            for name,change in cases:
                c=base|change;n=Tail(c,pc).run()
                assert n['fog_275']==0 and n['speed_278']==c['speed']
                if n['new_words'] is not None:assert n['new_words']==[0]*13
                if not c['old'] and not c['error']:assert n['pointer']==(0 if c['null'] else 'new')
                rows.append(dict(build=build,precision=pc,name=name,input=c,native=n))
    h.P=ORIGINAL
    excluded={0x511df0,0x4ee0a0,0x85d870,0x50aba0,0x50b3a0,0x6f7810,0x6f7430,0x53a780,0x4f8048,0x4f2c49}
    pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a not in excluded}
    for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
    out=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),
        machine_sha256=h.H(Path(h.__file__).read_bytes()),constructor_producer_sha256=h.H((ROOT/'research/development10/map-resource-mode/native_tail_constructor.py').read_bytes()),
        constructor_rows=constructors,rows=rows,pins=pins,
        production_domain='fresh constructor-null owner; positive Rust allocation; earlier GameSpeed writer must be real. Old-owner replacement/destructor remains unsupported.')
    p=HERE/'native-runtime-fields.json'
    if '--write' in sys.argv:p.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(p.read_text()),'runtime fields replay differs'
    print(json.dumps(dict(constructors=len(constructors),rows=len(rows),pins=len(pins),sha256=h.H(p.read_bytes()))))
if __name__=='__main__':main()
