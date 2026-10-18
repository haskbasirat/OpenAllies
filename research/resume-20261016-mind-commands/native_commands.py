"""Original saved actor command Load 4DACE0 and Element Mind Load 536340.

Executes native constructor 4D9C80, named reference reader control flow,
4DACE0 field/version gates, and 52C580 pending/current reference ownership.
Explicit seams: stream bytes, CString allocation/read, registry lookup by a
provided name, allocator/free, and inherited constructor ability-jitter seam.
ParameterSet bit5 fails explicitly in the Rust port; no proof claimed here.
All executed original instructions are checked against the unchanged v14 exe.
Page-zero guard is inherited. Fields contain no floating-point arithmetic.
"""
from pathlib import Path
import sys, json, struct, random, hashlib
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-restoration'))
from native_mind_load import Machine as Base, Outcome, P, V, VISITED, METADATA, STATE_NAMES, W, synthetic
from unicorn.x86_const import *

H=lambda b:hashlib.sha256(b).hexdigest()
MANAGERS={'definition':0xb3aa58,'parameter_definition':0xb3ab08,'extra_definition':0xb3aa2c}
REF_OFF={8:'definition',0x14:'object_definition',0x2c:'parameter_definition',0x38:'extra_definition'}

class Machine(Base):
    def __init__(self,payload,pc=0x27f):
        super().__init__(payload,pc)
        self.refs={};self.ref_names={};self.lookups=[]
        self.registry={}
        for key,addr in MANAGERS.items():
            value=self.alloc(64);self.registry[value]=key;self.write(addr,value)
        self.created=[]

    def hook(self,u,pc,size,arg):
        if P.base <= pc < P.base+P.image_size:
            raw=P.range(pc,size);assert raw==V.range(pc,size),hex(pc);VISITED[pc]=raw.hex()
        if pc in (0x4daaa0,0x4c2e60):
            return # These named readers must run, not inherited unresolved seams.
        if pc in (0x463fa0,0x48d500,0x4bdba0):
            key='object_definition' if pc==0x4bdba0 else self.registry[u.reg_read(UC_X86_REG_ECX)]
            units=tuple(self.wtext(self.arg()))
            if pc==0x4bdba0:assert self.arg(1)==0x2b
            self.lookups.append({'kind':key,'units':list(units),'offset':self.r.at})
            if units==tuple(map(ord,'missing')):
                result=0
            else:
                identity=(key,units)
                if identity not in self.refs:
                    result=self.alloc(64);self.refs[identity]=result;self.ref_names[result]={'kind':key,'units':list(units)}
                result=self.refs[identity]
            self.ret(8 if pc==0x4bdba0 else 4,value=result)
            return
        if pc==0x41dfb0:
            raise Outcome('unknown_reference')
        if pc==0x4d9c80:
            self.created.append(u.reg_read(UC_X86_REG_ECX))
        super().hook(u,pc,size,arg)

    def fields(self,p):
        out={}
        for off,key in REF_OFF.items():
            ptr=self.read(p+off);out[key]=self.ref_names[ptr]['units'] if ptr else None
        for off,key in [(0xc,'flags'),(0x10,'word10'),(0x28,'scalar_bits')]:out[key]=self.read(p+off)
        out['position_bits']=[self.read(p+0x18),self.read(p+0x1c)]
        out['auxiliary_bits']=[self.read(p+0x20),self.read(p+0x24)]
        out['boolean']=self.u.mem_read(p+0x30,1)[0]
        out['trailing']=self.u.mem_read(p+0x3c,1)[0]
        out['reference_count']=self.read(p+4)
        out['owned_parameters_null']=self.read(p+0x34)==0
        return out

    def command(self):
        obj=self.alloc(64);self.u.mem_write(obj,b'\xcd'*64)
        self.call(0x4d9c80,obj,[])
        try:
            self.call(0x4dace0,obj,[self.world]);outcome='loaded'
        except Outcome as e:outcome=str(e)
        return {'outcome':outcome,'consumed':self.r.at,'fields':self.fields(obj),'lookups':self.lookups}

    def mind(self):
        for off,meta in METADATA.items():self.write(meta,self.string(STATE_NAMES[off].encode('utf-16-le')))
        owner=self.alloc(0x200);self.write(owner+4,self.alloc(0x800))
        obj=self.alloc(0x1c0);self.call(0x535ba0,obj,[owner])
        self.call(0x536340,obj,[self.world])
        pending=[];node=self.read(obj+0x18);last=0
        while node:
            pending.append(self.fields(self.read(node)));last=node;node=self.read(node+4)
        current=self.read(obj+0x20)
        assert self.read(obj+0x1c)==last
        stack=[];node=self.read(obj+0x10)
        while node:stack.append(self.read(node)-obj);node=self.read(node+4)
        return {'outcome':'loaded','consumed':self.r.at,'pending':pending,
                'current':self.fields(current) if current else None,'stack':stack,
                'word24':self.read(obj+0x24),'word28':self.read(obj+0x28),'word2c':self.read(obj+0x2c),
                'distinct_allocations':len(set(self.created))==len(self.created),
                'command_allocations':len(self.created),'lookups':self.lookups}

def text(value):
    if value is None: value=[]
    if isinstance(value,str):value=list(map(ord,value))
    return struct.pack('<H',len(value))+b''.join(struct.pack('<H',v) for v in value)

def command(version=3,seed=0,empty=False):
    rng=random.Random(seed);name=None if empty else 'move'
    flags=rng.getrandbits(32)&~32
    p=W(version)+text(name)+W(flags,rng.getrandbits(32))+text(None if empty else 'usa_infantry')
    p+=W(*[rng.getrandbits(32) for _ in range(5)])
    p+=bytes([rng.choice([0,1,2,255])])
    if version>=3:p+=bytes([rng.choice([0,1,2,255])])
    if version>=1:p+=text(None if empty else 'formation')
    if version>=2:p+=text(None if empty else 'property')
    return p

def mind_payload(pending,current,baseversion=3,flag=1):
    p=W(1,1,7,baseversion,0x12345678,0xfedcba98,len(pending))
    p+=b''.join(pending)
    if baseversion>=1:p+=W(0xdeadbeef,2)+text('uai_move')+text('uai_sleep')
    if baseversion>=3:p+=bytes([flag if current is not None else 0])+(current or b'')
    return p

def main():
    cases=[]
    for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
        for v in range(4):
            for seed in range(12):
                for empty in [False,True]:
                    p=command(v,seed,empty);m=Machine(p+b'AFTER',cw);native=m.command()
                    assert native['consumed']==len(p)
                    cases.append({'kind':'command','precision':pc,'payload':p.hex(),'native':native})
        for version in range(4):
            for n in [0,1,3]:
                for present in [False,True]:
                    queued=[command(i%4,100+i) for i in range(n)]
                    current=command(3,123) if present and version>=3 else None
                    p=mind_payload(queued,current,version,255);m=Machine(p+b'AFTER',cw);native=m.mind()
                    assert native['consumed']==len(p)
                    assert native['command_allocations']==n+int(current is not None)
                    assert all(x['reference_count']==1 for x in native['pending'])
                    if native['current']:assert native['current']['reference_count']==1
                    cases.append({'kind':'mind','precision':pc,'payload':p.hex(),'native':native})
        # Null command definitions are valid in saved Load itself. They are
        # not permission for the runtime to invent a dispatch definition.
        for negative in [False,True]:
            p=mind_payload([],command(3,42,True),3,2)
            if negative:p=p[:24]+W(0xfffffffe)+p[28:]
            m=Machine(p+b'AFTER',cw);native=m.mind()
            assert native['consumed']==len(p)
            assert native['current']['definition'] is None
            cases.append({'kind':'mind','precision':pc,'payload':p.hex(),'native':native})
    for version in [4,0xffffffff,0x80000000]:
        p=W(version);native=Machine(p).command()
        cases.append({'kind':'command','precision':'Double53','payload':p.hex(),'native':native})
    doc={'schema':1,'scope':__doc__,'original_sha256':H(P.b),'v14_sha256':H(V.b),
         'producer_sha256':H(Path(__file__).read_bytes()),'cases':cases,
         'instructions':[{'va':hex(a),'bytes':b} for a,b in sorted(VISITED.items())]}
    dest=HERE/'native-commands.json';dest.write_text(json.dumps(doc,indent=1)+'\n')
    print(json.dumps({'cases':len(cases),'sha256':H(dest.read_bytes())}))

if __name__=='__main__':main()
