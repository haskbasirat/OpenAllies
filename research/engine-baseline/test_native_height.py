"""Bounded native height-query differential fixtures; no game process is opened."""
from audit_baseline import HERE, RECON, PE, pin, sha
import json, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP

BASE=RECON/'research/native-aa/imported-0.bin'
def f32(v): return struct.unpack('<f',struct.pack('<f',v))[0]

def run(x,y,types,flags=0x122,base_value=10.,mountain_value=3.):
    pe=PE(BASE); u=Uc(UC_ARCH_X86,UC_MODE_32)
    u.mem_map(pe.base,(pe.image_size+4095)&~4095)
    for vs,r,size,off in pe.sections: u.mem_write(pe.base+r,pe.b[off:off+size])
    u.mem_map(0x20000000,0x20000);u.mem_map(0x21000000,0x10000)
    def i(a,v):u.mem_write(a,struct.pack('<I',v))
    def f(a,v):u.mem_write(a,struct.pack('<f',v))
    world=0x20000000;terrain=world+0x1000;base=world+0x2000;mountain=world+0x3000
    tiles=world+0x4000;variations=world+0x6000;defs=world+0x7000
    i(world+0x30,terrain);i(world+0x34,base);i(world+0x38,mountain)
    i(terrain+4,1);i(terrain+0x1c,4);i(terrain+0x20,4);i(terrain+0x28,tiles)
    for index,typ in enumerate(types):
        variation=variations+index*0x30;definition=defs+index*0x100
        i(tiles+index*0x30,variation);i(variation+0x1c,definition);i(definition+0x1c,typ)
        f(definition+0x68,.25);f(definition+0x6c,.75)
    for h,v in [(base,base_value),(mountain,mountain_value)]:
        i(h+0xc,5);i(h+0x10,5);i(h+0x14,1);f(h+0x18,2);i(h+0x1c,1);i(h+0x40,h+0x100)
        for q in range(25):f(h+0x100+q*4,v if not isinstance(v,list) else v[q])
    xy=world+0x9000;result=xy+8;code=world+0xa000
    f(xy,x);f(xy+4,y)
    # Execute unchanged CRT sampling, complete GWorld query, then capture x87 result.
    b=b'\xDB\xE3\xB9'+struct.pack('<I',world)+b'\x68'+struct.pack('<I',flags)+b'\x68'+struct.pack('<I',xy)
    call=code+len(b);b+=b'\xE8'+struct.pack('<i',0x4f0e50-call-5)
    b+=b'\xD9\x1D'+struct.pack('<I',result)
    u.mem_write(code,b);u.reg_write(UC_X86_REG_ESP,0x2100fff0)
    u.emu_start(code,code+len(b),count=10000)
    assert u.reg_read(UC_X86_REG_ESP)==0x2100fff0
    return struct.unpack('<f',u.mem_read(result,4))[0]

def main():
    cases=[]
    def check(name,types,expected,**kwargs):
        actual=run(2,2,types,**kwargs)
        assert abs(actual-expected)<2e-5,(name,actual,expected)
        cases.append({'name':name,'actual':actual,'expected':expected})
    t=[0]*16
    check('land ignores nonzero mountain map',t,10)
    t=[0]*16;t[5]=2;check('current WATER includes mountain map without noise',t,13)
    t=[0]*16;t[5]=1
    noise=.25+.5*(((5*4723+6323)%6997)%1000)*f32(.001)
    check('current MOUNTAIN includes deterministic authored noise',t,f32(13+noise))
    for at in [0,1,2,4,6,8,9,10]:
        t=[0]*16;t[at]=1;check(f'neighbor {at} enables mountain without neighbor noise',t,13)
    t=[0]*16;t[15]=1;check('distant mountain ignored',t,10)
    t=[0]*16;t[5]=1;check('query without bit2 ignores mountain and noise',t,10,flags=0x120)
    t=[0]*16;t[5]=2;check('bit1 lower clamp',t,9.5,flags=0x123,mountain_value=-3)
    check('bit1+bit4 special lower surface',t,9.5,flags=0x127)
    ramp=[float(x+10*y) for y in range(5) for x in range(5)]
    for x,y,expected in [(0,0,0),(2,0,1),(0,2,10),(8,8,44),(1,1,5.5),(3,5,26.5)]:
        actual=run(x,y,[0]*16,base_value=ramp)
        assert abs(actual-expected)<1e-5,(x,y,actual,expected)
        cases.append({'name':f'actual native row order/interpolation at {x},{y}','actual':actual,'expected':expected})
    report={'passed':True,'cases':cases,'case_count':len(cases),'script':pin(__import__('pathlib').Path(__file__)),
            'binary':pin(BASE),'scope':'Unchanged x86 GWorld height query, actual HeightMap samplers and CRT; controlled 4x4 terrain/5x5 height fixtures. No function mocks, no live process, no rendering/simulation tests.',
            'limits':['Finite positive in-map coordinates and stock2-unit spacing only; no exhaustive x87/boundary equivalence claim.']}
    (HERE/'native-height-tests.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'passed':True,'cases':len(cases)}))

if __name__=='__main__':main()
