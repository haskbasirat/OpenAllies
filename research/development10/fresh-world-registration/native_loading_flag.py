"""Exact4F953C..4F954C flag load/mask/store, not full registration.
The preceding CString/name capture is outside this separately identified
field primitive. Both immutable executables run; surrounding words are poison.
"""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn.x86_const import UC_X86_REG_EDI,UC_X86_REG_ESP,UC_X86_REG_EIP

def main():
    original,v14=h.P,h.V
    assert original.range(0x4f953c,16)==v14.range(0x4f953c,16)
    flags=[0,1,0x7fffffff,0x80000000,0xffffffff,0x08000000,0x10000000,0x1c000000]
    seed=0x627bc10e
    for _ in range(64):
        seed^=(seed<<13)&0xffffffff;seed^=seed>>17;seed^=(seed<<5)&0xffffffff
        flags.append(seed&0xffffffff)
    rows=[]
    try:
        for label,image in [('original',original),('v14',v14)]:
            h.P=image
            for pc in h.PRECISION:
                m=h.Machine(pc)
                actor=m.alloc(0x20)
                for value in flags:
                    before=[0x12345678,0xa5a5a5a5,value,0xcafebabe,0xdeadbeef]
                    m.w32(actor,*before);m.w32(h.STACK+0x24,0x11223344)
                    m.u.reg_write(UC_X86_REG_EDI,actor);m.u.reg_write(UC_X86_REG_ESP,h.STACK)
                    m.u.emu_start(0x4f953c,0x4f954c,count=4)
                    assert m.u.reg_read(UC_X86_REG_EIP)==0x4f954c
                    after=list(m.r32(actor,5))
                    assert after==[before[0],before[1],value&0x7fffffff,before[3],before[4]]
                    rows.append({'image':label,'precision':pc,'input':value,'output':after[2],'neighbors':after[:2]+after[3:]})
    finally:
        h.P=original
    half=len(rows)//2
    assert [{k:v for k,v in row.items() if k!='image'} for row in rows[:half]]==[{k:v for k,v in row.items() if k!='image'} for row in rows[half:]]
    doc={'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
         'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
         'pairs':half,'executions':len(rows),'rows':rows,
         'executed_pins':[{'address':hex(a),'bytes':b} for a,b in sorted(h.VISITED.items())]}
    path=HERE/'native-loading-flag.json';payload=(json.dumps(doc,indent=1)+'\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes()==payload,'immutable native fixture differs'
    else:
        assert not path.exists(),'use --check for immutable replay'
        path.write_bytes(payload)
    print(json.dumps({'pairs':half,'executions':len(rows),'pins':len(h.VISITED),'sha256':hashlib.sha256(payload).hexdigest(),'result':'PASS'}))
if __name__=='__main__':main()
