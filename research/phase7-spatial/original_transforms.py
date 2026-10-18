"""Native ordinary transform commits around actual spatial membership bodies.

Actor virtual+C8 and presentation sinks are required fixture boundaries. Their
entry snapshots and post-callback manager gating are observed; this does not
claim component/attachment effects inside the intercepted virtual callback.
"""
from pathlib import Path
import json, struct
from original_index import Index, shape, bits, W, H, original, modified
from unicorn.x86_const import *
HERE=Path(__file__).parent
class Transforms(Index):
    def __init__(self,grid,members,flags,cw,phase,available,change_phase):
        super().__init__(grid,members,flags,cw)
        self.callbacks=[];self.change_phase=change_phase
        self.w(0x200a000+0xd8,phase);self.w(0x200a000+0x130,0x200d000)
        self.w(0x200d004,0x200d100 if available else 0)
    def hook(self,u,pc,size,user):
        if pc in [0x4d0ee0,0x43b760,0x43b7d0]:
            sp=u.reg_read(UC_X86_REG_ESP)
            ptr=u.reg_read(UC_X86_REG_ECX) if pc==0x4d0ee0 else self.r(sp+4)
            self.callbacks.append({'action':{0x4d0ee0:'position_changed',0x43b760:'presentation_position',0x43b7d0:'presentation_facing'}[pc],
                'actor':self.inverse[ptr],'position':list(struct.unpack('<3I',u.mem_read(ptr+32,12))),
                'facing':list(struct.unpack('<3I',u.mem_read(ptr+44,12)))})
            if pc==0x4d0ee0 and self.change_phase is not None:self.w(0x200a000+0xd8,self.change_phase)
            u.reg_write(UC_X86_REG_EIP,self.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4+(0 if pc==0x4d0ee0 else 4));return
        super().hook(u,pc,size,user)
    def transform(self,action):
        self.reset(0);self.callbacks=[];self.trace=[]
        ptr=self.pointers[action['actor']];self.u.reg_write(UC_X86_REG_ECX,ptr)
        self.u.mem_write(0x200b500,W(*action['value']));self.w(self.STACK+4,0x200b500)
        self.running=True
        self.u.emu_start(0x4ee5c0 if action['op']=='position' else 0x4eebe0,self.STOP,count=10000000)
        self.running=False
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
        out=self.snapshot(self.trace)
        out['poses']={str(k):{'position':list(struct.unpack('<3I',self.u.mem_read(p+32,12))),
            'facing':list(struct.unpack('<3I',self.u.mem_read(p+44,12)))}for k,p in self.pointers.items()}
        out['callbacks']=self.callbacks
        out['world_phase']=self.r(0x200a000+0xd8)
        return out

def main():
    index=json.loads((HERE/'native-index.json').read_text());cases=[];windows={}
    faces=[[0.,-1.],[.6,.8],[1.,1.],[1e-5,-2.],[1.00001,0.],[3.,4.],[0.,0.]]
    for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
      for scenario in range(32):
        src=index['cases'][scenario%16];members=json.loads(json.dumps(src['members']))
        for m in members:
            m['actor']['flags'] |= (1<<12 if scenario&8 else 0)|(1<<17 if scenario&16 else 0)
        phase=6 if scenario%4==0 else 3;available=scenario%4!=1
        change_phase=6 if scenario%4==2 else 3 if scenario%4==0 else None
        machine=Transforms(src['grid'],members,src['flags'],cw,phase,available,change_phase)
        publish=not(scenario&4)
        if publish:
            for m in members:machine.apply({'op':'insert','actor':m['key']})
        face=faces[scenario%len(faces)]
        if face==[0.,0.] and not scenario&8:face=[.3,-.4]
        actions=[{'op':'position','actor':1,'value':list(map(bits,[6.25,6.75,4.]))},
            {'op':'facing','actor':2,'value':list(map(bits,face))},
            {'op':'position','actor':2,'value':list(map(bits,[10.75,11.25,-1.]))}]
        steps=[{'input':a,'native':machine.transform(a)}for a in actions]
        windows.update(machine.visited)
        cases.append({'precision':pc,'grid':src['grid'],'flags':src['flags'],'members':members,
            'publish':publish,'phase':phase,'available':available,'change_phase':change_phase,'steps':steps})
    pins=[]
    for a,n in sorted(windows.items()):
        raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
    out={'schema':1,'producer_sha256':H(Path(__file__).read_bytes()),'index_producer_sha256':H((HERE/'original_index.py').read_bytes()),
        'original_sha256':H(original.b),'v14_sha256':H(modified.b),
        'scope':__doc__,'seams':['malloc/free','Actor virtual+C8: captured pose, optional world phase mutation','43B760/43B7D0 presentation sinks: captured pose'],
        'cases':cases,'executed_windows':pins}
    path=HERE/'native-transforms.json';path.write_text(json.dumps(out,indent=2)+'\n')
    print(json.dumps({'sequences':len(cases),'steps':sum(len(c['steps'])for c in cases),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
