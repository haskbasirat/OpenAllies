"""Native460310/460000 completed recruitment backreferences.

Unmodified473AD0/475230 queue, producers,4C2310 and45EEF0 execute on typed registry inputs.
Explicit seams are target-registry Find4BDBA0, allocation6F7810, diagnostic
throw410C40, CString buffer retain4012C0, deferred cleanup9454B0/6F7430,
and the warning-only log. Resource count0 omits warning iteration;
neither warning branch writes recruitment lists. Deferred sources are walked
in reverse creation order after the complete final lookup map exists.
"""
from pathlib import Path
import sys,json,copy,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import Machine,MODES,ORIGINAL,ACCEPTED,pin,sha
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ECX

class NativeThrow(Exception):pass
class Backrefs(Machine):
    def __init__(self,cw,c,image):
        super().__init__(cw);self.c=c;self.order=[];self.cursor=0x2001000
        if image=='v14':
            old=(ORIGINAL.size+4095)&~4095;new=(ACCEPTED.size+4095)&~4095
            if new>old:self.uc.mem_map(ORIGINAL.base+old,new-old)
            self.uc.mem_write(ACCEPTED.base,ACCEPTED.data[:ACCEPTED.header_size])
            for rva,size,offset,_ in ACCEPTED.sections:
                if size:self.uc.mem_write(ACCEPTED.base+rva,ACCEPTED.data[offset:offset+size])
        self.gd=self.fixture(0x1000);self.put(0xb36530,self.gd);self.put(self.gd+0x38,0)
        self.actors={a['id'].lower():self.fixture(0x600) for a in c['actors']}
        self.names={self.actors[a['id'].lower()]:a['id'] for a in c['actors']}
        for a in c['actors']:
            p=self.actors[a['id'].lower()];self.put(p+0x174,a['flags']);self.put(p+8,self.string(a['id']))
            self.put(p+0x558,0xdead0000 if a['research_items'] else 0)
            self.put(p+0x1d4,self.list(a['properties']))
            if a['layout'] is not None:
                layout=a['layout'];data=self.fixture(0x60);obj=self.fixture(0x30)
                entries=self.fixture(max(4,len(layout['properties'])*16));groups=self.fixture(max(4,len(layout['groups'])*4))
                params=self.fixture(max(4,len(layout['properties'])*4))
                for i,prop in enumerate(layout['properties']):
                    self.put(entries+i*16+4,prop);self.put(entries+i*16+8,layout['numbers'][i])
                for i,group in enumerate(layout['groups']):self.put(groups+i*4,group)
                self.put(data+0x1c,entries);self.put(data+0x20,len(layout['properties']))
                self.put(data+0x28,groups);self.put(data+0x2c,len(layout['groups']))
                self.put(obj+8,data);self.put(obj+0xc,params);self.put(obj+0x10,len(layout['properties']))
                self.put(p+0x2e8,data);self.put(p+0x2f0,obj)
        self.seam(0x4bdba0,self.find);self.seam(0x6f7810,self.new)
        self.seam(0x410c40,self.throw);self.seam(0x404ed0,lambda m,e:(None,0))
        self.seam(0x4012c0,lambda m,e:(self.arg(0),0))
        self.seam(0x9454b0,lambda m,e:(None,0));self.seam(0x6f7430,lambda m,e:(None,0))
        self.uc.hook_add(UC_HOOK_CODE,self.code)
    def fixture(self,n):
        p=self.cursor;self.cursor+=(n+15)&~15;assert self.cursor<0x2400000
        self.uc.mem_write(p,b'\0'*n);return p
    def string(self,s):
        data=s.encode('utf-16le')+b'\0\0';p=self.fixture(len(data));self.uc.mem_write(p,data);return p
    def read_string(self,p):
        data=bytearray()
        while bytes(self.uc.mem_read(p,2))!=b'\0\0':data.extend(self.uc.mem_read(p,2));p+=2
        return data.decode('utf-16le')
    def list(self,values):
        cur=0
        for value in reversed(values):
            p=self.fixture(8);self.put(p,value);self.put(p+4,cur);cur=p
        return cur
    def new(self,m,esp):return self.alloc(self.arg(0)),0
    def throw(self,m,esp):raise NativeThrow(self.read_string(self.arg(0)))
    def find(self,m,esp):
        assert self.arg(1)==0x2b
        key=self.read_string(self.arg(0)).lower();canonical=self.c['lookups'].get(key)
        return self.actors.get(canonical.lower(),0) if canonical else 0,8
    def values(self,p):
        out=[];seen=set()
        while p:
            assert p not in seen;seen.add(p)
            out.append(self.names[self.read(p)]);p=self.read(p+4)
        return out
    def code(self,u,address,size,user):
        if address in (0x460310,0x460000):
            owner=self.names[self.uc.reg_read(UC_X86_REG_ECX)]
            target=self.read_string(self.read(self.arg(0)))
            self.order.append([address,owner,self.c['lookups'].get(target.lower())])
    def run(self):
        refs=[]
        for a in self.c['actors']:
            if a['layout'] is not None:
                refs.extend((0x460000,a['id'],name,slot) for slot,name in a['layout']['elements'])
            refs.extend((0x460310,a['id'],name,None) for name in a['recruit_targets'])
        outcome='ok';error=None
        try:
            #Actual prepend constructor runs while the source actors load.
            #The final reference lookup map is read only when475230 dispatches.
            for address,owner,target,slot in refs:
                cell=self.fixture(4);self.put(cell,self.string(target))
                ref=self.fixture(0x14)
                self.call(0x473ad0,[cell],ref)
                self.put(ref,0xa03470 if slot is None else 0xa0348c)
                self.put(ref+0xc,self.actors[owner.lower()])
                if slot is not None:self.put(ref+0x10,slot)
            self.call(0x475230,[],self.gd+0x824)
            assert self.read(self.gd+0x824)==0
        except NativeThrow as e:outcome='native_throw';error=str(e)
        except AssertionError:
            if self.null_accesses:outcome='native_invalid_state';error=self.null_accesses
            else:raise
        lists={key:self.values(self.read(p+0x17c)) for key,p in self.actors.items()}
        return {'outcome':outcome,'error':error,'lists':lists,'order':self.order}

def actor(name,flags=0,refs=None,layout=None):
    return {'id':name,'flags':flags,'properties':[101],'research_items':False,'layout':layout,'recruit_targets':refs or [],'sources':[]}
def layout(elements,numbers=None):
    numbers=numbers or [1]*len(elements)
    return {'properties':[101]*len(elements),'numbers':numbers,'groups':[i for i,n in enumerate(numbers) for _ in range(max(0,n))],'elements':[[i,e] for i,e in enumerate(elements) if e is not None]}
def case(name,actors,aliases=None):
    lookups={a['id'].lower():a['id'] for a in actors};lookups.update(aliases or {})
    return {'name':name,'actors':actors,'lookups':lookups}
def cases():
    yield case('empty',[actor('plain')])
    yield case('direct_duplicate',[actor('r',1<<13,['e','e','e']),actor('e')])
    for order in range(4):
        actors=[actor('r',1<<13,['o','o']),actor('e',1<<8),actor('o',1<<7,layout=layout(['e'],[3]))]
        if order==1:actors.reverse()
        if order==2:actors=[actors[2],actors[0],actors[1]]
        if order==3:actors=[actors[1],actors[2],actors[0]]
        yield case(f'org_order_{order}',actors)
    yield case('element_self_backref_suppresses_460310_propagation',[actor('r',1<<13,['o']),actor('e',(1<<8)|(1<<13),['e']),actor('o',1<<7,layout=layout(['e'],[3]))])
    yield case('aliases_final',[actor('r',1<<13,['late_alias']),actor('a'),actor('b')],{'late_alias':'b'})
    for i in range(32):
        rng=random.Random(0x460310+i)
        els=[actor(f'e{j}',1<<8) for j in range(3)]
        orgs=[actor(f'o{j}',1<<7,layout=layout([f'e{rng.randrange(3)}' for _ in range(3)],[rng.randrange(1,4) for _ in range(3)])) for j in range(2)]
        rs=[actor(f'r{j}',1<<13,[rng.choice(['e0','e1','e2','o0','o1']) for _ in range(rng.randrange(5))]) for j in range(3)]
        actors=els+orgs+rs;rng.shuffle(actors);yield case(f'varied_{i}',actors)
    yield case('unknown',[actor('r',1<<13,['missing'])])
    c=case('not_element',[actor('r',1<<13,['o']),actor('o',1<<7,layout=layout(['e'])),actor('e')]);yield c
    c=case('missing_property',[actor('o',1<<7,layout=layout(['e'])),actor('e',1<<8)]);c['actors'][1]['properties']=[];yield c
    c=case('mobile_slot',[actor('o',1<<7,layout=layout(['e'],[2])),actor('e',(1<<8)|(1<<19))]);yield c
    c=case('hq_research',[actor('r',1<<13,['o']),actor('o',(1<<7)|(1<<30),layout=layout(['e'])),actor('e',(1<<8)|(1<<21))]);c['actors'][2]['research_items']=True;yield c

def main():
    real='--real' in sys.argv
    inputs=[json.loads((HERE/'actual-source-inputs.json').read_text())['input']] if real else list(cases())
    rows=[]
    for image in ('original','v14'):
        for pc,cw in MODES:
            for c in inputs:
                machine=Backrefs(cw,c,image);rows.append({'image':image,'precision':pc,'input':c,'output':machine.run()})
    windows={'recruit':(0x460310,0x4604b8),'element':(0x460000,0x46019f),'parameter':(0x4c2310,0x4c2364),'property':(0x45eef0,0x45ef15),
             'deferred_constructor':(0x473ad0,0x473b0c),'deferred_dispatch':(0x475230,0x475275),'element_callback':(0x461750,0x461765),'recruit_callback':(0x461810,0x46181d)}
    doc={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),'function_windows':pin(windows),'generator_sha256':sha(Path(__file__).read_bytes()),'cases':rows}
    path=HERE/('native-real-recruit-backrefs.json' if real else 'native-recruit-backrefs.json');path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
