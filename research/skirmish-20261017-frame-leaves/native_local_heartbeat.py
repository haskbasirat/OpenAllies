"""75B820 actual self-peer path, original/v14/all PC. Default immutable replay.

Typed initial Node fields and diagnostics are explicit seams. No remote peer,
takeover, request packet or successful diagnostic implementation is claimed.
"""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-net'))
from native_heartbeat import HbMachine,PE0,PE1,base,node,env,hist,PC,s32
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_ECX

class Machine(HbMachine):
    def __init__(self,image):
        self.active_capture=False
        super().__init__()
        if image=='v14':
            for vs,r,size,off in PE1.sections:
                if PE1.base+r in (0x401000,0x9f0000):self.u.mem_write(PE1.base+r,PE1.b[off:off+size])
        self.executed={};self.seams={};self.diag=[];self.stopped=None
        self.stubs[0x404f90]=self.diagnostic
    def diagnostic(self):
        fmt=self.text(self.arg(4))
        value=s32(self.arg(5)) if '%d' in fmt else None
        self.diag.append({'format':fmt,'index':value})
        if self.fail_diag:self.stopped='diagnostic';self.u.emu_stop()
        else:self.ret()
    def hook(self,u,pc,size,user):
        capture=self.active_capture
        super().hook(u,pc,size,user)
        if capture and 0x400000<=pc<0xb30000:
            if u.reg_read(UC_X86_REG_EIP)==pc and self.stopped is None:
                raw=bytes(u.mem_read(pc,size)).hex()
                assert PE0.range(pc,size)==PE1.range(pc,size),(hex(pc),raw)
                self.executed[f'{pc:08x}']=raw
            else:self.seams[f'{pc:08x}']=bytes(u.mem_read(pc,size)).hex()
    def pulse(self):
        self.w(self.stack,self.stop);self.u.reg_write(UC_X86_REG_ESP,self.stack)
        self.u.reg_write(UC_X86_REG_ECX,self.admin)
        self.u.emu_start(0x75b820,self.stop,count=500000)
        if self.stopped is None:assert self.u.reg_read(UC_X86_REG_EIP)==self.stop

def inputs():
    rows=[]
    for counter in (0,15,16,17):
        for behind in (0,128):
            for flag31 in (0,255):
                rows.append(dict(counter=counter,behind=behind,flag31=flag31,ratio=0x3f000000,vote=1,next=0,fail_diag=False,steps=1))
    for counter,next_ in ((0,-1),(16,0),(16,-1)):
        rows.append(dict(counter=counter,behind=255,flag31=128,ratio=0x38d1b717,vote=255,next=next_,fail_diag=True,steps=1))
    rows.append(dict(counter=0,behind=128,flag31=255,ratio=0x3f000000,vote=1,next=0,fail_diag=False,steps=36))
    return rows

def main():
    rows=[];pins={};seams={}
    for image in ('original','v14'):
        m=Machine(image)
        for pc in PC:
            for inp in inputs():
                st={'env':env(heartbeat_counter=inp['counter'],falling_behind=inp['behind']),
                    'node':node(base=base(player=1,flag_30=37,flag_31=inp['flag31'],flag_48=38,flag_49=39,flag_4a=40,flag_4b=41,
                                          vote=inp['vote'],rtt=0x3e800000,received_fraction=0x3f400000,frame_ratio=inp['ratio']),
                                vote_count=7,max_patience=0x3f800000,next_backup_decree=inp['next']),
                    'conns':[],'list':['node']}
                m.setup(st);m.fpu(pc);m.fail_diag=inp['fail_diag'];m.stopped=None;m.diag=[];m.active_capture=True
                steps=[]
                for i in range(inp['steps']):
                    m.pulse();steps.append({'after':m.snapshot(),'diagnostics':list(m.diag),'stop':m.stopped})
                    if m.stopped:break
                m.active_capture=False
                rows.append({'image':image,'precision':pc,'input':inp,'steps':steps})
        pins.update(m.executed);seams.update(m.seams)
    for i in range(len(rows)//2):
        a,b=rows[i],rows[i+len(rows)//2]
        assert a['steps']==b['steps'],(a['precision'],a['input'])
    out={'scope':'Actual75B820 self peer; supplied current fields and explicit diagnostic boundary; no native whole Host/clock/frontend proof',
         'rows':rows,'executed_pins':pins,'seam_entries_not_executed':seams}
    data=json.dumps(out,indent=2,sort_keys=True)+'\n';path=HERE/'native-local-heartbeat.json'
    if '--write' in sys.argv:path.write_text(data,encoding='utf-8',newline='\n')
    else:assert path.read_text(encoding='utf-8')==data,'immutable heartbeat replay differs'
    print(f'{len(rows)} rows / {sum(len(r["steps"]) for r in rows)} pulses / {len(pins)} executed pins PASS')

if __name__=='__main__':main()
