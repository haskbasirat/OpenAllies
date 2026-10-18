"""Native bounded Recruit source producer, deferred list links, transport
reference validation and ActorState type store. Parser/enum/native functions
execute;4BDBA0 is an explicit typed target-registry seam, not a fake loader.
"""
from pathlib import Path
import sys,json,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-layouts'))
from layout_machine import Machine,P,V,H,native_windows,DATA
from unicorn.x86_const import *
CW={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
TEMPLATE='Templates/Components/template_recruit.tgi'
class Source(Machine):
    def __init__(self):
        super().__init__();self.targets={};old=self.gd;self.gd=self.alloc(0x1000)
        self.u.mem_write(self.gd,bytes(self.u.mem_read(old,0x600)));self.w(0xb36530,self.gd)
    def hook(self,u,pc,size,user):
        if pc==0x96cab0:self.ret(0);return
        if pc==0x405d60:self.w(self.arg(),self.cstring('<diagnostic text>'));self.ret();return
        if pc==0x4bdba0:
            name=self.raw_wstr(self.arg());assert self.arg(1)==0x2b
            found=self.targets.get(name.lower(),0)
            self.recorded.append({'event':'actor_lookup','ids':name,'found':bool(found)})
            self.ret(found,pop=8);return
        super().hook(u,pc,size,user)
    def target(self,name,transport=False,actor=True):
        obj=self.alloc(0x600);self.w(obj+8,self.cstring(name));self.w(obj+0x174,(1<<24)if transport else 0)
        if actor:self.targets[name.lower()]=obj
        return obj
    def setup(self,pc):self.u.reg_write(UC_X86_REG_FPCW,CW[pc])
    def frame(self):return self.alloc(0x10000)+0xf000
    def source_list(self,text,structure):
        self.parse_file(TEMPLATE)
        parsed=self.parse_file('owner.tgi',text.encode())
        node=self.children(parsed['root'])[0];actor=self.alloc(0x600)
        self.w(actor+8,self.cstring('owner'));self.w(actor+0x174,(1<<13)if structure else 0)
        frame=self.frame();self.w(frame+0xb0,node)
        self.u.reg_write(UC_X86_REG_ESP,frame);self.u.reg_write(UC_X86_REG_EBP,actor)
        self.thrown=None;self.messages=[]
        self.u.emu_start(0x467c89,0x467db6,count=2_000_000);self.check_null()
        if self.thrown is not None:return {'error':'fragment','list':[],'messages':self.messages}
        refs=[];cur=self.r(self.gd+0x824)
        while cur:refs.append(cur);cur=self.r(cur+8)
        order=[]
        for ref in refs:
            assert self.r(ref)==0xa03470
            order.append(self.wstr(self.r(ref+4)))
            result=self.thiscall(0x461810,ref,[])
            if 'thrown'in result:return {'error':'link','list':self.targets_list(actor),'callback_order':order,'messages':self.messages}
        return {'error':None,'list':self.targets_list(actor),'callback_order':order,'mask':self.r(actor+0x174)}
    def targets_list(self,actor):
        out=[];cur=self.r(actor+0x4ec)
        while cur:out.append(self.wstr(self.r(self.r(cur)+8)));cur=self.r(cur+4)
        return out
    def transport(self,name):
        actor=self.alloc(0x600);self.w(actor+8,self.cstring('owner'));self.w(actor+0x184,0xa5a5a5a5)
        cell=self.alloc(4);self.w(cell,self.cstring(name));r=self.thiscall(0x4608c0,actor,[cell])
        return {'error':'thrown'in r,'nonnull':self.r(actor+0x184)!=0,'stored':self.r(actor+0x184) if 'thrown'in r else None,'messages':self.messages}
    def state_type(self,text):
        assert 'eax'in self.thiscall(0x9cc560,0,[])
        parsed=self.parse_file('state.tgi',text.encode());node=self.children(parsed['root'])[0]
        state=self.alloc(0x40);self.w(state+8,self.cstring('fixture_state'));frame=self.frame()
        self.u.reg_write(UC_X86_REG_ESP,frame);self.u.reg_write(UC_X86_REG_ESI,state);self.u.reg_write(UC_X86_REG_EDI,node)
        self.thrown=None;self.messages=[];self.u.emu_start(0x47dcb3,0x47dd32,count=2_000_000);self.check_null()
        return {'error':self.thrown is not None,'value':self.r(state+0x28),'messages':self.messages}

def owner(names,structure=True,component=True):
    t='[Thing]\n{ string IDS = owner\n string category = ACTOR\n'
    if structure:t+=' [StructureComponent]\n {}\n'
    if component:
        t+=' [RecruitComponent template=RecruitComponent]\n {\n'
        for n in names:t+=' [RecruitActor]\n actor_ids = '+n+'\n'
        t+=' }\n'
    return t+'}\n'
def main():
    rows=[];visited={}
    for pc in CW:
        for names,structure,component in [([],True,True),([],False,False),(['a'],True,True),(['a','b','a'],True,True),(['a','missing','b'],True,True),(['wrong_class'],True,True),(['a'],False,True)]:
            m=Source();m.setup(pc);m.target('a');m.target('b');m.target('wrong_class',actor=False)
            text=owner(names,structure,component);result=m.source_list(text,structure)
            rows.append({'kind':'targets','precision':pc,'text':text,'result':result});visited.update(m.visited)
        for name in ('','transport','plain','missing'):
            m=Source();m.setup(pc);m.target('transport',transport=True);m.target('plain')
            rows.append({'kind':'transport','precision':pc,'value':name,'result':m.transport(name)});visited.update(m.visited)
        for name in ('NONE','busy','Queue','invalid'):
            m=Source();m.setup(pc);text='[ActorState]\n{ string IDS = fixture_state\n int priority = 0\n string type = '+name+'\n}\n'
            rows.append({'kind':'state','precision':pc,'text':text,'result':m.state_type(text)});visited.update(m.visited)
    result={'schema':1,'scope':__doc__,'original_sha256':H(P.b),'v14_sha256':H(V.b),'template':{'path':TEMPLATE,'sha256':H((DATA/TEMPLATE).read_bytes()),'text':(DATA/TEMPLATE).read_bytes().decode('latin1')},'cases':rows,'native_windows':native_windows(visited)}
    path=HERE/'native-recruit-source.json';path.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
