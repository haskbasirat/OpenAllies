"""Read-only exact Session16D/16E writer instruction pins; static only.
Whole AllTasksComplete/projection proof remains its separately pinned oracle.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-net'))
from native_heartbeat import PE0,PE1
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
out=[]
for a,s,tag in [(0x76f890,0x310,'Session constructor'),(0x76ec70,0x260,'Session Reset'),(0x775bf0,0x185,'AllTasksComplete')]:
    for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(PE0.range(a,s),a):
        if (i.mnemonic=='mov' and ('0x16d]' in i.op_str or '0x16e]' in i.op_str)) or (i.mnemonic=='call' and i.op_str=='0x76ba90') or (i.mnemonic=='xor' and i.op_str=='ebx, ebx'):
            assert bytes(i.bytes)==PE1.range(i.address,len(i.bytes))
            out.append({'scope':tag,'address':f'{i.address:08x}','bytes':i.bytes.hex(),'instruction':i.mnemonic+' '+i.op_str})
assert len(out)==10
result={'scope':'Static exact Session constructor/Reset/AllTasksComplete instruction pins, original=v14; not whole constructor/load execution','pins':out}
path=HERE/'session-end-writers-static.json'
if '--write' in sys.argv:path.write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8',newline='\n')
else:assert json.loads(path.read_text(encoding='utf-8'))==result
print('10 static writer/control pins original=v14 PASS')
