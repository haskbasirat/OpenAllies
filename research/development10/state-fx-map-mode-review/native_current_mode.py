"""Read-only current MapResource+A8 authority probe (no whole load/startup).

Runs actual4F0460 and exact LoadMap save/store/restore intervals with explicit
valid lab pointers/registers. No callbacks or native function bodies replaced.
The separate existing factory/World alias proofs establish real pointer origin.
"""
from pathlib import Path
import sys, json, hashlib
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn.x86_const import *
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
ORIGINAL=h.P
H=lambda data:hashlib.sha256(data).hexdigest()

def interval(image,pc,prior,load_mode):
    h.P=image
    m=h.Machine(pc)
    world=m.alloc(0x300);resource=m.alloc(0xb4);frame=m.alloc(0x80)+0x40
    m.w32(world+0x28,resource);m.w32(resource+0xa8,prior)
    before=m.call(0x4f0460,world)&255
    for reg,value in [(UC_X86_REG_ESI,resource),(UC_X86_REG_EDI,load_mode),(UC_X86_REG_EBP,frame),(UC_X86_REG_ESP,h.STACK)]:m.u.reg_write(reg,value)
    m.u.emu_start(0x5a2bdf,0x5a2bf7,count=20)
    saved=m.r32(frame-0x24);during_word=m.r32(resource+0xa8)
    during=m.call(0x4f0460,world)&255
    m.w32(frame-12,0)
    for reg,value in [(UC_X86_REG_ESI,resource),(UC_X86_REG_EBP,frame),(UC_X86_REG_ESP,h.STACK)]:m.u.reg_write(reg,value)
    m.u.emu_start(0x5a3110,0x5a3120,count=20)
    after_word=m.r32(resource+0xa8)
    after=m.call(0x4f0460,world)&255
    assert saved==prior and during_word==load_mode and after_word==prior
    assert [before,during,after]==[int(prior!=0),1,int(prior!=0)]
    return dict(saved_prior=saved,current_during=during_word,current_after=after_word,active_reads=[before,during,after])

def pointers(image,pc):
    h.P=image;m=h.Machine(pc)
    world=m.alloc(0x300);a=m.alloc(0xb4);b=m.alloc(0xb4)
    m.w32(a+0xa8,1);m.w32(b+0xa8,0)
    reads=[]
    for ptr in [0,a,b]:
        m.w32(world+0x28,ptr);reads.append(m.call(0x4f0460,world)&255)
    m.w32(b+0xa8,0x80000000);reads.append(m.call(0x4f0460,world)&255)
    assert reads==[0,1,0,1]
    return dict(reads=reads,sequence=['actualNULL','resourceA.current1','resourceB.current0','sameResourceB.current0x80000000'])

def main():
    rows=[];ptr=[]
    for pc in h.PRECISION:
        for prior in [0,2,0xdeadbeef]:
            for mode in [1,2,4]:
                values=[interval(image,pc,prior,mode)for image in (ORIGINAL,h.V)]
                assert values[0]==values[1]
                rows.append(dict(pc=pc,prior=prior,captured_load_argument=mode,native=values[0]))
        values=[pointers(image,pc)for image in (ORIGINAL,h.V)]
        assert values[0]==values[1];ptr.append(dict(pc=pc,native=values[0]))
    h.P=ORIGINAL
    cs=Cs(CS_ARCH_X86,CS_MODE_32)
    state=list(cs.disasm(ORIGINAL.range(0x4d4a90,0x4d4bf8-0x4d4a90),0x4d4a90))
    call_index=next(n for n,i in enumerate(state)if i.mnemonic=='call'and i.op_str=='0x4f0460')
    caller=[]
    for i in state[call_index-3:call_index+7]:
        raw=bytes(i.bytes);assert h.V.range(i.address,len(raw))==raw
        caller.append(dict(va=hex(i.address),bytes=raw.hex(),mnemonic=i.mnemonic,operands=i.op_str))
    doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),
        original_sha256=H(ORIGINAL.b),v14_sha256=H(h.V.b),
        interval_rows=rows,pointer_rows=ptr,paired_cases=len(rows)+len(ptr),native_executions=2*(len(rows)+len(ptr)),
        executed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())],
        separate_static_state_caller=caller,
        exclusions=['Not a whole MapResource load, source factory or World constructor execution.',
          'All World/resource pointers and saved frames are explicit lab fields; no observed map values.',
          'No Rust build/execution, candidate release, whole state FX or full startup acceptance.'])
    target=HERE/'native-current-mode.json'
    with target.open('x',encoding='utf-8',newline='\n')as f:json.dump(doc,f,indent=2);f.write('\n')
    print(json.dumps(dict(passed=True,paired_cases=doc['paired_cases'],native_executions=doc['native_executions'],executed_pins=len(doc['executed_pins']),sha256=H(target.read_bytes()))))

if __name__=='__main__':main()
