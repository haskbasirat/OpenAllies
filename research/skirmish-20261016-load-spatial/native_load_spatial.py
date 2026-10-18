"""Unmodified4EE4C0 + concrete5536B0 over captured actual first Kingdom fields.

No complete membership/geometry function is seamed. Only operator new/delete
is provided. Default mode is immutable replay; --write creates the fixture.
"""
from pathlib import Path
import hashlib, json, struct, sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP

H=lambda b:hashlib.sha256(b).hexdigest()
VTABLES={2:0xa2a2a0,3:0xa273f0,4:0xa299d8}

class Machine(h.Machine):
    def __init__(self,case,image):
        super().__init__(case['precision'])
        if image is not h.P:
            original_end=(h.P.image_size+0xfff)&~0xfff
            image_end=(image.image_size+0xfff)&~0xfff
            if image_end>original_end:self.u.mem_map(image.base+original_end,image_end-original_end)
            self.u.mem_write(image.base,image.b[:0x1000])
            for vs,r,raw,off in image.sections:
                if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
        self.case=case;self.trace=[];self.running=False
        m=case['member'];self.member=m
        self.obj=self.alloc(0x500);self.definition=self.alloc(0x500)
        self.world=self.alloc(0x100);self.map=self.alloc(0x100)
        self.ids=self.alloc(0x60010)
        self.grid=case['grid'];count=self.grid['dimensions'][0]*self.grid['dimensions'][1]
        self.tiles=self.alloc(count*48)
        self.w32(0xb36534,self.world);self.w32(self.world+0x30,self.map)
        self.w32(0xb364c0,self.ids)
        self.w32(self.map+4,self.grid['coordinate_shift'],self.grid['cell_size_bits'])
        self.w32(self.map+0x1c,*self.grid['dimensions']);self.w32(self.map+0x28,self.tiles)
        assert len(case['initial_flags'])==count
        for i,flag in enumerate(case['initial_flags']):self.w32(self.tiles+i*48+0x20,flag)
        self.w32(self.obj,VTABLES[case['native_class']],self.definition,m['actor']['flags'])
        self.w32(self.obj+0x14,m['reference'])
        self.w32(self.obj+0x20,*m['actor']['position'],m['height_bits'],*m['actor']['facing'],m['facing_z_bits'])
        self.w32(self.obj+0x44,*m['cached_tile'])
        self.w32(self.obj+0x90,0) # captured actual first Kingdom has no Wall
        assert m['wall_present'] is False
        for offset,name in [(0x38,'blocking'),(0x50,'soft'),(0x68,'target')]:
            s=m[name];self.w32(self.definition+offset,s['kind'],s['radius_bits'],*s['bounds'])
        # +14 shape selection does not run inside membership; keep explicit
        # override absent because the captured effective target equals soft.
        assert m['target']==m['soft']
        self.w8(self.definition+0x80,0)
        ref=m['reference'];self.u.mem_write(self.ids+4+(ref&65535)*2,struct.pack('<H',ref>>16))
        self.w32(self.ids+0x20004+(ref&65535)*4,self.obj)
        self.seams[0x6f7810]=self.malloc
        self.seams[0x6f7430]=lambda _:self.ret()
        self.u.hook_add(UC_HOOK_MEM_WRITE,self.written)
    def malloc(self,_):
        assert self.arg()==8,'only actual native list-node allocation admitted'
        self.ret(value=self.alloc(8))
    def cell(self,address):
        i=(address-self.tiles)//48
        return [i%self.grid['dimensions'][0],i//self.grid['dimensions'][0]]
    def written(self,u,access,address,size,value,_):
        if not self.running or size!=4:return
        if address==self.obj+8:
            self.trace.append({'action':'member_flags','actor':self.member['key'],'before':self.r32(address),'after':value&0xffffffff})
        elif self.tiles<=address<self.tiles+len(self.case['initial_flags'])*48:
            off=(address-self.tiles)%48
            if off in (0x1c,0x20):
                self.trace.append({'action':'primary' if off==0x1c else 'flags','cell':self.cell(address-off),'before':self.r32(address),'after':value&0xffffffff})
    def _hook(self,u,pc,size,user):
        if getattr(self,'running',False) and pc in (0x54f6f0,0x54f780):
            assert self.arg()==self.obj
            self.trace.append({'action':'insert' if pc==0x54f6f0 else 'remove','cell':self.cell(self.ecx()),'actor':self.member['key']})
        super()._hook(u,pc,size,user)
    def snapshot(self):
        out=[];flag_bytes=bytearray()
        for i in range(len(self.case['initial_flags'])):
            p=self.tiles+i*48;node=self.r32(p+8);members=[];last=0
            while node:
                assert self.r32(node)==self.obj
                members.append(self.member['key']);last=node;node=self.r32(node+4)
            assert self.r32(p+12)==last
            flag=self.r32(p+0x20);primary=self.r32(p+0x1c)
            flag_bytes.extend(struct.pack('<I',flag))
            if members or primary:out.append({'cell':self.cell(p),'actors':members,'flags20':flag,'primary_reference':primary})
        return {'tiles':{'occupied':out,'tile_count':len(self.case['initial_flags']),'flags20_le_sha256':H(flag_bytes)},
            'member_flags':{str(self.member['key']):self.r32(self.obj+8)},'trace':self.trace}
    def run(self):
        self.trace=[];self.running=True
        self.call(0x4ee4c0,self.obj)
        self.running=False
        assert self.final_esp==h.STACK+4
        assert self.fpu_top()==0
        first=self.snapshot()
        self.trace=[];self.running=True
        self.call(0x4ee4c0,self.obj)
        self.running=False
        second=self.snapshot()
        assert second['trace']==[]
        assert second['tiles']==first['tiles'] and second['member_flags']==first['member_flags']
        return {'first':first,'second':second}

def main():
    inp=HERE/'actual-first-kingdom.json';capture=json.loads(inp.read_text())
    cases=[]
    for actual in capture['cases']:
        for cls in (4,2,3):
            case=dict(actual);case['native_class']=cls
            scope='actual stopped first Kingdom' if cls==4 else 'explicit native ordinary-class vtable probe over captured fields'
            outputs=[Machine(case,image).run() for image in (h.P,h.V)]
            assert outputs[0]==outputs[1]
            if cls==4:assert outputs[0]['first']==actual['rust']
            cases.append({'precision':case['precision'],'native_class':cls,'scope':scope,'expected':outputs[0]})
    pins=[{'va':hex(a),'bytes':raw} for a,raw in sorted(h.VISITED.items())]
    seen=set(h.VISITED)
    for address in (0x4ee4c0,0x4cf210,0x4ea0f0,0x5536b0,0x5534f0,0x54f6f0,0x4ee700,0x6a71d0):
        assert address in seen,hex(address)
    tables=[]
    for cls,vt in VTABLES.items():
        slots={hex(off):hex(h.P.u32(vt+off)) for off in (4,12,16,20,0xb0,0xb4)}
        for off in (4,12,16,20,0xb0,0xb4):assert h.P.range(vt+off,4)==h.V.range(vt+off,4)
        tables.append({'native_class':cls,'va':hex(vt),'slots':slots})
    doc={'schema':1,'scope':'Actual partial tutorial02 first Kingdom4EE4C0 through concrete native5536B0/54F6F0 and present-bit second no-op; explicit class2/3 vtable probes. Fresh original/v14 execution at all three x87 PCs. Sole seams native operator new/delete. No full Init/battle readiness claim.',
        'producer_sha256':H(Path(__file__).read_bytes()),'input_sha256':H(inp.read_bytes()),
        'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'cases':cases,
        'address_byte_pins':pins,'vtables':tables,'seams':{'0x6f7810':'allocate actual8-byte native tile node','0x6f7430':'delete (not reached in insertion/no-op cases)'}}
    path=HERE/'native-load-spatial.json'
    raw=(json.dumps(doc,indent=1)+'\n').encode()
    if '--write' in sys.argv:path.write_bytes(raw)
    else:assert json.loads(path.read_text())==doc,'immutable native replay differs'
    print(json.dumps({'cases':len(cases),'fresh_original_v14_machines':len(cases)*2,
        'native_refresh_calls':len(cases)*4,'address_byte_pins':len(pins),'sha256':H(raw),'immutable':'--write' not in sys.argv}))
if __name__=='__main__':main()
