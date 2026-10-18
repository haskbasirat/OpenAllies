"""Execute original/v14 Mover576700 and reached native4D64F0 until required services.
No scheduler/state effect returns success. Default immutable replay.
"""
from pathlib import Path
import json,hashlib,sys,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_ECX
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
REG=ROOT/'research/phase7-actors/native-actor-states.json'
REGISTERED=json.loads(REG.read_text())['registration']
H=lambda b:hashlib.sha256(b).hexdigest()
REQUIRED={0x50ac00:'required50AC00',0x4d4a90:'required4D4A90',0x4d16b0:'required4D16B0',0x4d1350:'required4D1350'}
class Machine(h.Machine):
    def __init__(self,pc,image,c):
        super().__init__(pc)
        if image is not h.P:
            old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
            if new>old:self.u.mem_map(image.base+old,new-old)
            self.u.mem_write(image.base,image.b[:0x1000])
            for vs,r,raw,off in image.sections:
                if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
        self.actor=self.alloc(0x200);self.mover=self.alloc(0x90);self.world=self.alloc(0x300)
        self.manager=self.alloc(0x400);self.table=self.alloc(42*4)
        self.w32(0xb36530,self.manager);self.w32(self.manager+0x2e4,self.table)
        for r in REGISTERED['records']:
            d=self.alloc(0x40);self.w32(self.table+r['index']*4,d)
            self.w32(d+0xc,r['index']);self.w32(d+0x2c,r['group']);self.w8(d+0x30,int(r['exclusive']))
        for i,m in enumerate(REGISTERED['masks']):self.w32(0xb3ac40+i*8,m);self.w32(0xb3ac44+i*8,m>>32)
        self.w32(self.mover,0xa3ce88);self.w32(self.mover+4,self.actor)
        self.w8(self.mover+0x56,c['falling']);self.w32(self.mover+0xc,self.world if c['scheduled'] else 0)
        self.w32(0xb36534,self.world if c['falling'] else 0)
        self.w32(self.world+0x2c,c['version']);self.w32(self.actor+0x100,c['state_low']);self.w32(self.actor+8,c['flags'])
        self.required=None;self.world_reads=[];self.writes=[]
        self.u.hook_add(UC_HOOK_MEM_READ,self.read_world,begin=self.world+0x2c,end=self.world+0x2f)
        self.u.hook_add(UC_HOOK_MEM_WRITE,self.write_actor,begin=self.actor,end=self.actor+0x1ff)
    def read_world(self,u,access,address,size,value,_):self.world_reads.append([address-self.world,size])
    def write_actor(self,u,access,address,size,value,_):self.writes.append([address-self.actor,size,value])
    def _hook(self,u,pc,size,_):
        if pc in REQUIRED:
            h.VISITED[pc]=bytes(u.mem_read(pc,size)).hex();assert h.P.range(pc,size)==h.V.range(pc,size)
            self.required=REQUIRED[pc];u.emu_stop();return
        super()._hook(u,pc,size,_)
    def run(self):
        try:self.call(0x576700,self.mover)
        except AssertionError:
            if not self.required:raise
        pc=self.u.reg_read(UC_X86_REG_EIP)
        assert pc==h.STOP or pc in REQUIRED,hex(pc)
        assert self.seams=={}
        return {'outcome':self.required or 'complete','state_low':self.r32(self.actor+0x100),'flags':self.r32(self.actor+8),'world_version_reads':self.world_reads,'actor_writes':self.writes}

def main():
    rows=[]
    for fall,ver,sched,state,flags in itertools.product([0,1,255],[0,21,22,30],[False,True],[0,1<<24],[0x100,0x80000100]):
        c={'falling':fall,'version':ver,'scheduled':sched,'state_low':state,'flags':flags}
        for pc in h.PRECISION:
            values=[Machine(pc,image,c).run() for image in (h.P,h.V)]
            assert values[0]==values[1]
            assert bool(values[0]['world_version_reads'])==bool(fall)
            rows.append({'precision':pc,'input':c,'native':values[0]})
    pins=[{'va':hex(a),'bytes':b,'required_entry':a in REQUIRED} for a,b in sorted(h.VISITED.items())]
    vt=0xa3ce88
    assert h.P.u32(vt+8)==h.V.u32(vt+8)==0x576700
    assert h.P.u32(vt+0x14)==h.V.u32(vt+0x14)==0x4b2830
    vtable_pins=[{'va':hex(vt+off),'bytes':h.P.range(vt+off,4).hex()} for off in (8,0x14)]
    doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'descriptor_fixture_sha256':H(REG.read_bytes()),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'fresh_original_v14_machines':len(rows)*2,'rows':rows,'address_byte_pins':pins,'vtable_byte_pins':vtable_pins}
    p=HERE/'native-mover-init.json'
    if '--write' in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
    else:assert json.loads(p.read_text())==doc
    counts={}
    for r in rows:counts[r['native']['outcome']]=counts.get(r['native']['outcome'],0)+1
    print(json.dumps({'rows':len(rows),'machines':len(rows)*2,'outcomes':counts,'pins':len(pins),'executed':sum(not p['required_entry'] for p in pins),'sha256':H(p.read_bytes()),'immutable':'--write' not in sys.argv}))
if __name__=='__main__':main()
