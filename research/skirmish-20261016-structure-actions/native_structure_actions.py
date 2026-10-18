"""Original/v14 Structure589A60, actual4CE980/588990/4CE9A0 virtuals.

Only parent4D1350 is a required gameplay seam. It records callback order,
changes explicit fixture fields, and can stop with an injected dependency
failure after those changes. That failure is not a claimed native exception.
No source definition is read by this contributor. Native queue+18 is supplied
as a raw count; no queued command execution is claimed.
"""
from pathlib import Path
import sys,json,copy,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import Machine,MODES,ORIGINAL,ACCEPTED,pin,sha
from unicorn.x86_const import *
A,B,P,S,SB,SET,VT=0x2010000,0x2011000,0x2012000,0x2013000,0x2013100,0x2014000,0x2015000
class Failure(Exception):pass
class Structure(Machine):
    def __init__(self,cw,c,image):
        super().__init__(cw);self.c=copy.deepcopy(c)
        if image=='v14':
            old=(ORIGINAL.size+4095)&~4095;new=(ACCEPTED.size+4095)&~4095
            if new>old:self.uc.mem_map(ORIGINAL.base+old,new-old)
            self.uc.mem_write(ACCEPTED.base,ACCEPTED.data[:ACCEPTED.header_size])
            for rva,size,offset,_ in ACCEPTED.sections:
                if size:self.uc.mem_write(ACCEPTED.base+rva,ACCEPTED.data[offset:offset+size])
        self.trace=[];self.virtual_calls=[]
        self.put(VT+0x158,0x4ce980);self.put(VT+0x15c,0x4ce9a0)
        for address in [A,B,P]:self.put(address,VT)
        self.put(S+4,A);self.put(S+0x10,P if c['parent'] else 0);self.put(S+0x18,c['queue'])
        self.put(A+0x94,S if c['structure'] else 0)
        self.put(B+0x94,SB);self.put(SB+4,B)
        self.put(P+0x98,SET if c['settlement'] else 0)
        self.put(SET+0x14,{'self':A,'other':B,'null':0}[c['center']])
        for address,key in [(A,'actions'),(B,'other_actions'),(P,'parent_actions')]:self.mask_set(address,c[key])
        self.seam(0x4d1350,self.rebuild)
        self.uc.hook_add(1<<2,self.observe) # UC_HOOK_CODE
    def mask(self,p):return self.read(p+0xf0)|(self.read(p+0xf4)<<32)
    def mask_set(self,p,n):self.put(p+0xf0,n);self.put(p+0xf4,n>>32)
    def label(self,p):return {A:1,B:2,P:3}.get(p,p)
    def observe(self,u,pc,size,user):
        if pc in (0x4ce980,0x588990,0x4ce9a0):
            self.virtual_calls.append(hex(pc))
        if pc in (0x589aaa,0x589ad5,0x589af6,0x589b05):
            owner=self.read(S+4)
            self.trace.append(['write',self.label(owner),self.mask(owner)])
    def rebuild(self,m,esp):
        owner=self.uc.reg_read(UC_X86_REG_ECX);assert owner==P
        self.trace.append(['rebuild',3])
        self.mask_set(P,self.c['rebuilt_actions'])
        if self.c.get('rebind'):self.put(S+4,B)
        if self.c.get('change_queue') is not None:self.put(S+0x18,self.c['change_queue'])
        if self.c.get('change_actions') is not None:self.mask_set(A,self.c['change_actions'])
        if self.c.get('change_parent'):self.put(S+0x10,B)
        if self.c.get('fail'):raise Failure()
        return 0,0
    def run(self):
        outcome='ok'
        try:self.call(0x589a60,[],S)
        except Failure:outcome='dependency_failure'
        except AssertionError:
            if self.null_accesses:outcome='native_invalid_settlement'
            else:raise
        return {'outcome':outcome,'actions':self.mask(A),'other_actions':self.mask(B),
                'parent_actions':self.mask(P),'queue':self.read(S+0x18),'owner':self.label(self.read(S+4)),
                'trace':self.trace,'virtual_calls':self.virtual_calls}
def cases():
    r=random.Random(0x589a60)
    base=dict(name='base',structure=True,parent=True,settlement=True,center='self',queue=0,
              actions=0xffffffffffffffff,other_actions=0x123456789abcdef0,
              parent_actions=0x2000,rebuilt_actions=0xabcdef0123456789)
    for structure in [False,True]:
        for parent in [False,True]:
            for settlement in [False,True]:
                for center in ['self','other','null']:
                    for queue in [0,1,0xffffffff]:
                        c=dict(base,structure=structure,parent=parent,settlement=settlement,center=center,queue=queue,
                               actions=r.getrandbits(64),other_actions=r.getrandbits(64),parent_actions=r.getrandbits(64),rebuilt_actions=r.getrandbits(64))
                        c['name']='gates_'+str(len(str(c)))+'_'+str(r.getrandbits(32));yield c
    for changes in [dict(rebind=True),dict(change_queue=4),dict(change_queue=0,queue=4),
                    dict(change_actions=0x1000),dict(change_parent=True),dict(fail=True),
                    dict(rebind=True,change_queue=1,change_actions=0xffff,fail=True),
                    dict(rebind=True,change_queue=1,change_actions=0xffff),
                    dict(rebuilt_actions=1<<13),dict(rebuilt_actions=1<<18),dict(rebuilt_actions=1<<63)]:
        yield dict(base,name='mutation_'+str(changes),**changes)
def main():
    rows=[]
    for image in ['original','v14']:
        for pc,cw in MODES:
            for c in cases():rows.append({'image':image,'precision':pc,'input':c,'output':Structure(cw,c,image).run()})
    out={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),
         'producer_sha256':sha(Path(__file__).read_bytes()),
         'harness_sha256':sha((ROOT/'research/phase8-economy/harness.py').read_bytes()),
         'function_windows':pin({'contributor':(0x589a60,0x589b06),'center':(0x4ce980,0x4ce99c),
                                 'parent':(0x4ce9a0,0x4ce9bd),'structure_center':(0x588990,0x5889ae)}),
         'cases':rows}
    path=HERE/'native-structure-actions.json'
    if '--write'in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
    else:assert json.loads(path.read_text())==out,'native fixture differs'
    print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
