"""Execute original/v14 Denizen569C70 and4CED40/4CED60/58CB30/517FA0.

No gameplay seam. Synthetic typed component storage supplies native pointers,
scalar bytes/words and action/state masks. Each action-low-word write is traced;
unchanged high-word writes are represented by the resulting full64-bit mask.
"""
from pathlib import Path
import sys,json,copy,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import Machine,MODES,ORIGINAL,ACCEPTED,pin,sha
from unicorn import UC_HOOK_MEM_WRITE,UC_HOOK_CODE
A,D,S,W,V,P=0x2010000,0x2011000,0x2012000,0x2013000,0x2014000,0x2015000

class Denizen(Machine):
    def __init__(self,cw,c,image):
        super().__init__(cw);self.c=c;self.trace=[];self.collect=False
        if image=='v14':
            old=(ORIGINAL.size+4095)&~4095;new=(ACCEPTED.size+4095)&~4095
            if new>old:self.uc.mem_map(ORIGINAL.base+old,new-old)
            self.uc.mem_write(ACCEPTED.base,ACCEPTED.data[:ACCEPTED.header_size])
            for rva,size,offset,_ in ACCEPTED.sections:
                if size:self.uc.mem_write(ACCEPTED.base+rva,ACCEPTED.data[offset:offset+size])
        self.put(A,V);self.put(V+0x100,0x4ced60)
        self.put(A+0xf0,c['actions']);self.put(A+0xf4,c['actions']>>32)
        self.put(A+0x100,c['state']);self.put(A+0x104,c['state']>>32)
        self.put(A+0x98,P if c['settlement'] else 0)
        self.put(A+0x94,S if c['structure'] else 0);self.put(S+0x10,P if c['parent'] else 0)
        self.put(A+0x90,W if c['wall'] else 0);self.put(W+0x10,P if c['wall_target'] else 0)
        self.put(D+4,A)
        for offset,key in [(0x20,'byte20'),(0x21,'byte21')]:self.put8(D+offset,c[key])
        self.put(D+0x38,c['flags38']);self.put(D+0x3c,c['flags3c'])
        self.uc.hook_add(UC_HOOK_MEM_WRITE,self.write)
        self.uc.hook_add(UC_HOOK_CODE,self.code)
        self.collect=True
    def mask(self):return self.read(A+0xf0)|(self.read(A+0xf4)<<32)
    def write(self,u,access,address,size,value,user):
        if self.collect and address==A+0xf0:
            self.trace.append(['write',value|(self.read(A+0xf4)<<32)])
    def code(self,u,address,size,user):
        if address in (0x4ced40,0x4ced60,0x58cb30):self.trace.append(['call',hex(address)])
    def run(self):
        self.call(0x569c70,[],D)
        return {'actions':self.mask(),'trace':self.trace}

def cases():
    rng=random.Random(0x569c70)
    base={'name':'base','state':0,'actions':0xffffffffffffffff,
          'settlement':False,'structure':False,'parent':False,'wall':False,'wall_target':False,
          'byte20':0,'byte21':0,'flags38':0,'flags3c':0}
    yield base
    for flags in range(256):
        c=copy.deepcopy(base);c.update(name=f'flags_{flags:02x}',flags38=flags,flags3c=rng.getrandbits(32),
            actions=rng.getrandbits(64),settlement=True,byte20=[0,1,7,255][flags%4],byte21=[255,0,1,128][flags%4])
        yield c
    for flags3c in (0,0x10,0x40,0x50,0xffffffaf,0xffffffff):
        for b20,b21 in ((0,0),(1,1),(255,7),(0,128)):
            c=copy.deepcopy(base);c.update(name=f'truth_{flags3c:x}_{b20}_{b21}',flags38=0xd8,
                flags3c=flags3c,byte20=b20,byte21=b21,settlement=True);yield c
    for settlement,structure,parent,wall,target in (
        (False,False,False,False,False),(False,True,False,False,False),
        (False,True,True,True,True),(False,False,False,True,False),
        (False,False,False,True,True),(False,True,False,True,True),
        (True,True,True,True,True)):
        for razing in (False,True):
            c=copy.deepcopy(base);c.update(name=f'parents_{settlement}_{structure}_{parent}_{wall}_{target}_{razing}',
                settlement=settlement,structure=structure,parent=parent,wall=wall,wall_target=target,
                state=(1<<30) if razing else 0,flags38=0xd8,flags3c=0x50,byte20=255,byte21=128)
            yield c
    for i in range(64):
        c=copy.deepcopy(base);c.update(name=f'varied_{i}',state=rng.getrandbits(64),actions=rng.getrandbits(64),
            settlement=bool(i&1),structure=bool(i&2),parent=bool(i&4),wall=bool(i&8),wall_target=bool(i&16),
            flags38=rng.getrandbits(32),flags3c=rng.getrandbits(32),byte20=rng.randrange(256),byte21=rng.randrange(256));yield c

def main():
    rows=[]
    saved='--saved' in sys.argv
    inputs=json.loads((HERE/'actual-saved-inputs.json').read_text()) if saved else list(cases())
    for image in ('original','v14'):
        for precision,cw in MODES:
            for c in inputs:
                m=Denizen(cw,c,image)
                rows.append({'image':image,'precision':precision,'input':c,'output':m.run()})
    windows={'contributor':(0x569c70,0x569e3f),'settlement_or_parent':(0x4ced40,0x4ced55),
             'parent':(0x4ced60,0x4ced97),'wall_reference':(0x58cb30,0x58cb34),'action_write':(0x517fa0,0x518004)}
    doc={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),
         'function_windows':pin(windows),'cases':rows,'generator_sha256':sha(Path(__file__).read_bytes())}
    path=HERE/('native-saved-denizen-actions.json' if saved else 'native-denizen-actions.json');path.write_text(json.dumps(doc,indent=2)+'\n')
    print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
