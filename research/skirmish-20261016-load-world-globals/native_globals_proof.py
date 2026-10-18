"""Read-only original/v14 native instruction pins for startup global owners."""
from pathlib import Path
import hashlib
import json
import sys
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_EFLAGS

def sha(raw):return hashlib.sha256(raw).hexdigest()

def main():
    cs=Cs(CS_ARCH_X86,CS_MODE_32)
    ranges=[('session_ctor',0x76f890,775),('phase_getter',0x76ac90,7),
            ('session_change_to_state',0x77a750,500),('point_getter',0x4e3e90,6),
            ('application_state_read',0x54ef10,42)]
    rows=[]
    for name,start,length in ranges:
        raw=original.range(start,length)
        assert raw==modified.range(start,length),(name,'original/v14 code differs')
        inst=[{'va':x.address,'bytes':x.bytes.hex(),'mnemonic':x.mnemonic,'operands':x.op_str}
              for x in cs.disasm(raw,start)]
        (HERE/(name+'.asm.txt')).write_text('\n'.join(
            f'{x["va"]:08x} {x["bytes"]} {x["mnemonic"]} {x["operands"]}' for x in inst)+'\n')
        rows.append({'name':name,'start':start,'length':length,'sha256':sha(raw),'instructions':inst})
    field=[{'function':v['name'],**x} for v in rows for x in v['instructions']
           if '0xf8' in x['operands']]
    values=[0,1,2,3,4,5,6,7,11,12,0x55aa,0x7fffffff,0x80000000,0xffffffff]
    state=0x9a735217
    for _ in range(64):
        state=(1664525*state+1013904223)&0xffffffff;values.append(state)
    getter_cases=[];ctor_cases=[];point_cases=[]
    for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
        m=Machine(cw)
        for phase in values:
            for application in [0,3,0xffffffff,phase^0xa5a55a5a]:
                m.reset(0);m.w(m.STATE+0xf8,phase);m.w(m.STATE+0xe4,application)
                output=m.call(0x76ac90)
                assert output==phase
                m.w(0xb36550,m.STATE)
                # Exact native global dereference and CMP Session+E4,3;
                # stop BEFORE either branch can read World+48 or a node.
                m.u.emu_start(0x54ef10,0x54ef1c,count=10)
                equals=bool(m.u.reg_read(UC_X86_REG_EFLAGS)&0x40)
                assert equals==(application==3)
                getter_cases.append({'precision':pc,'phase':phase,'application_state':application,
                                     'native_phase_result':output,'native_application_equals3':equals})
            m.reset(0);m.w(m.STATE+0xf8,phase)
            m.u.reg_write(UC_X86_REG_ESI,m.STATE);m.u.reg_write(UC_X86_REG_EBX,0)
            m.u.emu_start(0x76fae1,0x76fae7,count=1)
            assert m.r(m.STATE+0xf8)==0
            ctor_cases.append({'precision':pc,'before':phase,'after':m.r(m.STATE+0xf8)})
        m.reset(0);address=m.call(0x4e3e90)
        assert address==0xb3a8c4
        point_cases.append({'precision':pc,'getter_address':address})
    doc={'schema':1,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),
         'ranges':rows,'phase_field_instructions':field,'script_sha256':sha(Path(__file__).read_bytes()),
         'getter_cases':getter_cases,'constructor_store_cases':ctor_cases,'point_getter_cases':point_cases,
         'scope':'Actual native phase leaf, application-state dereference/CMP prefix, constructor single MOV and point getter execute without decision hooks at allPC. Constructor EBX=0 is a documented isolated seam established by76F8C7 and actual Windows ctor trace, not full native session initialization. No World+48/default node is supplied.'}
    out=HERE/'native-instructions.json';out.write_text(json.dumps(doc,indent=2)+'\n')
    print(json.dumps({'phase_field_instructions':field,'sha256':sha(out.read_bytes())}))

if __name__=='__main__':main()
