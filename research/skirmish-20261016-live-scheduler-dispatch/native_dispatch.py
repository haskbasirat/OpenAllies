"""Actual Updatable dispatch/prefix over native queues, not a complete battle.

Period/no-op leaves and Performer57F040 execute unchanged, including native
4D03D0 and50ACC0/50B1C0 queue mutations. Alloc/free/log are explicit queue
fixture seams. Reached unbound gameplay callees stop BEFORE execution and are
recorded, not returned as successful callbacks. Page0 remains unmapped.
"""
from pathlib import Path
import hashlib
import itertools
import json
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT/'research/phase6-queues'))
from native_queues import Machine as Queues, ORIGINAL, ACCEPTED, VISITED, bits
from unicorn.x86_const import *

PERFORMER_VTABLE = 0xa3df18
NOOPS = {16:0xa3e6a4, 18:0xa3b27c, 19:0xa3ccd8, 21:0xa3eb30, 30:0xa3ca24}
UNBOUND = {0x57ed20:'begin', 0x57e8c0:'use_frame', 0x4c8300:'end_context',
           0x57dc90:'finish', 0x563aa0:'animator_cue'}

def sha(raw): return hashlib.sha256(raw).hexdigest()

class Machine(Queues):
    def __init__(self, c):
        self.first_unbound = None
        super().__init__({'counts':[1,4,20,20,20], 'seed':0x12345678,
            'clock':{'begin_bits':bits(12.25),'end_bits':bits(12.5),'step_bits':bits(.25)},
            'precision':c['pc']})
        self.u.mem_unmap(0,4096)
        self.actor = self.alloc(0x180)
        self.perf = self.alloc(0x80)
        self.props = self.alloc(0x100)
        self.write(self.perf+8, PERFORMER_VTABLE)
        self.write(self.perf+4, self.actor)
        self.write(self.actor+0xe4, self.perf if c['active'] else 0)
        self.write(self.actor+8, c['thing_flags'])
        self.write(self.actor+0x78, self.props if c['property_flags'] is not None else 0)
        self.write(self.props+0x1c, c['property_flags'] or 0)
        self.write(self.actor+0x68, self.actor+0x600 if c['animator'] else 0)
        self.write(self.perf+0x10, c['stage'])
        self.write(self.perf+0x58, 17)
        self.owners[1] = self.perf+8
        self.names[self.perf+8] = 1
        for queue, owner, payload in [(0,1,0),(0,1,7),(0,2,0),(3,1,0),(4,2,13)]:
            self.action({'type':'add','queue':queue,'owner':owner,'data':payload,'delay_bits':bits(1)})
        self.rng_before = self.read(self.world+0xd4)
        self.before = self.snapshot()

    def hook(self,u,pc,size,arg):
        if pc in UNBOUND:
            self.first_unbound = UNBOUND[pc]
            u.emu_stop()
            return
        super().hook(u,pc,size,arg)

    def timed(self,event):
        self.write(self.stack,self.stop)
        self.write(self.stack+4,event)
        self.u.reg_write(UC_X86_REG_ECX,self.perf+8)
        self.u.reg_write(UC_X86_REG_ESP,self.stack)
        self.u.emu_start(0x4f05c0,self.stop,count=100000)
        if self.first_unbound is None:
            assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
            assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+8
        else:
            assert self.u.reg_read(UC_X86_REG_EIP) in UNBOUND
        assert self.read(self.world+0xd4)==self.rng_before
        return {'first_unbound':self.first_unbound,
                'stage_bits':self.read(self.perf+0x10), 'turret_index_bits':self.read(self.perf+0x58),
                'rng_after':self.read(self.world+0xd4),'queues':self.snapshot()}

def cases():
    rows=[]
    for pc in [24,53,64]:
        for flags,props,event,matched,animator in itertools.product(
                [0,1],[None,0,2,0x102],[-1,0,1,2,3,4,5,6,-0x80000000,0x7fffffff],
                [True,False],[False,True]):
            rows.append({'pc':pc,'active':True,'thing_flags':flags,'property_flags':props,
                         'event':event & 0xffffffff,'stage':(event if matched else event^1)&0xffffffff,
                         'animator':animator})
        for event in [0,1,6,0xffffffff]:
            rows.append({'pc':pc,'active':False,'thing_flags':0,'property_flags':2,
                         'event':event,'stage':event,'animator':True})
    return rows

def main():
    rows=[]
    for c in cases():
        m=Machine(c)
        try: out=m.timed(c['event'])
        except BaseException as error:
            raise RuntimeError((c,hex(m.u.reg_read(UC_X86_REG_EIP)))) from error
        rows.append({'input':c,'before_queues':m.before,'rng_before':m.rng_before,'output':out})
    noops=[]
    for pc, (slot,vt),data,elapsed in itertools.product([24,53,64],NOOPS.items(),
                                                       [0,7,0xffffffff],[0,bits(.25),0x7fc00001]):
        c={'pc':pc,'active':False,'thing_flags':0,'property_flags':None,'stage':4,'animator':False}
        m=Machine(c)
        assert struct.unpack('<I',ORIGINAL.range(vt+0x14,4))[0]==0x4d1e10
        assert ORIGINAL.range(vt,32)==ACCEPTED.range(vt,32)
        m.write(m.perf+8,vt)
        before=bytes(m.u.mem_read(m.perf,0x80))
        m.call(0x4d1e10,m.perf+8,[elapsed,data])
        assert bytes(m.u.mem_read(m.perf,0x80))==before
        assert m.read(m.world+0xd4)==m.rng_before and m.snapshot()==m.before
        noops.append({'pc':pc,'slot':slot,'vtable':vt,'data':data,'elapsed_bits':elapsed})
    pins=[]
    for name,address,length in [('dispatchers',0x4f05b0,21),('performer',0x57f040,0x130),
        ('pause',0x4d03d0,29),('relative_schedule',0x50acc0,86),
        ('remove_matching',0x50b1c0,117),('noop',0x4d1e10,3),
        ('performer_vtable',PERFORMER_VTABLE,32)]:
        raw=ORIGINAL.range(address,length)
        assert raw==ACCEPTED.range(address,length)
        pins.append({'name':name,'address':address,'length':length,'bytes':raw.hex()})
    result={'schema':1,'method':__doc__,'original_sha256':sha(ORIGINAL.b),
        'v14_sha256':sha(ACCEPTED.b),'producer_sha256':sha(Path(__file__).read_bytes()),
        'cases':rows,'noops':noops,'pins':pins,
        'seams':['allocator6F7810','free6F7430','queue diagnostics/logs as phase6 harness'],
        'unbound_callees':{hex(k):v for k,v in UNBOUND.items()},
        'scope':'Actual native no-op Update and Performer signed dispatch/pause/stage0 first-effect prefix only; explicit first unbound callee stop, not full event/gameplay success.'}
    out=HERE/'native-dispatch.json'
    out.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'performer_rows':len(rows),'noop_rows':len(noops),'sha256':sha(out.read_bytes()),
        'outcomes':{str(k):sum(r['output']['first_unbound']==k for r in rows) for k in [None,*UNBOUND.values()]}}))

if __name__=='__main__':main()
