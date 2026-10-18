"""Actual529EC0/51D1B0 nonempty sync through native getters,4D64F0,4D7DA0.
Reached reset/state-effect/action children stop at their real entries; no mock
success is returned. Default immutable replay. Fixtures are valid field probes,
plus four actual saved core/context projections, not whole native maps. Every
reached actual input is captured, including Organization presence, Actor+118
and all embedded-state statuses. Unread absent Actor+118 is poisoned, not 0.
"""
from pathlib import Path
import hashlib,json,struct,sys,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import UC_X86_REG_EIP
REG=ROOT/'research/phase7-actors/native-actor-states.json';registration=json.loads(REG.read_text())['registration']
CT=ROOT/'research/phase7-mind/native-constructors.json';ctors=json.loads(CT.read_text());this=int(ctors['addresses']['this'],16)
TEMPLATES={}
for kind in ['element','regiment']:
    c=next(c for c in ctors['cases'] if c['input']['kind']==kind);raw=bytes.fromhex(c['output']['initialized_hex']);states={}
    for off in range(0x34,len(raw)-15,4):
        vt,parent,meta,status=struct.unpack_from('<IIII',raw,off)
        if parent==this and 0xa00000<vt<0xa60000 and 0xb30000<meta<0xb40000:states[off]=(vt,meta,status)
    TEMPLATES[kind]=(struct.unpack_from('<I',raw)[0],len(raw),states)
SUPPORTED={'element':{0x34,0x48,0x68,0x7c,0xf8,0x12c,0x164,0x178},'regiment':{0x34,0x54,0x120,0x1a8,0x1b8,0x1cc,0x1f0,0x20c,0x220,0x24c,0x270,0x2a4}}
REQUIRED={0x4d4a90:'required4D4A90',0x4d16b0:'required4D16B0',0x4d1350:'required4D1350',0x52d7c0:'requiredResetElement',0x51e3a0:'requiredResetRegiment'}
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(h.Machine):
    def __init__(self,pc,image,c):
        super().__init__(pc)
        if image is not h.P:
            old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
            if new>old:self.u.mem_map(image.base+old,new-old)
            self.u.mem_write(image.base,image.b[:0x1000])
            for vs,r,raw,off in image.sections:
                if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
        self.c=c;vt,size,states=TEMPLATES[c['kind']]
        self.actor=self.alloc(0x200);self.mind=self.alloc(size);self.node=self.alloc(8);self.org=self.alloc(0x30)
        self.u.mem_write(self.mind,b'\xcc'*size)
        self.w32(self.mind,vt);self.w32(self.mind+4,self.actor)
        self.w32(self.actor+8,c['flags']);self.w32(self.actor+0x100,c['bits']);self.w32(self.actor+0x104,c['bits']>>32)
        self.w32(self.actor+0x7c,self.org if c['organization'] else 0);self.w32(self.actor+0x118,c['word118'] if c['word118'] is not None else 0xcccccccc)
        for off,(svt,meta,status) in states.items():
            captured=c.get('state_statuses')
            value=captured[str(off)] if captured is not None else c['disable_status'] if off==0x34 else status
            self.w32(self.mind+off,svt,self.mind,meta,value)
        off=c['offset'];self.w32(self.mind+0x10,self.node if off is not None else 0);self.w32(self.mind+0x14,self.node if off is not None else 0)
        if off is not None:self.w32(self.node,self.mind+off,0)
        self.manager=self.alloc(0x400);self.table=self.alloc(42*4)
        self.w32(0xb36530,self.manager);self.w32(self.manager+0x2e4,self.table)
        for r in registration['records']:
            d=self.alloc(0x40);self.w32(self.table+r['index']*4,d);self.w32(d+0xc,r['index']);self.w32(d+0x2c,r['group']);self.w8(d+0x30,int(r['exclusive']))
        for i,m in enumerate(registration['masks']):self.w32(0xb3ac40+i*8,m);self.w32(0xb3ac44+i*8,m>>32)
        self.boundary=None;self.required=dict(REQUIRED)
        if off is not None and off not in SUPPORTED[c['kind']]:self.required[h.P.u32(states[off][0]+0x20)]='requiredStateVirtual20'
    def _hook(self,u,pc,size,_):
        if pc in self.required:
            h.VISITED[pc]=bytes(u.mem_read(pc,size)).hex();assert h.P.range(pc,size)==h.V.range(pc,size)
            self.boundary=self.required[pc];u.emu_stop();return
        super()._hook(u,pc,size,_)
    def run(self):
        start=0x529ec0 if self.c['kind']=='element' else 0x51d1b0
        try:self.call(start,self.mind)
        except AssertionError:
            if not self.boundary:raise
        pc=self.u.reg_read(UC_X86_REG_EIP);assert pc==h.STOP or pc in self.required,hex(pc)
        assert not self.seams
        return {'outcome':self.boundary or 'complete','bits':self.r32(self.actor+0x100)|(self.r32(self.actor+0x104)<<32),'flags':self.r32(self.actor+8),'stack_offset':None if not self.r32(self.mind+0x10) else self.r32(self.r32(self.mind+0x10))-self.mind}

def main():
    inputs=[]
    for kind in TEMPLATES:
        for off in [None,*TEMPLATES[kind][2]]:
            for bits,flags in [(0,0x100),(0x8001000,0x100),(0x8001000,0x80000100)]:
                for org,w118,disabled in [(False,0,0),(True,0,0),(True,1,2)]:
                    inputs.append({'label':'explicit_field_probe','kind':kind,'offset':off,'bits':bits,'flags':flags,'organization':org,'word118':w118,'disable_status':disabled})
    actual_path=HERE/'actual-before-init-v2.json';actual=json.loads(actual_path.read_text())
    for r in actual['rows']:
        inputs.append({'label':'actual_saved_projection_'+str(r['owner']),'kind':r['mind_kind'].lower(),'offset':r['stack_states'][0]['offset'],'bits':r['state_low']|(r['state_high']<<32),'flags':r['thing_flags'],'organization':r['organization_present'],'word118':r['actor_word118'],'state_statuses':r['state_statuses']})
    rows=[];required=set(REQUIRED)
    for c in inputs:
        if c['offset'] is not None and c['offset'] not in SUPPORTED[c['kind']]:required.add(h.P.u32(TEMPLATES[c['kind']][2][c['offset']][0]+0x20))
        for pc in h.PRECISION:
            values=[Machine(pc,image,c).run() for image in (h.P,h.V)];assert values[0]==values[1]
            rows.append({'precision':pc,'input':c,'native':values[0]})
    pins=[{'va':hex(a),'bytes':b,'intercepted_entry':a in required} for a,b in sorted(h.VISITED.items())]
    after=[]
    for kind in ['element','regiment','aircraft']:
        case=next(c for c in ctors['cases'] if c['input']['kind']==kind)
        vt=struct.unpack_from('<I',bytes.fromhex(case['output']['initialized_hex']))[0]
        slot=vt+0x14;assert h.P.range(slot,4)==h.V.range(slot,4) and h.P.u32(slot)==0x4b2830
        after.append({'kind':kind,'slot':hex(slot),'entry':hex(h.P.u32(slot)),'bytes':h.P.range(slot,4).hex()})
    doc={'schema':2,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'descriptor_fixture_sha256':H(REG.read_bytes()),'constructor_fixture_sha256':H(CT.read_bytes()),'actual_input_sha256':H(actual_path.read_bytes()),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'fresh_original_v14_machines':len(rows)*2,'rows':rows,'address_byte_pins':pins,'after_init_vtable_pins':after}
    p=HERE/'native-mind-init.json'
    if '--write' in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
    else:assert json.loads(p.read_text())==doc
    counts={}
    for r in rows:counts[r['native']['outcome']]=counts.get(r['native']['outcome'],0)+1
    print(json.dumps({'rows':len(rows),'machines':len(rows)*2,'outcomes':counts,'pins':len(pins),'executed':sum(not p['intercepted_entry'] for p in pins),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
