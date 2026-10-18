from collections import deque
import native_morale as n
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
m=n.MoraleMachine()
name,t,s=next(x for x in n.cases() if x[0]=='wrong_float')
m.parse_file('t.tgi',t.encode())
p=m.parse_file('s.tgi',s.replace('[Thing]','[Thing template=Thing]').encode())
seen=deque(maxlen=20000)
m.u.hook_add(UC_HOOK_CODE, lambda u,a,s,d:seen.append((a,s)))
try:
    print(m.run_actor(m.children(p['root'])[0]))
except Exception as e:
    c=Cs(CS_ARCH_X86,CS_MODE_32)
    for a,s in seen:
        for i in c.disasm(bytes(m.u.mem_read(a,s)),a):
            print(hex(a),i.mnemonic,i.op_str)
    raise

