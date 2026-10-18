"""Actual775EE0 scenario dispatch,59E6F0 task storage,5A4280 branch and
5A3E20/5A3F20/5A3EB0 ->5A2B90 prefix A8 store. No whole-task/Load success.
The distinct initialized WorldRuleMode factory is already proved by the
immutable load-zone-services/native-initial-zones.json fixture.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from unicorn.x86_const import *
ORIGINAL,V14=h.P,h.V
class Machine(h.Machine):
    def __init__(self,c,pc):
        super().__init__(b'',pc);self.c=c;self.events=[];self.selected=None
        self.session=self.alloc(0x200);self.creator=self.alloc(0x60);self.old_resource=self.alloc(0xc0);self.new_resource=self.alloc(0xc0)
        self.w(self.session+0x54,c['captured_game_type']);self.w(self.session+0x58,2)
        self.w(self.session+0x70,self.creator);self.w(self.creator+0x50,self.old_resource if c['scenario_present'] else 0)
        self.w(self.new_resource+0xa8,0xabcdef01);self.w(self.old_resource+0xa8,0xabcdef02)
        def log(m):
            if m.arg(2)==0xaf4:m.w(m.session+0x54,c['current_game_type'])
            m.ret(0)
        self.seam(0x404f60,log)
        self.seam(0x775db0,lambda m:m.ret(4))
        self.seam(0x77cbc0,lambda m:m.ret(4))
        self.seam(0x76e0f0,lambda m:m.ret(0))
        self.seam(0x855d30,lambda m:m.ret(12,1))
        def load(m):
            assert m.u.reg_read(UC_X86_REG_ECX)==m.session and m.arg(0)==m.creator
            m.selected={'kind':'Load','word':m.arg(1)};m.ret(8)
        def film(m):
            m.selected={'kind':'Film','editor':bool(m.arg(1)&255)};m.ret(8)
        self.seam(0x76be80,load);self.seam(0x76c0a0,film)
        self.seam(0x5b17f0,lambda m:m.ret(4)) # base task ctor explicitly outside
        self.seam(0x5a28b0,lambda m:m.ret(0,m.new_resource)) # actual GWorld/Resource factory required separately
        def cutoff(m):
            m.outcome={'kind':'Cutoff','pc':0x5a2bf7};raise h.Throw()
        self.probes[0x5a2bf7]=cutoff
    def run(self):
        self.call(0x775ee0,self.session,[])
        accepted=bool(self.u.reg_read(UC_X86_REG_EAX)&255)
        out={'selected':self.selected or {'kind':'MissingScenario'},'accepted':accepted}
        if self.selected and self.selected['kind']=='Load':
            out['load']=self.task_prefix(self.selected['word'])
        return out
    def task_prefix(self,kind):
        self.task=self.alloc(0x28,0xa5)
        self.call(0x59e6f0,self.task,[self.creator,kind])
        after_ctor={'kind_word':self.r(self.task+0x20),'source_same':self.r(self.task+0x1c)==self.creator,
                    'result':'source_resource' if self.r(self.task+0x24)==self.old_resource else 'null' if self.r(self.task+0x24)==0 else 'other'}
        assert self.r(self.task)==0xa43794
        self.u.reg_write(UC_X86_REG_ESP,h.STACK);self.u.reg_write(UC_X86_REG_EBP,h.STACK+0x200)
        self.u.reg_write(UC_X86_REG_ESI,self.task)
        try:self.u.emu_start(0x5a4311,h.STOP,count=100000)
        except h.Throw:pass
        assert self.outcome=={'kind':'Cutoff','pc':0x5a2bf7}
        resource=self.r(self.task+0x24)
        return {'after_ctor':after_ctor,'mode_a8':self.r(resource+0xa8),
                'resource':'source_resource' if resource==self.old_resource else 'new_resource'}
def main():
    rows=[];raw=[]
    cases=[dict(captured_game_type=g,current_game_type=g,scenario_present=present)
           for g in range(7) for present in [False,True]]
    cases += [dict(captured_game_type=a,current_game_type=b,scenario_present=True)
              for a,b in [(3,1),(5,1),(1,4),(3,4),(4,1)]]
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.P=image
        for pc in h.CW:
            for c in cases:rows.append(dict(build=build,precision=pc,input=c,native=Machine(c,pc).run()))
            for kind in [0,1,2,3,0xffffffff]:
                m=Machine(dict(captured_game_type=1,current_game_type=1,scenario_present=True),pc)
                raw.append(dict(build=build,precision=pc,kind=kind,native=m.task_prefix(kind)))
    h.P=ORIGINAL
    excluded={0x404f60,0x775db0,0x77cbc0,0x76e0f0,0x855d30,0x76be80,0x76c0a0,0x5b17f0,0x5a28b0,
       0x6f7810,0x6f7430,0x856370,0x401b00,0x856560,0x41d290,0x41dfb0,0x858a00,0x858ad0,0x4f0110,0x404ea0,0x404f30,0x5a1040,0x5a2bf7}
    pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a not in excluded}
    for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
    mode_fixture=ROOT/'research/skirmish-20261016-load-zone-services/native-initial-zones.json'
    out={'schema':1,'scope':'actual scenario dispatch/task ctor and load-mode store prefix; required preceding task/sync/reset and factory/logging services are explicit seams; no task/world success',
         'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
         'machine_sha256':hashlib.sha256(Path(h.__file__).read_bytes()).hexdigest(),
         'mode_factory_fixture_sha256':hashlib.sha256(mode_fixture.read_bytes()).hexdigest(),
         'callback_mutation_scope':'explicit diagnostic logging seam storage intervention changes currentSession+54 after cached3/5 comparisons; not proof of actual application reentrancy',
         'rows':rows,'raw_kind_rows':raw,'pins':pins}
    p=HERE/'native-map-resource-mode.json'
    if '--write' in sys.argv:p.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(p.read_text()),'native mode replay differs from frozen fixture'
    print(json.dumps({'rows':len(rows),'raw_rows':len(raw),'pins':len(pins),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
