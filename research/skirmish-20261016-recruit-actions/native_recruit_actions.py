"""Original Recruit582C40 action contributor with explicit5827B0 eligibility.

Executes582C40,4D14C0,4D0820,529C30,517FA0 unchanged. State group mask is
the pinned original477FC0 registration oracle. ActorState+28 and Recruit
list contents are typed input facts. Eligibility5827B0 is the only gameplay
seam; it records arguments/results and can mutate live fields or stop with
an explicitly injected dependency failure (not a native exception).
"""
from pathlib import Path
import sys,json,random,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import Machine,MODES,ORIGINAL,ACCEPTED,pin,sha
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_ECX

A,B,R,D,DB,SET,GD,TABLE=0x2010000,0x2011000,0x2012000,0x2013000,0x2014000,0x2015000,0x2020000,0x2021000
NODES,DEFS=0x2030000,0x2040000
class Failure(Exception):pass

class Recruit(Machine):
    def __init__(self,cw,c,image):
        super().__init__(cw);self.c=copy.deepcopy(c);c=self.c
        if image=='v14':
            old_size=(ORIGINAL.size+4095)&~4095;new_size=(ACCEPTED.size+4095)&~4095
            if new_size>old_size:self.uc.mem_map(ORIGINAL.base+old_size,new_size-old_size)
            self.uc.mem_write(ACCEPTED.base,ACCEPTED.data[:ACCEPTED.header_size])
            for rva,size,offset,_ in ACCEPTED.sections:
                if size:self.uc.mem_write(ACCEPTED.base+rva,ACCEPTED.data[offset:offset+size])
        self.collect=False;self.trace=[]
        self.put(0xb36530,GD);self.put(GD+0x2e4,TABLE)
        prior=ROOT/'research/phase7-actors/native-actor-states.json'
        masks=json.loads(prior.read_text())['registration']['masks']
        self.put(0xb3ac40+13*8,masks[13]);self.put(0xb3ac44+13*8,masks[13]>>32)
        for i,t in enumerate(c['types']):
            p=TABLE+0x100+i*0x40;self.put(TABLE+i*4,p);self.put(p+0x28,t)
        for p,state,actions,definition in [(A,c['state'],c['actions'],D),(B,c['parent_state'],c['parent_actions'],DB)]:
            self.put(p+4,definition);self.put(p+0x100,state);self.put(p+0x104,state>>32);self.setmask(p,actions)
        self.put(A+0x98,SET if c['settlement'] else 0)
        self.put(SET+0x14,B if c['settlement_target'] else 0)
        self.put(R+4,A);self.put8(R+0x10,c['byte10']);self.put8(R+0x11,c['byte11'])
        self.put(D+0x4ec,NODES if c['entries'] else 0);self.put(DB+0x4ec,0)
        for i,e in enumerate(c['entries']):
            node=NODES+i*8;d=DEFS+i*0x600
            self.put(node,d);self.put(node+4,node+8 if i+1<len(c['entries']) else 0)
            self.put(d+0x184,e['location']);self.put(d+0x2f0,e['layout'])
        self.seam(0x5827b0,self.eligible)
        self.uc.hook_add(UC_HOOK_MEM_WRITE,self.write)
        self.collect=True

    def mask(self,a):return self.read(a+0xf0)|(self.read(a+0xf4)<<32)
    def setmask(self,a,bits):self.put(a+0xf0,bits);self.put(a+0xf4,bits>>32)
    def write(self,u,access,address,size,value,user):
        if self.collect and address in (A+0xf0,B+0xf0):
            a=address-0xf0
            self.trace.append(['write',1 if a==A else 2,value|(self.read(a+0xf4)<<32)])
    def eligible(self,m,esp):
        assert self.uc.reg_read(UC_X86_REG_ECX)==R
        d,layout,requirements,diagnostic=[self.arg(i) for i in range(4)]
        i=(d-DEFS)//0x600
        assert d==DEFS+i*0x600 and self.c['entries'][i]['layout']==layout
        assert requirements==1 and diagnostic==0
        owner=1 if self.read(R+4)==A else 2
        self.trace.append(['eligibility',i,owner,layout,requirements,diagnostic])
        if self.c.get('fail_at')==i:raise Failure()
        self.collect=False
        if self.c.get('rebind_at')==i:self.put(R+4,B)
        if self.c.get('skip_at')==i:
            self.put(NODES+i*8+4,NODES+2*8 if len(self.c['entries'])>2 else 0)
        if self.c.get('flip_class_at')==i:self.put(d+0x184,int(not self.c['entries'][i]['location']))
        if self.c.get('change_actions_at')==i:self.setmask(A,self.mask(A)^(1<<42))
        self.collect=True
        return self.c['entries'][i]['result'],16
    def run(self):
        outcome='ok'
        try:self.call(0x582c40,[],R)
        except Failure:outcome='dependency_failure'
        except AssertionError:
            if self.null_accesses:outcome='native_invalid_state'
            else:raise
        return {'outcome':outcome,'actions':self.mask(A),'parent_actions':self.mask(B),'trace':self.trace}

def cases():
    rng=random.Random(0x582c40)
    base={'name':'base','state':1<<27,'parent_state':1<<27,'actions':0xffffffffffffffff,'parent_actions':0x123456789abc,
          'types':[0]*42,'settlement':True,'settlement_target':False,'byte10':1,'byte11':1,
          'entries':[{'location':0,'layout':401,'result':0},{'location':1,'layout':402,'result':0}]}
    yield base
    for state in range(25,32):
        for stype in (0,1,2):
            for settlement,target in ((False,False),(True,False),(True,True)):
                c=copy.deepcopy(base);c.update(name=f'state_{state}_type_{stype}_settlement_{settlement}_{target}',state=1<<state,settlement=settlement,settlement_target=target)
                c['types'][state]=stype;yield c
    for i in range(80):
        c=copy.deepcopy(base);c.update(name=f'varied_{i}',state=(1<<rng.randrange(25,32))|(rng.randrange(2)<<20),parent_state=1<<rng.randrange(25,32),
                actions=rng.getrandbits(64),parent_actions=rng.getrandbits(64),settlement=bool(i%2),settlement_target=bool(i%3),byte10=[0,1,255][i%3],byte11=[0,7][i%2])
        c['types']=[rng.randrange(3) for _ in range(42)]
        c['entries']=[{'location':rng.choice([0,1,0x80000000]),'layout':0 if j%4==0 else 100+j,'result':rng.choice([0,1,2,3,0xffffffff])} for j in range(i%7)]
        yield c
    for key in ('fail_at','rebind_at','skip_at','flip_class_at','change_actions_at'):
        c=copy.deepcopy(base);c.update(name=key);c[key]=0
        c['entries'].append({'location':0,'layout':403,'result':1});yield c
    c=copy.deepcopy(base);c.update(name='no_selected_state',state=0);yield c
    c=copy.deepcopy(base);c.update(name='multibit_group',state=(1<<25)|(1<<27)|(1<<31));yield c

def main():
    rows=[]
    for image in ('original','v14'):
        for precision,cw in MODES:
            for c in cases():
                m=Recruit(cw,c,image);rows.append({'image':image,'precision':precision,'input':c,'output':m.run()})
    ranges={'contributor':(0x582c40,0x582e80),'group':(0x4d14c0,0x4d150b),'group_select':(0x4d0820,0x4d0880),
            'definition':(0x529c30,0x529c37),'action_write':(0x517fa0,0x51800f)}
    previous=ROOT/'research/phase7-actors/native-actor-states.json'
    report={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),
            'state_registration_oracle_sha256':sha(previous.read_bytes()),'function_windows':pin(ranges),
            'cases':rows,'generator_sha256':sha(Path(__file__).read_bytes())}
    path=HERE/'native-recruit-actions.json';path.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
