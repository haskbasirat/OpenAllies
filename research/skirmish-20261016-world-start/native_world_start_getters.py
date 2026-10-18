"""Run original/v14 world-start virtual getters without callback seams.

Protected phase8-ai Machine guards all non-FS null accesses. Actual shipped
vtable entries supply call targets. Test object/component words are the only
fixture inputs; none of the executed getters or leaf calls are hooked.
"""
from pathlib import Path
import json,sys,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
from native_machine import Machine,instructions,sha
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EAX

ORIGINAL,V14=nm.original,nm.modified
TABLES=[('Actor',0xa273f0),('Kingdom',0xa299d8),('Group',0xa29490)]

def cases():
    for structure,wall in [(None,None),(None,0),(None,0x12345678),(0,None),(0,0),(0,0x12345678),(0x87654321,None),(0x87654321,0),(0x87654321,0x12345678),(0xffffffff,0xffffffff)]:
        yield {'kind':'structure','structure':structure,'wall':wall,'wall_poison':False}
    # Proves Structure's nonzero payload bypasses even an unreadable Wall.
    yield {'kind':'structure','structure':0x87654321,'wall':None,'wall_poison':True}
    for owner in [0,1,0x200abc00,0xffffffff]:
        yield {'kind':'owner','owner':owner}
    for seed in [0,0xa5a5a500,0xffffffff]:
        yield {'kind':'actor_boolean','eax_seed':seed}

def run(exe,vt,case):
    nm.original=exe;m=Machine('Double53')
    actor=m.alloc(0x200);m.u.mem_write(actor,b'\xa5'*0x200);m.w(actor,vt)
    m.w(actor+0x90,0);m.w(actor+0x94,0);m.w(actor+0xe8,0)
    if case['kind']=='structure':
        for key,off in [('structure',0x94),('wall',0x90)]:
            if case[key] is not None:
                obj=m.alloc(0x20);m.u.mem_write(obj,b'\xa5'*0x20);m.w(obj+0x10,case[key]);m.w(actor+off,obj)
        if case['wall_poison']:m.w(actor+0x90,0xdead0000)
        slot=0x100
    elif case['kind']=='owner':m.w(actor+0xe8,case['owner']);slot=0x108
    else:slot=0xb4;m.u.reg_write(UC_X86_REG_EAX,case['eax_seed'])
    wall_calls=[]
    m.observe(0x58cb30,lambda mm:wall_calls.append(mm.ecx()))
    reads=[];writes=[]
    m.u.hook_add(UC_HOOK_MEM_READ,lambda u,access,address,size,value,_:reads.append({'offset':address-actor,'size':size}),begin=actor,end=m.heap-1)
    m.u.hook_add(UC_HOOK_MEM_WRITE,lambda u,access,address,size,value,_:writes.append({'offset':address-actor,'size':size,'value':value}),begin=actor,end=m.heap-1)
    before=bytes(m.u.mem_read(actor,m.heap-actor))
    target=m.r(vt+slot);result=m.call(target,actor)
    assert not m.seams and not writes
    assert bytes(m.u.mem_read(actor,m.heap-actor))==before
    if case['kind']=='structure':
        expected=case['structure'] or case['wall'] or 0
        assert result==expected
        assert len(wall_calls)==(0 if case['structure'] or case['wall'] is None else (2 if case['wall'] else 1))
        if case['structure']:assert not any(r['offset']==0x90 for r in reads)
    elif case['kind']=='owner':assert result==case['owner']
    else:assert result&0xff==1 and result==(case['eax_seed']&0xffffff00)|1
    for a,b in m.visited.items():assert ORIGINAL.range(a,len(b))==b==V14.range(a,len(b)),hex(a)
    return {'target':hex(target),'eax':result,'al':result&0xff,'wall_leaf_calls':len(wall_calls),'fixture_reads':reads,'fixture_writes':writes},m.visited

def main():
    table_rows=[];rows=[];visited={};executions=0
    for name,vt in TABLES:
        entry={'name':name,'address':hex(vt),'slots':{}}
        for slot,expected in [(0xb4,0x6a71d0),(0x100,0x4ced60),(0x108,0x4d1e80)]:
            a=ORIGINAL.range(vt+slot,4);b=V14.range(vt+slot,4)
            assert a==b and struct.unpack('<I',a)[0]==expected
            entry['slots'][hex(slot)]={'target':hex(expected),'bytes':a.hex()}
        table_rows.append(entry)
        for case in cases():
            outputs=[]
            for exe in [ORIGINAL,V14]:
                result,seen=run(exe,vt,case);outputs.append(result);visited.update(seen);executions+=1
            assert outputs[0]==outputs[1]
            rows.append({'class':name,'case':case,'expected':outputs[0]})
    nm.original=ORIGINAL
    out={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.b),'v14_sha256':sha(V14.b),'producer_sha256':sha(Path(__file__).read_bytes()),'tables':table_rows,'cases':rows,'executions':executions,'instructions':instructions(visited),'seams':[]}
    path=HERE/'world-start-getters.json';path.write_text(json.dumps(out,indent=2)+'\n')
    print(f'{len(rows)} cases; {executions} original/v14 executions; {len(visited)} unchanged instructions; no seams')
    print(path,sha(path.read_bytes()))

if __name__=='__main__':main()
