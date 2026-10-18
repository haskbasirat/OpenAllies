"""Original39 ActorCommand definitions from4DA9D0/4DA2F0 +475870/4740F0.

Runs the original enum construction, DataManager constructor47F7C0 and
complete GameData command registration475870. No definition values are
supplied by the harness. Uses the existing definition/layout allocator,
CString, mutex, line source and page-zero guard seams.
"""
from pathlib import Path
import sys, json
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-layouts'))
from layout_machine import Machine, P, V, H, native_windows
from unicorn.x86_const import *

CW={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
DATA=ROOT.parent/'reconstruction/assets/original/Data'
TEMPLATE='Templates/template_actor_command.tgi'
FILES=['Game/actor_commands.tgi','Game/actor_commands_aa.tgi'] + [
    'Game/ActorCommands/'+n+'.tgi' for n in ('air_recon','air_attack','bombard','construct','pack','unpack')]

class Commands(Machine):
    def hook(self,u,pc,size,user):
        if pc==0x96cab0:
            # CRT atexit only records the enum destructor for shutdown.
            self.recorded.append({'event':'atexit','function':hex(self.arg())})
            self.ret(0)
            return
        super().hook(u,pc,size,user)

    def setup(self, pc):
        self.u.reg_write(UC_X86_REG_FPCW,CW[pc])
        # Exact CRT thunk: ctor4DA9D0(B3CBA0,1), then atexit9ED2F0.
        assert 'eax' in self.thiscall(0x9cd7e0,0,[])
        self.manager=self.gd+0x274
        assert 'eax' in self.thiscall(0x47f7c0,self.manager,[39,2])
        assert self.r(0xb3aa58)==self.manager
        assert 'eax' in self.thiscall(0x475870,self.gd,[])
        assert self.r(self.manager+0x28)==39

    def definitions(self):
        out=[]
        for p in self.manager_entries(self.manager):
            out.append({'ids':self.wstr(self.r(p+8)), 'mind_type':self.r(p+0xc),
                        'component_slot':self.r(p+0x18),'behavior_flags':self.r(p+0x2c)})
        return out

    def lookup(self, name):
        # Same original manager Find used by the saved-reference reader4DAB60.
        result=self.thiscall(0x463fa0,self.manager,[self.cstring(name)])
        assert 'eax' in result,result
        return result['eax']

    def ui_flags(self, obj, node):
        # Original flag reads only. Earlier UI localization/resource/VFX
        # loading and later CString cleanup are outside this bounded fragment.
        frame=self.alloc(0x10000);esp=frame+0xf000
        self.u.reg_write(UC_X86_REG_ESP,esp)
        self.u.reg_write(UC_X86_REG_ESI,obj)
        self.u.reg_write(UC_X86_REG_EDI,node)
        self.thrown=None;self.messages=[]
        self.u.emu_start(0x48cf0a,0x48cf97,count=2_000_000)
        self.check_null()
        assert self.thrown is None,(self.thrown,self.messages)
        assert self.u.reg_read(UC_X86_REG_EIP)==0x48cf97
        assert self.u.reg_read(UC_X86_REG_ESP)==esp
        return list(self.u.mem_read(obj+0x3c,6))+[self.r(obj+0x44)]

    def load_flag_nodes(self):
        self.parse_file(TEMPLATE)
        rows=[]
        for path in FILES:
            parsed=self.parse_file(path)
            for node in self.children(parsed['root']):
                result=self.thiscall(0x5b54d0,node,[self.cstring('IDS')])
                assert 'eax' in result,result
                ids=self.wstr(self.r(result['eax']))
                obj=self.lookup(ids)
                assert obj,ids
                before=[self.r(obj+o) for o in (0xc,0x18,0x2c)]
                flags=self.ui_flags(obj,node)
                after=[self.r(obj+o) for o in (0xc,0x18,0x2c)]
                assert before==after,ids
                rows.append({'path':path,'ids':ids,'before':before,'after':after,'ui_flags_3c_to_41_and_position_44':flags})
        return rows

def main():
    runs=[];visited={}
    for pc in CW:
        m=Commands();m.setup(pc)
        defs=m.definitions()
        lookup=[]
        for name in ('move','MOVE','MoVe','kill','KILL','not_a_command','',' move','move ','\u00e9move'):
            obj=m.lookup(name)
            lookup.append({'query':name,'id':m.wstr(m.r(obj+8)) if obj else None})
        flags=m.load_flag_nodes()
        runs.append({'precision':pc,'definitions':defs,'lookups':lookup,'authored_flag_loads':flags,
                     'crt_seams':[e for e in m.recorded if e['event']=='atexit']})
        visited.update(m.visited)
        print(pc,defs,flush=True)
    result={'schema':1,'scope':__doc__,'original_sha256':H(P.b),'v14_sha256':H(V.b),
            'source_files':[{'path':p,'sha256':H((DATA/p).read_bytes())} for p in [TEMPLATE]+FILES],
            'runs':runs,'native_windows':native_windows(visited)}
    out=HERE/'native-actor-commands.json'
    out.write_text(json.dumps(result,indent=2)+'\n')
    print('wrote',out,H(out.read_bytes()))

if __name__=='__main__':main()
