"""Run original Builder4604C0 with real473AD0/475230 ordering and461820 thunk.

Explicit seams: final typed FindActor4BDBA0 from supplied canonical lookup,
successful alloc6F7810, throw410C40 (stops, never returns), CString retain4012C0,
deferred cleanup9454B0/free6F7430. This Builder callback subsequence assumes
unrelated deferred callbacks complete without Builder-visible mutation/failure.
No historical source-manager or arbitrary allocator-reentry claim is made.
"""
from pathlib import Path
import json
import random
import sys

HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import Machine,MODES,ORIGINAL,ACCEPTED,pin,sha
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ECX

class NativeThrow(Exception): pass

class Builder(Machine):
    def __init__(self,cw,source,image):
        super().__init__(cw);self.source=source;self.cursor=0x2001000
        self.order=[];self.failure=None;self.allocated=[];self.authored={};self.active=None
        if image=='v14':
            old=(ORIGINAL.size+4095)&~4095;new=(ACCEPTED.size+4095)&~4095
            if new>old:self.uc.mem_map(ORIGINAL.base+old,new-old)
            for rva,size,offset,_ in ACCEPTED.sections:
                if size:self.uc.mem_write(ACCEPTED.base+rva,ACCEPTED.data[offset:offset+size])
        self.manager=self.fixture(0x1000);self.put(0xb36530,self.manager)
        self.actors={a['id'].lower():self.fixture(0x600) for a in source['actors']}
        self.names={self.actors[a['id'].lower()]:a['id'] for a in source['actors']}
        for actor in source['actors']:
            address=self.actors[actor['id'].lower()]
            self.put(address+8,self.string(actor['id']))
            self.put(address+0x174,int(actor['body_present'])|(int(actor['builder_present'])<<18))
        self.seam(0x4bdba0,self.find);self.seam(0x6f7810,self.new)
        self.seam(0x410c40,self.throw);self.seam(0x4012c0,lambda m,e:(self.arg(0),0))
        self.seam(0x9454b0,lambda m,e:(None,0));self.seam(0x6f7430,lambda m,e:(None,0))
        self.uc.hook_add(UC_HOOK_CODE,self.code)
    def fixture(self,n):
        address=self.cursor;self.cursor+=(n+15)&~15;assert self.cursor<0x2400000
        self.uc.mem_write(address,b'\0'*n);return address
    def string(self,value):
        data=value.encode('utf-16le')+b'\0\0';address=self.fixture(len(data))
        self.uc.mem_write(address,data);self.authored[address]=value;return address
    def text(self,address):
        raw=bytearray()
        while bytes(self.uc.mem_read(address,2))!=b'\0\0':raw.extend(self.uc.mem_read(address,2));address+=2
        return raw.decode('utf-16le')
    def find(self,m,esp):
        assert self.arg(1)==0x2b
        key=self.text(self.arg(0)).lower();canonical=self.source['lookups'].get(key)
        return self.actors.get(canonical.lower(),0) if canonical else 0,8
    def new(self,m,esp):
        assert self.arg(0)==8
        address=self.alloc(8);self.allocated.append(address);return address,0
    def throw(self,m,esp):
        message=self.text(self.arg(0))
        kind='UnknownActor' if message.startswith('Unknown actor') else 'MissingBody'
        assert kind=='UnknownActor' or 'does not have a body' in message,message
        self.failure={'owner':self.active[0],'authored':self.active[1],'kind':kind}
        raise NativeThrow()
    def code(self,uc,address,size,user):
        if address==0x4604c0:
            owner=self.names[uc.reg_read(UC_X86_REG_ECX)]
            authored=self.authored[self.read(self.arg(0))]
            self.active=(owner,authored);self.order.append([address,owner,authored])
    def run(self):
        try:
            for actor in self.source['actors']:
                for authored in actor['targets']:
                    cell=self.fixture(4);self.put(cell,self.string(authored))
                    reference=self.fixture(0x10)
                    self.call(0x473ad0,[cell],reference)
                    self.put(reference,0xa03474)
                    self.put(reference+0xc,self.actors[actor['id'].lower()])
            self.call(0x475230,[],self.manager+0x824)
            assert self.read(self.manager+0x824)==0
        except NativeThrow:pass
        index={address:i for i,address in enumerate(self.allocated)}
        nodes=[{'target':self.names[self.read(address)],
                'next':index[self.read(address+4)] if self.read(address+4) else None}
               for address in self.allocated]
        lists={};heads={};tails={}
        for key,address in self.actors.items():
            head=self.read(address+0x504);tail=self.read(address+0x508);values=[];seen=set();node=head
            while node:
                assert node not in seen;seen.add(node);values.append(self.names[self.read(node)]);node=self.read(node+4)
            lists[key]=values;heads[key]=index[head] if head else None;tails[key]=index[tail] if tail else None
        return {'lists':lists,'heads':heads,'tails':tails,'nodes':nodes,'order':self.order,'failure':self.failure}

def actor(ids,body=False,targets=None,builder=None):
    return {'id':ids,'body_present':body,'builder_present':bool(targets) if builder is None else builder,
            'targets':targets or [],'sources':[]}
def case(name,actors,aliases=None):
    lookups={a['id'].lower():a['id'] for a in actors};lookups.update(aliases or {})
    return {'name':name,'actors':actors,'lookups':lookups}
def cases():
    yield case('empty',[actor('plain')])
    yield case('empty_builder',[actor('builder',builder=True)])
    yield case('order_duplicates',[actor('owner',targets=['A','B','A']),actor('A',True),actor('B',True)])
    yield case('two_owners',[actor('first',targets=['A','B']),actor('A',True),actor('second',targets=['B','A']),actor('B',True)])
    yield case('aliases_shadow_owner',[actor('old',targets=['alias']),actor('A',True),actor('B',True)],{'old':'B','alias':'A'})
    yield case('nul_case_lookup',[actor('owner',targets=['A\0unread']),actor('A',True)])
    yield case('unknown_after_prefix',[actor('owner',targets=['missing','A']),actor('A',True)])
    yield case('missing_body_after_prefix',[actor('owner',targets=['nobody','A']),actor('A',True),actor('nobody')])
    yield case('empty_unknown',[actor('owner',targets=[''])])
    yield case('other_owner_preserved_before_failure',[actor('first',targets=['A']),actor('bad',targets=['missing','A']),actor('last',targets=['A','A']),actor('A',True)])
    for n in range(32):
        rng=random.Random(0x4604c0+n)
        actors=[actor('A',True),actor('B',True),actor('nobody')]
        actors += [actor(f'owner{i}',targets=[rng.choice(['A','B','nobody','missing']) for _ in range(rng.randrange(5))],builder=True) for i in range(3)]
        rng.shuffle(actors);yield case(f'varied_{n}',actors)

def typed_actor_probes():
    rows=[]
    for image in ['original','v14']:
        for pc,cw in MODES:
            for category,vtable in [('ActorData',0xa05540),('RandomActorData',0xa09b30),('ObjectData',0xa13318)]:
                machine=Machine(cw)
                if image=='v14':
                    old=(ORIGINAL.size+4095)&~4095;new=(ACCEPTED.size+4095)&~4095
                    if new>old:machine.uc.mem_map(ORIGINAL.base+old,new-old)
                    for rva,size,offset,_ in ACCEPTED.sections:
                        if size:machine.uc.mem_write(ACCEPTED.base+rva,ACCEPTED.data[offset:offset+size])
                actor_address=machine.alloc(0x600);descriptor=machine.alloc(0x20)
                machine.put(actor_address,vtable);machine.put(actor_address+0x18,descriptor)
                machine.put(descriptor+0xc,0x99)
                machine.seam(0x427bd0,lambda m,e:(actor_address,4))
                result,_=machine.call(0x4bdba0,[0x12345678,0x2b],0)
                accepted=result==actor_address
                assert accepted==(category!='ObjectData')
                rows.append({'image':image,'precision':pc,'category':category,
                             'native_vtable':vtable,'category_word':0x99,'requested_kind':0x2b,'accepted':accepted})
    return rows

def main():
    real='--real' in sys.argv
    source=HERE/'actual-source-inputs.json'
    inputs=[json.loads(source.read_text())['input']] if real else list(cases())
    rows=[]
    for image in ['original','v14']:
        for pc,cw in MODES:
            for item in inputs:
                machine=Builder(cw,item,image)
                rows.append({'image':image,'precision':pc,'input':item,'output':machine.run()})
    doc={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),
         'producer_sha256':sha(Path(__file__).read_bytes()),'harness_sha256':sha((ROOT/'research/phase8-economy/harness.py').read_bytes()),
         'function_windows':pin({'builder':(0x4604c0,0x46057e),'deferred_constructor':(0x473ad0,0x473b0c),
                                 'deferred_dispatch':(0x475230,0x475275),'builder_thunk':(0x461820,0x46182d),
                                 'typed_find_actor':(0x4bdba0,0x4bdbd9),
                                 'ActorData_vtable':(0xa05540,0xa0557c),'RandomActorData_vtable':(0xa09b30,0xa09b6c),
                                 'ObjectData_predicate_slot':(0xa13350,0xa13354),
                                 'actor_predicate_true':(0x4f0500,0x4f0506),'object_predicate_false':(0x4c9fa0,0x4c9fa3),
                                 'RandomActor_constructor_parent':(0x469980,0x4699db),'RandomActor_load_parent':(0x468d20,0x468d4d)}),
         'metadata_only_windows':pin({'Builder_LoadData_ref_creation':(0x467f08,0x468007),
                                      'constructor_Builder_head_tail_zero_stores':(0x462cf9,0x462d05)}),
         'native_initial_head_tail':'zero supplied from constructor4624B0 observed504/508 initialization; constructor body not reexecuted',
         'lookup_scope':'current final typed FindActor is an explicit seam; canonical source/Translate tested separately',
         'typed_FindActor_scope':'actual4BDBA0 and actual native vtable+38 predicates; only427BD0 Find transport seam returns supplied object',
         'typed_actor_probes':typed_actor_probes(),
         'page_zero_guard':'only legitimate FS:[0] accesses permitted','cases':rows}
    if real:doc['actual_source_input_sha256']=sha(source.read_bytes())
    path=HERE/('native-real-builder-data.json' if real else 'native-builder-data.json')
    if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
    else:assert json.loads(path.read_text())==doc
    print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))

if __name__=='__main__':main()
