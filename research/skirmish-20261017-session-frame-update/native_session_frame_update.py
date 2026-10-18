"""Read-only original/v14776540 proof, with precise unsupported boundaries.

775DB0(NULL/unfinished),76E3D0(nonpositive/nohost),76C4B0 and74B900/74BB40
run without successful child seams. Positive task/log, countdown/readiness,
and migration/request STOP at first unbound entry. No callback is faked.
Constructor proof runs actual xorEBX and actual2E0 store as two intervals;
whole75CE80 is a static byte pin, not a whole native constructor execution.
"""
from pathlib import Path
import sys, json, itertools
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT/'research/phase8-ai'))
import native_machine as n
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import *
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

STOP_ENTRIES = {0x404f60:'finished_task_diagnostic', 0x76c960:'countdown_readiness', 0x76b570:'migration_request'}
CALLS = (0x776540,0x775db0,0x76e3d0,0x76c4b0,0x74b900,0x74bb40)

class Machine(n.Machine):
    def __init__(self, image, precision):
        super().__init__(precision)
        pe = n.original if image == 'original' else n.modified
        if image != 'original':
            old_size=(n.original.image_size+0xfff)&~0xfff
            new_size=(pe.image_size+0xfff)&~0xfff
            if new_size>old_size:self.u.mem_map(pe.base+old_size,new_size-old_size)
            for _, rva, raw, off in pe.sections:
                self.u.mem_write(pe.base+rva, pe.b[off:off+raw])
        self.session = self.alloc(0x180)
        self.node = self.alloc(0x300)
        self.task = self.alloc(0x30)
        self.w(0xb36558, self.node)
        self.calls = []
        self.stop = None
        self.reads = []
        self.fields = {
            self.session+0x124:'Session124',self.session+0x11c:'Session11C',
            self.session+0x16c:'Session16C',self.session+0xe4:'SessionE4',
            self.node+0x2c4:'Node2C4',self.node+0x2e0:'Node2E0',self.task+0x14:'Task14',
        }
        self.u.hook_add(UC_HOOK_MEM_READ, self.read)
        for a in CALLS:
            self.observe(a,lambda m,a=a:m.calls.append(f'{a:08X}'[-6:]))
        for a,label in STOP_ENTRIES.items():
            self.seam(a,lambda m,label=label:m.boundary(label))
    def read(self,u,access,address,size,value,user):
        if address in self.fields:
            self.reads.append(self.fields[address])
    def boundary(self,label):
        self.stop=label
        self.u.emu_stop()
    def run(self,case):
        self.w(self.session+0x124,0 if case['task_status'] is None else self.task)
        if case['task_status'] is not None:self.w(self.task+0x14,case['task_status'])
        self.w(self.node+0x2c4,self.node if case['host'] else 0)
        self.w(self.node+0x2e0,case['takeover'])
        self.w(self.session+0x11c,case['countdown'])
        self.w(self.session+0xe4,case['state'])
        self.wb(self.session+0x16c,int(case['sent']))
        self.u.reg_write(UC_X86_REG_FPCW,n.PRECISIONS[self.precision])
        self.u.reg_write(UC_X86_REG_FPSW,0)
        self.w(n.STACK,n.STOP)
        self.u.reg_write(UC_X86_REG_ESP,n.STACK)
        self.u.reg_write(UC_X86_REG_ECX,self.session)
        saved={r:0x11111111*(i+1) for i,r in enumerate(n.CALLEE_SAVED)}
        for r,v in saved.items():self.u.reg_write(r,v)
        self.u.emu_start(0x776540,n.STOP,count=10000)
        assert not self.null_accesses,self.null_accesses
        if self.stop is None:
            assert self.u.reg_read(UC_X86_REG_EIP)==n.STOP
            assert self.u.reg_read(UC_X86_REG_ESP)==n.STACK+4
            assert all(self.u.reg_read(r)==v for r,v in saved.items())
        return {'stop':self.stop,'sent_after':bool(self.rb(self.session+0x16c)),
                'countdown_after':(self.r(self.session+0x11c)^0x80000000)-0x80000000,
                'task_retained':self.r(self.session+0x124)==(0 if case['task_status'] is None else self.task),
                'calls':self.calls,'reads':self.reads}

def cases():
    out=[]
    for host,sent,takeover,state in itertools.product([False,True],[False,True],[0,1,0xffffffff],[0,1,2]):
        out.append(dict(task_status=None,host=host,countdown=0,sent=sent,takeover=takeover,state=state))
    for host,sent,countdown in itertools.product([False,True],[False,True],[-2147483648,-1,1,2147483647]):
        out.append(dict(task_status=None,host=host,countdown=countdown,sent=sent,takeover=1,state=1))
    for status,host,countdown in itertools.product([0,1,2,3,0xffffffff],[False,True],[0,1]):
        out.append(dict(task_status=status,host=host,countdown=countdown,sent=False,takeover=1,state=1))
    out.append(dict(task_status=None,host=True,countdown=0,sent=False,takeover=1,state=0xffffffff))
    return out

def main():
    rows=[];visited={};constructors=[]
    for image,p in itertools.product(['original','v14'],n.PRECISIONS):
        for case in cases():
            m=Machine(image,p);result=m.run(case)
            rows.append(dict(image=image,precision=p,input=case,**result))
            visited.update(m.visited)
        for poison in [1,0xffffffff]:
            m=Machine(image,p)
            m.u.reg_write(UC_X86_REG_FPCW,n.PRECISIONS[p])
            m.u.reg_write(UC_X86_REG_FPSW,0)
            m.w(m.node+0x2e0,poison)
            m.u.reg_write(UC_X86_REG_ESI,m.node)
            m.u.reg_write(UC_X86_REG_EBX,poison)
            m.u.emu_start(0x75ceb2,0x75ceb4,count=1)
            assert m.u.reg_read(UC_X86_REG_EBX)==0
            m.u.emu_start(0x75d1ae,0x75d1b4,count=1)
            assert m.r(m.node+0x2e0)==0
            constructors.append(dict(image=image,precision=p,poison=poison,takeover_after=m.r(m.node+0x2e0)))
            visited.update(m.visited)
    lhs=[{k:v for k,v in r.items() if k!='image'} for r in rows if r['image']=='original']
    rhs=[{k:v for k,v in r.items() if k!='image'} for r in rows if r['image']=='v14']
    assert lhs==rhs
    pins=[]
    for a,b in sorted(visited.items()):
        assert n.original.range(a,len(b))==b==n.modified.range(a,len(b))
        assert a not in STOP_ENTRIES
        pins.append(dict(address=f'{a:08X}',bytes=b.hex()))
    static=[]
    for a,z,label in [(0x75ce80,0x340,'75CE80 ctor context, not whole-executed'),
                      (0x751770,0x1d0,'751770 reset does not overwrite2E0'),
                      (0x75c6c0,0x10,'75C6C0 is destructor, not reset')]:
        b=n.original.range(a,z);assert b==n.modified.range(a,z)
        static.append(dict(address=f'{a:08X}',length=z,sha256=n.sha(b),label=label))
    #The ctor's only direct EBX writer before2E0 is the pinned zeroing xor;
    #all intervening calls retain the native callee-saved register contract.
    cs=Cs(CS_ARCH_X86,CS_MODE_32);cs.detail=True
    writes=[]
    for i in cs.disasm(n.original.range(0x75ceb2,0x75d1ae-0x75ceb2),0x75ceb2):
        if i.mnemonic not in ('cmp','test','push') and i.op_str.split(',')[0] in ('ebx','bx','bl','bh'):
            writes.append(f'{i.address:08X}')
    assert writes==['0075CEB2'],writes
    out=dict(scope='Bounded recurring776540, actual NULL/unfinished tasks and inactive countdown; stops at first positive child, no successful seams',
             original_sha256=n.sha(n.original.b),v14_sha256=n.sha(n.modified.b),
             producer_sha256=n.sha(Path(__file__).read_bytes()),rows=rows,constructors=constructors,
             executed_instruction_pins=pins,required_entries={f'{a:08X}':label for a,label in STOP_ENTRIES.items()},
             static_windows=static,ctor_ebx_direct_writers=writes)
    encoded=(json.dumps(out,indent=2)+'\n').encode()
    path=HERE/'native-session-frame-update.json'
    if '--write' in sys.argv:path.write_bytes(encoded)
    else:assert path.read_bytes()==encoded,'immutable fixture differs'
    print('PASS',len(rows),'native rows;',len(constructors),'ctor interval rows;',len(pins),'executed pins; zero successful seams')

if __name__=='__main__':main()
