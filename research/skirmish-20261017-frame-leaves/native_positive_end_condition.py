"""Actual76F600 +76BA90 +76BB90 +765E00 with typed current objects.
CString transports/lookup/format/UI/GameSpy are explicit recorded leaves;
no successful application UI or online-manager construction is claimed.
"""
from pathlib import Path
import sys,json,itertools,random
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_local_heartbeat import Machine as Base
from native_heartbeat import env,node,PC
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EIP,UC_X86_REG_ECX
class Machine(Base):
    def __init__(self,image):
        self.watch=False;super().__init__(image)
        self.stubs.update({0x4b9f40:self.lookup,0x405d60:self.format,0x40bc50:self.ui,0x793b40:self.gamespy})
        self.u.hook_add(UC_HOOK_MEM_READ,self.read);self.u.hook_add(UC_HOOK_MEM_WRITE,self.write)
    def cut(self,label):
        if self.case['failure']==label:self.stopped=label;self.u.emu_stop();return True
        return False
    def read(self,u,access,a,size,value,user):
        if not self.watch:return
        if a==self.session+0x16d:self.events.append('read16d')
        if a==self.session+0x16e:self.events.append('read16e')
        if a==self.world_ptr+0x48:self.events.append('world48')
    def write(self,u,access,a,size,value,user):
        if self.watch and a==self.session+0x16e:self.events.append('store16e')
    def lookup(self):
        out=self.arg(0);key=self.text(self.r(out));self.events.append({'lookup':key})
        if self.cut('lookup'):return
        self.w(out,self.string('No human opponents remain.'));self.ret(8,out)
    def format(self):
        out=self.arg(0);text=self.text(self.arg(1));self.events.append({'format':text})
        if self.cut('format'):return
        self.w(out,self.string(text));self.ret(0,out)
    def ui(self):
        self.events.append({'ui':self.arg(1),'text':self.text(self.arg(2))})
        if self.cut('ui'):return
        self.w(0xb40c9c,self.online_ptr if self.case['online_after'] else 0);self.ret()
    def gamespy(self):
        self.events.append({'gamespy':self.text(self.r(self.arg(0)))})
        if self.cut('gamespy'):return
        self.ret(4)
    def hook(self,u,pc,size,user):
        if self.watch:
            if pc in (0x76ba90,0x76bb90):
                phase='opposition' if pc==0x76ba90 else 'first_active';self.phase=phase;self.events.append(phase)
                if self.cut(phase):return
            if pc==0x765e00:self.getters.append({'phase':self.phase,'player':self.players_by_ptr[u.reg_read(UC_X86_REG_ECX)]})
            if pc==0x76f654 and self.cut('world'):return
        super().hook(u,pc,size,user)
    def run(self,c,p):
        self.setup({'env':env(),'node':node(),'conns':[],'list':['node']});self.fpu(p)
        self.case=c;self.events=[];self.getters=[];self.players_by_ptr={};self.fail_diag=False;self.stopped=None
        self.world_ptr=self.alloc(0x60);self.w(self.world_ptr+0x48,c['world48']);self.w(0xb36534,self.world_ptr)
        self.b(self.session+0x16d,c['enabled']);self.b(self.session+0x16e,c['notified'])
        teams=[0]+[self.alloc(4) for _ in range(3)];kingdoms=[]
        for k in c['kingdoms']:
            ptr=self.alloc(0x240);self.w(ptr+0x19c,k['result']);self.w(ptr+0x1ec,teams[k['team']]);kingdoms.append(ptr)
        tail=0
        for i,player in reversed(list(enumerate(c['players']))):
            ptr=self.alloc(0x60);self.players_by_ptr[ptr]=i;self.b(ptr+0xc,player['ai']);self.w(ptr+0x28,0 if player['kingdom'] is None else kingdoms[player['kingdom']]);self.w(ptr+0x24,self.string('kingdom'+str(i)))
            link=self.alloc(8);self.w(link,ptr);self.w(link+4,tail);tail=link
        self.w(self.session+0xbc,tail);self.online_ptr=self.alloc(0xc0);self.w(self.online_ptr+0xb8,1)
        self.w(0xb40c9c,self.online_ptr if c['online'] else 0)
        self.phase=None;self.watch=True;self.active_capture=True
        self.w(self.stack,self.stop);self.u.reg_write(UC_X86_REG_ESP,self.stack)
        self.u.emu_start(0x76f600,self.stop,count=500000)
        if self.stopped is None:assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
        self.watch=False;self.active_capture=False
        return {'events':self.events,'getters':self.getters,'notified':self.rb(self.session+0x16e),'stop':self.stopped}
def cases():
    out=[]
    for enabled,notified,world48,online,after in itertools.product((0,1),(0,1),(1,2),(False,True),(False,True)):
        out.append(dict(enabled=enabled,notified=notified,world48=world48,online=online,online_after=after,failure=None,
                        kingdoms=[dict(result=0,team=0)],players=[dict(ai=False,kingdom=0)]))
    rng=random.Random(0x76f600)
    for _ in range(64):
        out.append(dict(enabled=1,notified=0,world48=1,online=False,online_after=False,failure=None,
            kingdoms=[dict(result=rng.randrange(3),team=rng.randrange(4)) for _ in range(4)],
            players=[dict(ai=bool(rng.randrange(2)),kingdom=rng.choice([None,0,1,2,3])) for _ in range(rng.randrange(6))]))
    for failure in ('opposition','world','first_active','lookup','format','ui','gamespy'):
        out.append(dict(enabled=1,notified=0,world48=1,online=True,online_after=True,failure=failure,
                        kingdoms=[dict(result=0,team=0)],players=[dict(ai=False,kingdom=0)]))
    return out
def main():
    rows=[];pins={};seams={}
    for image in ('original','v14'):
        m=Machine(image)
        for p in PC:
            for c in cases():rows.append({'image':image,'precision':p,'input':c,'output':m.run(c,p)})
        pins.update(m.executed);seams.update(m.seams)
    for a,b in zip(rows[:len(rows)//2],rows[len(rows)//2:]):assert a['output']==b['output']
    data=json.dumps({'scope':__doc__,'rows':rows,'executed_pins':pins,'seam_entries_not_executed':seams},sort_keys=True,indent=2)+'\n'
    path=HERE/'native-positive-end-condition.json'
    if '--write' in sys.argv:path.write_text(data,encoding='utf-8',newline='\n')
    else:assert path.read_text(encoding='utf-8')==data
    print(f'{len(rows)} rows / {len(pins)} genuine executed pins PASS')
if __name__=='__main__':main()
