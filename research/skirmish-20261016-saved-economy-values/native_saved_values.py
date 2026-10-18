"""Original/v14 resource getters over captured real saved actors/components.

Reuses the existing native value harness (allocator/free and pure Properties
context lookup seams), adding the guarded-null-page rule and independently
loading v14. Captured source +17C heads are actual completed producer facts.
Constructor fragment proves absent-Economy vectors/supply defaults; the only
constructor seam is allocation. No game/save is launched or written.
"""
from pathlib import Path
import sys,json,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import Machine,MODES,ORIGINAL,ACCEPTED,sha,pin,guard_null_page,check_null_accesses
sys.path.insert(0,str(ROOT/'research/phase7-economy-values'))
from native_values import EconomyMachine,ACTOR,DEF,ECO,OUTPUT,DOMAIN
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *

def replace_image(u):
    old=(ORIGINAL.size+4095)&~4095;new=(ACCEPTED.size+4095)&~4095
    if new>old:u.mem_map(ORIGINAL.base+old,new-old)
    u.mem_write(ACCEPTED.base,ACCEPTED.data[:ACCEPTED.header_size])
    for rva,size,offset,_ in ACCEPTED.sections:
        if size:u.mem_write(ACCEPTED.base+rva,ACCEPTED.data[offset:offset+size])

def constructors():
    rows=[]
    for image in ('original','v14'):
        for pc,cw in MODES:
            m=Machine(cw)
            if image=='v14':replace_image(m.uc)
            actor=0x2010000;m.uc.mem_write(actor,b'\xa5'*0x600)
            m.uc.reg_write(UC_X86_REG_ESP,m.STACK);m.uc.reg_write(UC_X86_REG_ESI,actor)
            m.uc.reg_write(UC_X86_REG_EBX,0);m.uc.reg_write(UC_X86_REG_EDI,8)
            m.seam(0x6f7810,lambda machine,esp:(machine.alloc(machine.arg(0)),0))
            m.uc.emu_start(0x4628d1,0x462943,count=10000);check_null_accesses(m)
            values={name:{'count':m.read(actor+offset+4),'capacity':m.read(actor+offset+8),'pointer_nonnull':m.read(actor+offset)!=0}
                for name,offset in [('cost',0x324),('upkeep',0x330),('production',0x33c)]}
            assert all(v['count']==0 and v['capacity']==8 and v['pointer_nonnull'] for v in values.values())
            assert m.read8(actor+0x348)==0
            rows.append({'image':image,'precision':pc,'vectors':values,'supply_required':False})
    result={'cases':rows,'windows':pin({'constructor_fragment':(0x4628d1,0x462943)}),'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data)}
    (HERE/'native-absent-economy-constructor.json').write_text(json.dumps(result,indent=2)+'\n')

class Native(EconomyMachine):
    def __init__(self,c,image):
        super().__init__(c);self.uc=self.u;guard_null_page(self)
        if image=='v14':
            replace_image(self.u)
            # Image .data replacement clears the two input registry globals
            # initialized by the inherited setup. Reinstall these explicit
            # fixture pointers after loading v14; heap/source arrays are outside
            # the image and have not changed.
            self.w(0xb3a660,DOMAIN);self.w(0xb36530,DOMAIN+0x100)
    def run(self):
        if self.c['operation']=='upkeep_blocked':
            self.reset(0);actor=ACTOR+self.c['actor']*0x1000;component=ECO+self.c['actor']*0x1000;vt=0x4621000
            self.w(component+4,actor);self.w(actor,vt);self.w(vt+0x108,0x4d1e80);self.w(actor+0xe8,0x4622000)
            self.u.reg_write(UC_X86_REG_ECX,component);self.gate=None
            def boundary(u,pc,size,user):
                if pc in (0x57008b,0x570102):self.gate=(pc==0x570102);u.emu_stop()
            self.u.hook_add(UC_HOOK_CODE,boundary)
            self.u.emu_start(0x570020,self.STOP,count=200000);check_null_accesses(self)
            assert self.gate is not None
            result={'predicate':self.gate,'values':self.vector(OUTPUT),'caches':{k:self.vector(ECO+int(k)*0x1000+0x44)for k in self.c['caches']}}
            for addr,data in self.visited.items():assert data==ORIGINAL.at(addr,len(data))==ACCEPTED.at(addr,len(data)),hex(addr)
            return result
        result=super().run();check_null_accesses(self)
        for addr,data in self.visited.items():assert data==ORIGINAL.at(addr,len(data))==ACCEPTED.at(addr,len(data)),hex(addr)
        return result

def main():
    constructors()
    source=HERE/'actual-saved-input.json'
    if not source.exists():
        print('constructor6cases passed; actual capture pending');return
    graph=json.loads(source.read_text());rows=[]
    for image in ('original','v14'):
        for key,actor in graph['actors'].items():
            for op in ('eligible','supplied','upkeep_blocked','Cost','Upkeep','Production'):
                if op=='upkeep_blocked' and actor['economy'] is None:continue
                c=copy.deepcopy(graph);c.update(name=f'actor{key}_{op}_{image}',actor=int(key),operation=op,cache_members=True)
                try:result=Native(c,image).run()
                except Exception:
                    print(c['name']);raise
                rows.append({'image':image,'actor':int(key),'operation':op,'output':result})
    result={'schema':1,'scope':__doc__,'input':graph,'cases':rows,'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),
            'captured_input_sha256':sha(source.read_bytes()),'generator_sha256':sha(Path(__file__).read_bytes()),
            'reused_harness_sha256':sha((ROOT/'research/phase7-economy-values/native_values.py').read_bytes())}
    path=HERE/'native-saved-values.json';path.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
