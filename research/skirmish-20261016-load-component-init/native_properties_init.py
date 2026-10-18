"""Original/v14 Properties InitExisting6A71D0 and AfterInit4B2830.

Execute the actual component vtable leaves in separate images at every PC.
Opaque allocation bytes are deliberately poisoned and never read by these
constant bodies. No ctor, saved Load, nonconstant component, or full Init claim.
Default is immutable replay; --write creates the author's fixture.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE

H=lambda b:hashlib.sha256(b).hexdigest()
VT=0xa3e3dc

class Machine(h.Machine):
    def __init__(self,pc,image):
        super().__init__(pc)
        if image is not h.P:
            end=(h.P.image_size+0xfff)&~0xfff;other=(image.image_size+0xfff)&~0xfff
            if other>end:self.u.mem_map(image.base+end,other-end)
            self.u.mem_write(image.base,image.b[:0x1000])
            for vs,r,raw,off in image.sections:
                if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
        self.component=self.alloc(0xdc)
        self.u.mem_write(self.component,b'\xa5'*0xdc)
        self.w32(self.component,VT)
        self.reads=[];self.writes=[]
        self.u.hook_add(UC_HOOK_MEM_READ,self.read,begin=self.component,end=self.component+0xdb)
        self.u.hook_add(UC_HOOK_MEM_WRITE,self.write,begin=self.component,end=self.component+0xdb)
    def read(self,u,access,address,size,value,_):self.reads.append([address-self.component,size])
    def write(self,u,access,address,size,value,_):self.writes.append([address-self.component,size,value])
    def run(self,image):
        before=bytes(self.u.mem_read(self.component,0xdc))
        init=image.u32(VT+8);after=image.u32(VT+0x14)
        assert init==0x6a71d0 and after==0x4b2830
        result=self.call(init,self.component)&255
        assert result==1 and self.final_esp==h.STACK+4
        self.call(after,self.component)
        assert self.final_esp==h.STACK+4 and self.fpu_top()==0
        assert before==bytes(self.u.mem_read(self.component,0xdc))
        assert self.seams=={} and self.reads==[] and self.writes==[]
        return {'init_result':bool(result),'reads':self.reads,'writes':self.writes,'seams':[],
            'opaque_allocation_unchanged':True,'component_vtable':hex(VT),
            'init_leaf':hex(init),'after_leaf':hex(after)}

def main():
    rows=[]
    for pc in h.PRECISION:
        values=[Machine(pc,image).run(image) for image in (h.P,h.V)]
        assert values[0]==values[1]
        rows.append({'precision':pc,**values[0]})
    assert set(h.VISITED)=={0x6a71d0,0x6a71d2,0x4b2830}
    for off in (8,0x14):assert h.P.range(VT+off,4)==h.V.range(VT+off,4)
    doc={'schema':1,'scope':'Actual PropertiesComponent vtable+8/+14 constant bodies; opaque allocation bytes are poisoned/unread. No constructor/saved Load/whole Actor Init claim. No seams.',
        'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
        'fresh_original_v14_machines':6,'native_leaf_calls':12,'rows':rows,
        'executed_address_byte_pins':[{'va':hex(a),'bytes':b} for a,b in sorted(h.VISITED.items())],
        'vtable_byte_pins':[{'va':hex(VT+off),'bytes':h.P.range(VT+off,4).hex()} for off in (8,0x14)]}
    path=HERE/'native-properties-init.json';raw=(json.dumps(doc,indent=1)+'\n').encode()
    if '--write' in sys.argv:path.write_bytes(raw)
    else:assert json.loads(path.read_text())==doc
    print(json.dumps({'rows':3,'fresh_original_v14_machines':6,'native_leaf_calls':12,
        'executed_address_byte_pins':len(h.VISITED),'sha256':H(raw),'immutable':'--write' not in sys.argv}))
if __name__=='__main__':main()
