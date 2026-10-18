"""Execute original/v14 Research583FC0, group13 selection and517FA0 stores.

ActorState+28 and completed ActorData+558 head are typed input facts. No
gameplay seams. Record type/head reads and low/high writes to prove short
circuiting and store order. Null-page guards distinguish invalid source
or invalid selected state; no claim about arbitrary malformed pointers.
"""
from pathlib import Path
import sys,json,copy,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import Machine,MODES,ORIGINAL,ACCEPTED,pin,sha
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE

A,R,D,GD,TABLE=0x2010000,0x2011000,0x2012000,0x2020000,0x2021000
class Research(Machine):
    def __init__(self,cw,c,image):
        super().__init__(cw);self.trace=[];self.collect=False;self.c=c
        if image=='v14':
            old=(ORIGINAL.size+4095)&~4095;new=(ACCEPTED.size+4095)&~4095
            if new>old:self.uc.mem_map(ORIGINAL.base+old,new-old)
            self.uc.mem_write(ACCEPTED.base,ACCEPTED.data[:ACCEPTED.header_size])
            for rva,size,offset,_ in ACCEPTED.sections:
                if size:self.uc.mem_write(ACCEPTED.base+rva,ACCEPTED.data[offset:offset+size])
        self.put(0xb36530,GD);self.put(GD+0x2e4,TABLE)
        prior=ROOT/'research/phase7-actors/native-actor-states.json'
        masks=json.loads(prior.read_text())['registration']['masks']
        self.put(0xb3ac40+13*8,masks[13]);self.put(0xb3ac44+13*8,masks[13]>>32)
        for i,t in enumerate(c['types']):
            ptr=TABLE+0x100+i*0x40;self.put(TABLE+i*4,ptr);self.put(ptr+0x28,t)
        self.put(R+4,A);self.put(A+4,D if c['source_present'] else 0)
        self.put(A+0x100,c['state']);self.put(A+0x104,c['state']>>32)
        self.put(A+0xf0,c['actions']);self.put(A+0xf4,c['actions']>>32)
        self.put(D+0x558,c['head'])
        self.uc.hook_add(UC_HOOK_MEM_READ,self.read_event)
        self.uc.hook_add(UC_HOOK_MEM_WRITE,self.write_event)
        self.collect=True
    def read_event(self,u,access,addr,size,value,user):
        if not self.collect:return
        if addr==D+0x558:self.trace.append(['head',self.read(addr)])
        elif addr>=TABLE+0x100+0x28 and addr<TABLE+0x100+42*0x40 and (addr-TABLE-0x100)%0x40==0x28:
            self.trace.append(['type',(addr-TABLE-0x100)//0x40,self.read(addr)])
    def write_event(self,u,access,addr,size,value,user):
        if self.collect and addr in (A+0xf0,A+0xf4):
            self.trace.append(['write',addr-A,value])
    def run(self):
        outcome='ok'
        try:self.call(0x583fc0,[],R)
        except AssertionError:
            if not self.null_accesses:raise
            outcome='invalid_state' if self.c['state']&sum(1<<i for i in range(25,32))==0 else 'missing_source'
        return {'outcome':outcome,'actions':self.read(A+0xf0)|(self.read(A+0xf4)<<32),'trace':self.trace}

def cases():
    base={'name':'normal','state':1<<27,'actions':0xffffffffffffffff,'types':[0]*42,'head':0,'source_present':True}
    for i in range(25,32):
        for ty in (0,1,2,0xffffffff):
            for head in (0,0xdeadbeef):
                c=copy.deepcopy(base);c.update(name=f'state{i}_type{ty}_head{head}',state=1<<i,head=head)
                c['types'][i]=ty;yield c
    rng=random.Random(0x583fc0)
    for i in range(32):
        c=copy.deepcopy(base);c.update(name=f'varied{i}',state=rng.getrandbits(64)|(1<<27),actions=rng.getrandbits(64),head=rng.choice([0,1,0xffffffff]))
        c['types']=[rng.choice([0,1,2,0x80000000]) for _ in range(42)];yield c
    for ty in (0,1):
        c=copy.deepcopy(base);c.update(name=f'missing_source_type{ty}',source_present=False);c['types'][27]=ty;yield c
    for state in (0,1<<35):
        c=copy.deepcopy(base);c.update(name=f'invalid_group_{state}',state=state);yield c

def main():
    rows=[]
    for image in ('original','v14'):
        for pc,cw in MODES:
            for case in cases():
                machine=Research(cw,case,image)
                rows.append({'image':image,'precision':pc,'input':case,'output':machine.run()})
    windows=pin({'contributor':(0x583fc0,0x584007),'group':(0x4d14c0,0x4d150b),'group_select':(0x4d0820,0x4d0880),'write_action':(0x517fa0,0x518004)})
    previous=ROOT/'research/phase7-actors/native-actor-states.json'
    result={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),
            'state_registration_oracle_sha256':sha(previous.read_bytes()),'function_windows':windows,'cases':rows,'generator_sha256':sha(Path(__file__).read_bytes())}
    out=HERE/'native-research-actions.json';out.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'cases':len(rows),'sha256':sha(out.read_bytes())}))
if __name__=='__main__':main()
