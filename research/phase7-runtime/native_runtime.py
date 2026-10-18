"""Original clock initialization and composed world/queue execution.

No game process or original file mutation. Actor/component callbacks are
explicit fixture leaves; actual world phases and queue operations execute.
"""
from pathlib import Path
import hashlib,json,struct,sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-queues'))
import native_queues as q
from unicorn.x86_const import *

bits=q.bits

class Machine(q.Machine):
    def __init__(self,pc=53,rate=16):
        super().__init__({'counts':[1]*5,'seed':0x12345678,'precision':pc,
            'clock':{'begin_bits':0,'end_bits':0,'step_bits':bits(1/rate)},'callback_actions':{}})
        self.outer=[]
        self.timeline=[]
        self.start_actions=[]
        self.manager_actions=[]
        self.actions_once=True
        svar=self.alloc(0x300);value=self.alloc(4)
        self.write(0xb39ab0,svar);self.write(svar+0x1c,value);self.write(value,bits(rate))
        self.write(svar+0x1bc,self.alloc(4))
        self.write(self.world+0x13c,self.alloc(0x60))
        #Execute only native reciprocal initialization; surrounding managers
        #are separately required by whole world Init and not claimed here.
        self.u.reg_write(UC_X86_REG_ESI,self.world)
        self.u.reg_write(UC_X86_REG_ESP,self.stack)
        self.u.emu_start(0x4f287d,0x4f28a6,count=100)
        assert self.u.reg_read(UC_X86_REG_EIP)==0x4f28a6
        self.u.reg_write(UC_X86_REG_ESP,self.stack)
        for i,queue in enumerate(self.queues):
            for off in range(8,32 if i>=3 else 20,4):self.write(queue+off,0)
            if i<3:self.call(0x50b580,queue,[bits([.0001,1,5][i])])
            else:
                self.call(0x50b690,queue,[0x4f05c0 if i==3 else 0x4f05b0])
        self.write(self.owner_vtable+0x18,0x23f0020)
        self.write(self.owner_vtable+0x1c,0x23f0020)

    def hook(self,u,pc,size,_):
        ecx=u.reg_read(UC_X86_REG_ECX)
        if pc in (0x41c0e0,0x41c110):self.ret(4);return
        if pc==0x77d9d0:
            self.outer.append(['fog',self.read(self.world+0xdc),self.arg(1)&255]);self.ret(12);return
        if pc==0x4f96d0:
            self.outer.append(['first',self.read(self.world+0xdc)])
            self.nested_actions(self.start_actions)
            self.ret();return
        if pc==0x54c280:
            self.outer.append(['manager',self.read(self.world+0xdc)])
            if self.actions_once:
                self.actions_once=False
                self.nested_actions(self.manager_actions)
            self.ret();return
        if pc in (0x50b330,0x50b020):
            self.current_queue=self.queues.index(ecx)
        before=len(self.events)
        super().hook(u,pc,size,_)
        if len(self.events)>before:
            self.timeline.append({'tick':self.read(self.world+0xdc),'phase':self.read(self.world+0xd8),'event':self.events[-1]})

    def nested_actions(self,actions):
        registers=[UC_X86_REG_EAX,UC_X86_REG_EBX,UC_X86_REG_ECX,UC_X86_REG_EDX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP,UC_X86_REG_ESP,UC_X86_REG_EIP,UC_X86_REG_EFLAGS]
        saved={r:self.u.reg_read(r) for r in registers}
        for op in actions:self.action(op,nested=True)
        for r,v in saved.items():self.u.reg_write(r,v)

def run():
    plans=[]
    for pc in (24,53,64):
        for rate in (1,7,10,16,30,60,120,256):
            m=Machine(pc,rate)
            snapshot=m.snapshot()
            assert all(row['current']==0 and all(not bucket for bucket in row['buckets']) for row in snapshot)
            plans.append({'precision':pc,'rate_bits':bits(rate),'step_bits':m.read(m.world+0xe8),
                          'counts':[len(row['buckets']) for row in snapshot]})
    composed=[]
    specs=[('periodic_and_deferred',[],[],{}),
           ('first_tick_scheduling',[q.add(3,20,201,.125)],[],{}),
           ('manager_same_tick_late_queue',[],[q.add(4,30,301,0),q.add(3,31,302,0)],{}),
           ('callback_mutation',[],[],{12:[q.cancel(2),q.add(3,4,41,0),q.add(4,4,42,0)]})]
    for pc in (24,53,64):
        for name,start,manager,actions in specs:
            m=Machine(pc,16)
            initial=[q.add(0,1,11),q.add(0,2,12),q.add(1,3,13),q.add(2,4,14),q.add(3,5,15,.125),q.add(4,6,16,.25)]
            for op in initial:m.action(op)
            m.callback_actions={int(k):v[:] for k,v in actions.items()}
            m.start_actions=start;m.manager_actions=manager
            for _ in range(85):m.call(0x4f9970,m.world)
            composed.append({'name':f'{name}_pc{pc}','precision':pc,'initial':initial,'first':start,'manager':manager,
                'callbacks':actions,'ticks':85,'timeline':m.timeline,'outer':m.outer,'queues':m.snapshot(),
                'rng_state':m.read(m.world+0xd4),'clock':[m.read(m.world+x) for x in (0xdc,0xe0,0xe4,0xe8)],'phase':m.read(m.world+0xd8)})
    out={'schema':1,'passed':True,'plans':plans,'composed':composed,
         'original_sha256':hashlib.sha256(q.ORIGINAL.b).hexdigest(),'v14_sha256':hashlib.sha256(q.ACCEPTED.b).hexdigest(),
         'visited':[{'va':hex(va),'bytes':raw} for va,raw in sorted(q.VISITED.items())],
         'seams':['allocator/free','logging/Lua fog callback','first-tick gameplay/audio callback','manager54C280','component virtual Update callbacks'],
         'scope':'Native clock reciprocal and queue initialization; actual world phase update invokes actual periodic/deferred queues. Fixture callbacks supply deterministic scheduling actions; no complete actor/match behavior claim.'}
    target=HERE/'native-runtime.json';target.write_text(json.dumps(out,indent=2)+'\n')
    print(json.dumps({'passed':True,'plans':len(plans),'composed':len(composed),'sha256':hashlib.sha256(target.read_bytes()).hexdigest()}))
if __name__=='__main__':run()
