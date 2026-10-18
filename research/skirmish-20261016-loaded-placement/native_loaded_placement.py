"""Untouched 4F4590 descriptor wrapper; 4F3640 and CString ownership are seams.
Both original/v14 images run independently at PC24/53/64. No SDK code.
Page-zero fault guard is inherited; each executed native instruction is pinned.
"""
from pathlib import Path
import sys, json, struct, hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-conversion'))
import native_machine as nm
from unicorn import UC_HOOK_CODE, UC_HOOK_MEM_READ
from unicorn.x86_const import *
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
H=lambda b:hashlib.sha256(b).hexdigest()
class Dependency(Exception): pass

def run(c,image,pc):
    m=nm.Machine(pc)
    old=(nm.P.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
    if new>old:m.u.mem_map(image.base+old,new-old)
    for _,rva,size,off in image.sections:m.u.mem_write(image.base+rva,image.b[off:off+size])
    obj=m.alloc(0x200);definition=m.alloc(0x600);other_definition=m.alloc(0x600)
    world=m.alloc(0x800);pos=m.alloc(12);face=m.alloc(8)
    names=[m.cstring(''),m.cstring('replacement')]
    name_ids={p:i+4 for i,p in enumerate(names)}
    def_ids={definition:2,other_definition:3}
    for p,ref in zip(names,[c['refs'],c.get('replacement_refs',1)]):m.w(p-4,ref)
    m.w(obj+4,definition);m.w(obj+8,c['flags'])
    for base,words in [(obj+0x20,c['position']),(obj+0x2c,c['facing']),
                       (pos,c['override_position']),(face,c['override_facing'])]:
        for i,w in enumerate(words):m.w(base+4*i,w)
    events=[];desc_before=None;desc_after=None;result=None;outcome='ok'
    def event(name,*args):
        events.append([name,*args])
        if c.get('failure')==name:raise Dependency(name)
    def mutate(prefix):
        if prefix+'_flags' in c:m.w(obj+8,c[prefix+'_flags'])
        if c.get(prefix+'_definition'):m.w(obj+4,other_definition)
        if prefix+'_position' in c:
            for i,w in enumerate(c[prefix+'_position']):m.w(obj+0x20+4*i,w)
        if prefix+'_facing' in c:
            for i,w in enumerate(c[prefix+'_facing']):m.w(obj+0x2c+4*i,w)
    def alloc(_):
        assert m.arg()==2
        m.ret(0,m.allocator)
    def empty(_):
        event('empty');mutate('empty');m.ret(0,names[0]-16)
    def b4(_):
        assert m.u.reg_read(UC_X86_REG_ECX)==obj
        event('b4');mutate('b4');m.ret(0,c['b4'])
    def v108(_):
        assert m.u.reg_read(UC_X86_REG_ECX)==obj
        event('v108');mutate('v108');m.ret(0,c['v108'])
    def descriptor(p):
        words=[m.r(p+4*i) for i in range(20)]
        words[1]=def_ids[words[1]];words[7]=name_ids[words[7]]
        return {'words':words,'bytes':list(m.u.mem_read(p+0x50,3))}
    def coordinator(_):
        nonlocal desc_before,desc_after
        assert m.u.reg_read(UC_X86_REG_ECX)==world
        d=m.arg(0)
        assert [m.arg(i) for i in range(1,7)]==[obj,obj,0,0,c['arg4'],c['arg5']]
        desc_before=descriptor(d)
        event('coordinator',desc_before,[1,1,0,0,c['arg4'],c['arg5']])
        if c.get('replace_name'):m.w(d+0x1c,names[1])
        if c.get('mutate_descriptor'):
            m.w(d+4,other_definition);m.w(d+0x20,0x13579bdf);m.wb(d+0x50,0x87)
        mutate('coordinator');desc_after=descriptor(d);m.ret(28,c['result'])
    def free(_):
        h=m.arg(0);event('free',name_ids[h+16]);m.ret(4)
    m.seam(0x856370,alloc)
    av=m.r(m.allocator);m.w(av+0xc,m.stub(empty));m.w(av+4,m.stub(free))
    m.w(obj,m.vtable({0xb4:b4,0x108:v108}))
    m.seam(0x4f3640,coordinator)
    def observe(u,address,size,_):
        if address==0x4f46ac:
            event('release',name_ids[m.u.reg_read(UC_X86_REG_EAX)])
    def reads(u,access,address,size,value,_):
        if address==obj+4:event('definition',def_ids[m.r(address)])
        elif address in [obj+8,obj+0x20,obj+0x24,obj+0x28,obj+0x2c,obj+0x30]:
            event('object_word',address-obj,m.r(address))
        elif pos<=address<pos+12:event('vector_word',2,address-pos,m.r(address))
        elif face<=address<face+8:event('vector_word',3,address-face,m.r(address))
    m.u.hook_add(UC_HOOK_CODE,observe);m.u.hook_add(UC_HOOK_MEM_READ,reads)
    try:
        result=m.call(0x4f4590,world,[obj,pos if c['use_position'] else 0,
                 face if c['use_facing'] else 0,c['arg4'],c['arg5']],20)
    except Dependency as error:outcome=str(error)
    assert m.outcome is None,m.outcome
    assert not m.null_accesses,m.null_accesses
    return {'outcome':outcome,'result':result,'events':events,
            'descriptor_after':desc_after,'refs':[m.r(p-4) for p in names],
            'object':{'definition':def_ids[m.r(obj+4)],'flags':m.r(obj+8),
                      'position':[m.r(obj+0x20+4*i) for i in range(3)],
                      'facing':[m.r(obj+0x2c+4*i) for i in range(2)]}}

def cases():
    base={'flags':0,'position':[0xc1233333,0x417ccccd,0x80000000],
          'facing':[0x3f800000,0],'override_position':[0x7fc12345,0xff800000,0x7f800000],
          'override_facing':[0x80000000,0x7f812345],'use_position':False,'use_facing':False,
          'b4':1,'v108':0x12345678,'refs':2,'arg4':0,'arg5':0,'result':0}
    yield dict(base,label='saved_null')
    for pos in [False,True]:
        for face in [False,True]:
            for b4 in [0,1,2,0x100,0xff000001]:
                yield dict(base,label=f'overrides_{pos}_{face}_{b4}',use_position=pos,use_facing=face,b4=b4)
    for flags in [0,1<<10,0xffffffff,0x80000400,0x80000000]:
        yield dict(base,label=f'flags_{flags}',flags=flags)
    for ref in [0,1,2,1000,0x80000000,0x7fffffff,0xffffffff]:
        yield dict(base,label=f'refs_{ref}',refs=ref)
    for code in [0,1,2,3,4,5,6,7,0xffffffff]:
        yield dict(base,label=f'result_{code}',result=code,arg4=0x87654321,arg5=0x13579bdf)
    for prefix in ['empty','b4','v108']:
        yield dict(base,label=prefix+'_mutates',**{prefix+'_flags':1<<10,prefix+'_definition':True,
                   prefix+'_position':[0xdeadbeef,0x81234567,0x7fc12345],
                   prefix+'_facing':[0x80000000,0x7f812345]})
    yield dict(base,label='b4_false_changes_flags',b4=0,b4_flags=1<<10)
    yield dict(base,label='replace_current_name',replace_name=True,mutate_descriptor=True)
    yield dict(base,label='coordinator_mutates',coordinator_flags=0x56789abc,coordinator_definition=True,
               coordinator_position=[0x11111111,0x22222222,0x33333333])
    for name in ['empty','b4','v108','coordinator','release']:
        yield dict(base,label='dependency_'+name,failure=name)

def main():
    rows=[]
    for pc in ['Single24','Double53','Extended64']:
        for c in cases():
            a=run(c,nm.P,pc);b=run(c,nm.V,pc);assert a==b,(pc,c['label'],a,b)
            rows.append({'precision':pc,'input':c,'output':a})
    cs=Cs(CS_ARCH_X86,CS_MODE_32)
    instructions=[{'address':hex(a),'bytes':b,'asm':'; '.join(i.mnemonic+' '+i.op_str for i in cs.disasm(bytes.fromhex(b),a))}
                  for a,b in sorted(nm.VISITED.items())]
    out={'schema':1,'scope':__doc__,'original_sha256':H(nm.P.b),'v14_sha256':H(nm.V.b),
         'producer_sha256':H(Path(__file__).read_bytes()),'harness_sha256':H(Path(nm.__file__).read_bytes()),
         'native_executions':len(rows)*2,'cases':rows,'instructions':instructions,
         'seams':['856370(2)/allocator+C: empty CString allocation',
                  'Object virtual+B4/+108: fixture current-class dispatch',
                  '4F3640: complete required placement coordinator, test probe only',
                  'allocator+4: CString release callback; atomic refcount runs untouched',
                  'dependency errors: stop inside a required service, no SEH/unwind claim']}
    path=HERE/'native-loaded-placement.json'
    if '--write' in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
    else:assert json.loads(path.read_text())==out,'fixture changed'
    print(json.dumps({'rows':len(rows),'executions':len(rows)*2,'instructions':len(instructions),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
