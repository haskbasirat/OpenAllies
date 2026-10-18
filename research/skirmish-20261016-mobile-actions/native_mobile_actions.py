"""Original575FB0/575C00/575BB0/575B70 and4D0230(false), all callees native.

477FC0 registers the actual state descriptors/group masks once. Its allocator,
CString and registry-transport seams are inherited from actor_states.py.
No contributor/predicate/health/group/property read is replaced. Fresh arena
words are fixture inputs; no game process or image file is written. Default
compares the evidence; --write creates it after original/v14 byte validation.
"""
from pathlib import Path
import sys, json, struct, random
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-actors'))
sys.path.insert(0,str(ROOT/'research/phase8-restoration'))
from actor_states import Machine as Base,P,V,H
import null_guard
from unicorn.x86_const import *

PCS={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
WATCH={0x575fb0:'contribute',0x575c00:'can_pack',0x575bb0:'can_unpack',0x575b70:'state_permits',
       0x4d14c0:'group13',0x529c30:'definition',0x4cf110:'properties',0x4d0230:'health_ratio'}

class Machine(Base):
    def __init__(self):
        self.watch=False
        self.trace=[]
        super().__init__()
        self.register()
        self.actor=0x20d0000
        self.component=0x20d0200
        self.definition=0x20d0300
        self.body=0x20d1000
        self.props=0x20d1100
        null_guard.install(self)
    def hook(self,u,pc,size,user):
        if self.watch and pc in WATCH:
            self.trace.append(WATCH[pc])
        super().hook(u,pc,size,user)
    def run(self,c):
        self.u.mem_write(self.actor,bytes(0x1400))
        self.w(self.component+4,self.actor)
        self.w(self.actor+4,self.definition)
        self.w(self.definition+0x548,0x20df000 if c['deployment'] else 0)
        self.w(self.definition+0x534,0x20df000 if c['unpack'] else 0)
        self.w(self.definition+0x54c,c['threshold'])
        self.w(self.actor+0x100,c['state']&0xffffffff)
        self.w(self.actor+0x104,c['state']>>32)
        self.w(self.actor+0xf0,c['actions']&0xffffffff)
        self.w(self.actor+0xf4,c['actions']>>32)
        self.w(self.actor+0x70,0x20de000 if c['mind'] else 0)
        if c['properties'] is not None:
            self.w(self.actor+0x78,self.props)
            self.w(self.props+0x1c,c['properties'])
        if c['body'] is not None:
            self.w(self.actor+0x60,self.body)
            self.w(self.body+0x10,c['body'][0])
            self.w(self.body+0x14,c['body'][1])
        self.u.reg_write(UC_X86_REG_FPCW,PCS[c['precision']])
        self.trace=[]
        self.watch=True
        if c['entry']=='contribute':
            self.invoke(0x575fb0,self.component)
            value=None
        elif c['entry']=='state_permits':
            self.invoke(0x575b70,self.component)
            value=bool(self.u.reg_read(UC_X86_REG_EAX)&0xff)
        elif c['entry']=='can_unpack':
            self.invoke(0x575bb0,self.component,int(c['check']))
            value=bool(self.u.reg_read(UC_X86_REG_EAX)&0xff)
        else:
            self.invoke(0x575c00,self.component,int(c['check']))
            value=bool(self.u.reg_read(UC_X86_REG_EAX)&0xff)
        self.watch=False
        assert not self.null_accesses,self.null_accesses
        return {'value':value,'actions':self.r(self.actor+0xf0)|(self.r(self.actor+0xf4)<<32),'calls':self.trace}

def bits(f):return struct.unpack('<I',struct.pack('<f',f))[0]
def base(**kw):
    c=dict(entry='contribute',check=True,precision='Double53',deployment=True,unpack=True,threshold=bits(.5),
           state=0,actions=0xfedcba9876543210,mind=False,properties=None,body=[bits(75),bits(100)])
    c.update(kw)
    return c
def cases():
    cases=[]
    # Every state13 selector, bit21, Properties presence, Mind and reference gates.
    for i,g in enumerate([None,25,26,27,28,29,30,31]):
        for state21 in [False,True]:
            for props in [None,0,0x800,0xffffffff]:
                for mind in [False,True]:
                    c=base(state=((1<<g)if g is not None else 0)|((1<<21)if state21 else 0),
                           properties=props,mind=mind)
                    cases.append(c)
    for deployment in [False,True]:
        for threshold in [0,0x80000000,bits(-.5),bits(.5),0x3f7fffff,bits(1),bits(2),
                          0x7f800000,0xff800000,0x7fc12345,0x7f812345]:
            for body in [None,[0,0],[bits(1),0],[bits(-1),0],[0,0x80000000],
                         [bits(1),bits(2)],[bits(3),bits(4)],[bits(-1),bits(2)],
                         [0x7f800000,0x7f800000],[0x7fc55555,bits(1)],
                         [bits(1),0x7f800000],[0xff800000,bits(1)]]:
                cases.append(base(deployment=deployment,threshold=threshold,body=body))
    # A PC24 rounded quotient exactly equals the binary32 threshold while
    # PC53/64 keep a slightly greater value: the strict comparison matters.
    for pc in PCS:
        for health,maximum in [(1,3),(1,10),(2,3),(7,10),(16777215,16777216)]:
            for threshold in [bits(health/maximum),bits(health/maximum)-1,bits(health/maximum)+1]:
                cases.append(base(precision=pc,body=[bits(health),bits(maximum)],threshold=threshold))
        for threshold in [0,bits(.5),bits(-.5),0x7fc12345]:
            cases.append(base(precision=pc,threshold=threshold,body=[0,0]))
    for check in [False,True]:
        for c in list(cases)[::13]:
            cases.append(dict(c,entry='can_pack',check=check))
    for state in [0,1<<27,1<<26,(1<<26)|(1<<27),(1<<27)|(1<<30),1<<21]:
        cases.append(base(entry='state_permits',state=state))
        for check in [False,True]:
            for unpack in [False,True]:
                cases.append(base(entry='can_unpack',state=state,check=check,unpack=unpack))
    return cases

def main():
    m=Machine()
    rows=[{'input':c,'output':m.run(c)}for c in cases()]
    for a,b in m.observed.items():assert b==P.range(a,len(b))==V.range(a,len(b)),hex(a)
    doc={'schema':1,'passed':True,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),
         'original_sha256':H(P.b),'v14_sha256':H(V.b),
         'dependencies':{str(p.relative_to(ROOT)).replace('\\','/'):H(p.read_bytes())for p in [
             ROOT/'research/phase7-actors/actor_states.py',ROOT/'research/phase6-mapworld/native_load_oracle.py',
             ROOT/'research/phase8-restoration/null_guard.py']},
         'constants':{'9F32DC':0,'9F32E0':0x3f800000},
         'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(m.observed.items())],
         'cases':rows}
    p=HERE/'native-mobile-actions.json'
    if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
    else:assert json.loads(p.read_text())==doc,'native fixture changed'
    print(json.dumps({'cases':len(rows),'instructions':len(m.observed),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
