"""Actual752AE0 wrapper; two explicit59ABA0 outputs and76F790 callback seam.
Default immutable replay. Successful World callback is a typed seam, not a
wholeWorld execution claim. Partial clock/World failures preserve history.
"""
from pathlib import Path
import json,sys,struct,itertools
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_local_heartbeat import Machine as Base
from native_heartbeat import env,node,PC,s32
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EIP,UC_X86_REG_ECX
def f32(x):return struct.unpack('<I',struct.pack('<f',x))[0]
class Machine(Base):
    def __init__(self,image):
        super().__init__(image);self.active_capture=False
        self.stubs[0x59aba0]=self.timer;self.stubs[0x76f790]=self.world
    def timer(self):
        index=self.timer_index;self.timer_index+=1;self.events.append('time'+str(index))
        if self.failure=='time'+str(index):self.stopped=self.failure;self.u.emu_stop()
        else:self.ret(0,self.times[index])
    def world(self):
        self.events.append('world')
        if self.failure=='world':self.stopped='world';self.u.emu_stop()
        else:self.ret()
    def hook(self,u,pc,size,user):
        if self.active_capture and pc==0x752b38:self.start=self.r(u.reg_read(UC_X86_REG_ESP)+0x10)
        super().hook(u,pc,size,user)
    def raw_history(self,begin,values,capacity):
        a=self.admin+0x238;cap=max(capacity,len(values),1);arr=self.alloc(cap*4)
        self.w(a,begin);self.w(a+4,begin+len(values));self.w(a+8,cap);self.w(a+12,arr)
        for i,bits in enumerate(values):self.w(arr+((begin+i+0x80000000)&0xffffffff)%cap*4,bits)
    def result(self):
        a=self.admin+0x238;b=s32(self.r(a));e=s32(self.r(a+4));cap=self.r(a+8);arr=self.r(a+12)
        return {'begin':b,'end':e,'values':[self.r(arr+((i+0x80000000)&0xffffffff)%cap*4) for i in range(b,e)],
                'ratio':self.r(self.admin+0x60),'events':list(self.events),'stop':self.stopped,'start':self.start}
    def run(self,case):
        self.setup({'env':env(),'node':node(),'conns':[],'list':['node']})
        self.raw_history(case['begin'],case['values'],case['capacity']);self.w(self.admin+0x60,case['ratio'])
        self.w(self.admin+0x268,0);self.w(self.admin+0x26c,0)
        self.failure=case['failure'];self.events=[];self.start=None;self.stopped=None
        outputs=[];self.active_capture=True
        for pair in case['times']:
            self.times=pair;self.timer_index=0
            self.w(self.stack,self.stop);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.admin)
            self.u.emu_start(0x752ae0,self.stop,count=500000)
            if self.stopped is None:assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
            outputs.append(self.result())
            if self.stopped:break
        self.active_capture=False;return outputs
def cases():
    out=[]
    for begin,n,t0,delta in itertools.product((0,17),(0,9,15,16),(0,1,0x7fffffff,0xffffffff),(0,1,16,1000)):
        out.append(dict(begin=begin,values=[f32((i%5)*.001) for i in range(n)],capacity=4,ratio=f32(.0001),failure=None,
                        times=[[t0,(t0+delta)&0xffffffff]]))
    for failure in ('time0','world','time1'):
        out.append(dict(begin=0,values=[f32(.01)]*9,capacity=16,ratio=f32(.0001),failure=failure,times=[[100,117]]))
    out.append(dict(begin=0,values=[],capacity=4,ratio=f32(.0001),failure=None,times=[[i*100,i*100+5] for i in range(36)]))
    return out
def main():
    rows=[];pins={};seams={}
    for image in ('original','v14'):
        m=Machine(image)
        for p in PC:
            for c in cases():
                m.fpu(p);rows.append({'image':image,'precision':p,'input':c,'steps':m.run(c)})
        pins.update(m.executed);seams.update(m.seams)
    for a,b in zip(rows[:len(rows)//2],rows[len(rows)//2:]):assert a['steps']==b['steps']
    out={'scope':'Actual752AE0; explicit59ABA0u32 and76F790 callback seams, recorder/playback null; finite initialized nonnegative raw-history windows',
         'rows':rows,'executed_pins':pins,'seam_entries_not_executed':seams}
    data=json.dumps(out,sort_keys=True,indent=2)+'\n';path=HERE/'native-frame-duration.json'
    if '--write' in sys.argv:path.write_text(data,encoding='utf-8',newline='\n')
    else:assert path.read_text(encoding='utf-8')==data,'immutable duration replay differs'
    print(f'{len(rows)} rows / {sum(len(r["steps"]) for r in rows)} wrappers / {len(pins)} executed pins PASS')
if __name__=='__main__':main()
