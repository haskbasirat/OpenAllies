"""Original4FD7F0/4FD8C0 capture,4FC500 append/copy/growth,4FC4A0 order.

Seams: allocator/delete; CString allocator identity/empty/free virtuals;
ordinary owner vf108;404F60 diagnostic;4FBA50 drain visit only (never creation).
Constructors4F4B90/4C8C20,copy constructors/assignments,CString refcounts,
property filtering503080,vector growth,local destruction and all queue control
execute natively. Successful allocation only; no exception/unwind parity.
"""
from pathlib import Path
import json,sys,hashlib,random
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-metagame'))
from mg_machine import Machine,P,V,VISITED,ORIGINAL_SHA256,V14_SHA256,NullAccess
from unicorn.x86_const import UC_X86_REG_ECX

def run(spec,cw):
    m=Machine(cw); freed=[]; strings=[]; warnings=[]; visits=[]; owner_reads=[]
    def stub(fn):
        ptr=m.alloc(16);m.u.mem_write(ptr,b'\xc3');m.seams[ptr]=fn;return ptr
    manager=m.alloc(8); vt=m.alloc(32);m.w32(manager,vt)
    def empty(machine):
        p=machine.alloc(18);strings.append(p)
        machine.w32(p,manager);machine.w32(p+12,1);machine.ret(0,p)
    m.w32(vt+12,stub(empty))
    m.w32(vt+16,stub(lambda machine:machine.ret(0,manager)))
    m.w32(vt+4,stub(lambda machine:(freed.append(machine.arg(0)),machine.ret(4))))
    m.seams[0x856370]=lambda machine:machine.ret(0,manager)
    m.seams[0x6f7810]=lambda machine:machine.ret(0,machine.alloc(max(4,machine.arg(0))))
    m.seams[0x6f7430]=lambda machine:(freed.append(machine.arg(0)),machine.ret())
    m.seams[0x404f60]=lambda machine:(warnings.append([machine.arg(5),machine.arg(6)]),machine.ret())
    m.seams[0x4fba50]=lambda machine:(visits.append(machine.arg(0)),machine.ret(4,0))
    defptr={};reverse={}
    def definition(token):
        if token is None:return 0
        if token not in defptr:
            p=m.alloc(0x600);defptr[token]=p;reverse[p]=token;m.w32(p+8,token)
        return defptr[token]
    queue=m.alloc(24)
    for off in [0,12]:m.call(0x4f5160,[spec.get('capacity',8),1],queue+off)
    rows=[];outcomes=[]
    for s in spec['requests']:
        source=m.alloc(0x200);svt=m.alloc(0x110);m.w32(source,svt)
        m.w32(source+4,definition(s['original_definition']));m.w32(source+0x14,s['source'])
        for i,w in enumerate(s['position']+s['direction']):m.w32(source+0x20+4*i,w)
        owner=s.get('owner',0)
        def get_owner(machine,source=s['source'],owner=owner):
            owner_reads.append(source);machine.ret(0,owner)
        m.w32(svt+0x108,stub(get_owner))
        props=s.get('properties')
        if props is not None:
            component=m.alloc(32);m.w32(source+0x78,component);ptrs=[]
            for p in props:
                inst=m.alloc(32);ptrs.append(inst)
                m.w32(inst+8,p['kind']);m.w32(inst+12,definition(p.get('definition')))
                m.w32(inst+0x14,p['source']);m.u.mem_write(inst+0x18,bytes([p.get('disabled',0)]))
            m.w32(component+0x10,m.array(ptrs));m.w32(component+0x14,len(ptrs))
        repl=definition(s.get('replacement'))
        if repl:m.w32(repl+0x4e0,definition(s.get('required')))
        try:
            m.call(0x4fd8c0 if s['actor'] else 0x4fd7f0,[source,repl],queue)
            outcomes.append('queued')
        except NullAccess as e:
            outcomes.append({'null':hex(e.pc)})
            break
    def record(p):
        actor=m.u.mem_read(p,1)[0]!=0; offset=4 if actor else 0x58
        base=p+offset
        data={'actor':actor,'source':m.r32(p+0x80),'definition':reverse.get(m.r32(base+4)),
          'position':[m.r32(base+8+4*i) for i in range(3)],
          'direction':[m.r32(base+0x14+4*i) for i in range(2)],
          'variant':m.r32(base+0x20),'requested':m.r32(base+0x24),
          'name_length':m.r32(m.r32(base+0x1c)-12),
          'properties':[reverse.get(m.r32(m.r32(p+0x84)+4*i)) for i in range(m.r32(p+0x88))],
          'properties_alive':m.r32(p+0x84) not in freed}
        if actor:
            data['actor_words']={hex(off):m.r32(base+off) for off in range(0x28,0x50,4)}
            data['actor_bytes']=[m.u.mem_read(base+off,1)[0] for off in range(0x50,0x53)]
        return data
    for vector in [0,12]:
        rows.append([record(m.r32(queue+vector)+0x90*i) for i in range(m.r32(queue+vector+4))])
    before_alias=None
    if spec.get('alias_append'):
        vector=queue+spec['alias_append']['vector']*12
        original=m.r32(vector)+0x90*spec['alias_append']['index'];before_alias=record(original)
        m.call(0x4fc500,[original],vector)
        rows=[]
        for v in [0,12]:rows.append([record(m.r32(queue+v)+0x90*i) for i in range(m.r32(queue+v+4))])
    # 4FC4A0 ignores false creation results, retains both queues, and does not
    # interleave actor/object order. The callback is a visit seam only.
    m.call(0x4fc4a0,[],queue)
    order=[record(p)['source'] for p in visits]
    return {'input':spec,'cw':cw,'queues':rows,'outcomes':outcomes,'warnings':warnings,
            'owner_reads':owner_reads,'visit_order':order,'alias_original':before_alias,
            'capacities':[m.r32(queue+8),m.r32(queue+20)],
            'counts_after_visit':[m.r32(queue+4),m.r32(queue+16)]}

def request(n,actor=False,**kw):
    s=dict(actor=actor,source=n,original_definition=100+n,replacement=200+n,
           position=[0x7fc01234,0x80000000,n],direction=[0x7f800000,0xff800000],owner=700)
    s.update(kw);return s
def main():
    props=[dict(kind=k,definition=300+i,source=src,disabled=i%2)
           for i,(k,src) in enumerate([(0,99),(1,0),(2,0),(2,7),(2,88),(3,88),(4,88),(1,88),(3,7),(2,0)])]
    cases=[dict(name='empty',requests=[]),
      dict(name='object_null_definition',requests=[request(1,replacement=None)]),
      dict(name='actor_owner_null_and_required_match',requests=[request(7,True,owner=0,required=107,properties=props)]),
      dict(name='actor_required_mismatch',requests=[request(8,True,required=109,properties=[])]),
      dict(name='actor_null_definition_after_warnings',requests=[request(7,True,replacement=None,properties=props)]),
      dict(name='property_null_preserved',requests=[request(7,True,properties=[dict(kind=2,source=0,definition=None)])]),
      dict(name='foreign_null_property_fault',requests=[request(7,True,properties=[dict(kind=1,source=99,definition=None)])]),
      dict(name='mixed_growth',requests=[request(i+1,i%3!=0,properties=props if i%3 else None) for i in range(22)]),
      dict(name='alias_on_growth',capacity=1,requests=[request(7,True,properties=props)],alias_append=dict(vector=1,index=0)),
      dict(name='alias_without_growth',capacity=8,requests=[request(7,True,properties=props)],alias_append=dict(vector=1,index=0))]
    rows=[run(s,cw) for cw in [0x7f,0x27f,0x37f] for s in cases]
    doc={'scope':__doc__,'original_sha256':ORIGINAL_SHA256,'v14_sha256':V14_SHA256,
        'cases':rows,'instructions':[{'va':hex(a),'bytes':v} for a,v in sorted(VISITED.items())]}
    path=HERE/'native-queue.json';path.write_text(json.dumps(doc,indent=1)+'\n')
    print(json.dumps({'cases':len(rows),'instructions':len(VISITED),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
