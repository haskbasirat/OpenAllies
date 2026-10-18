"""Bounded World ctor4F8042 byte275 store. EBX0 comes from actual constructor
4F7D00 prefix's XOR EBX,EBX; no EBX-writer reaches this straight ctor path.
This is a field producer, not full constructor/SetCutsceneMode proof.
"""
from pathlib import Path
import sys,json,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parents[1]/'research/phase8-ai'))
from native_machine import Machine,original,modified,PRECISIONS
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_EIP
from unicorn import UC_HOOK_CODE
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
instructions=list(Cs(CS_ARCH_X86,CS_MODE_32).disasm(original.range(0x4f7d00,0x350),0x4f7d00))
zero=next(i for i in instructions if i.mnemonic=='xor'and i.op_str=='ebx, ebx')
assert all('ebx'not in i.op_str.split(',')[0] for i in instructions if zero.address<i.address<0x4f8042 and i.mnemonic not in('push','test','cmp'))
rows=[]
for image,pc,before in itertools.product(('original','v14'),PRECISIONS,(0,255)):
 m=Machine(pc)
 if image=='v14':
  old=(original.image_size+4095)&~4095;new=(modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(original.base+old,new-old)
  for vs,rva,raw,off in modified.sections:m.u.mem_write(modified.base+rva,modified.b[off:off+raw])
 target=original if image=='original'else modified
 assert target.range(0x4f8042,6).hex()=='889e75020000'
 p=m.alloc(0x300);m.wb(p+0x275,before);m.u.reg_write(UC_X86_REG_ESI,p);m.u.reg_write(UC_X86_REG_EBX,0)
 m.u.emu_start(0x4f8042,0x4f8048);after=m.rb(p+0x275);assert after==0
 rows.append(dict(image=image,precision=pc,before=before,after=after))
doc=dict(scope=__doc__,zero_ebx=dict(address=hex(zero.address),bytes=zero.bytes.hex()),store=dict(address='0x4f8042',bytes='889e75020000'),rows=rows)
p=HERE/'native-world-camera-flags.json'
if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
else:assert json.loads(p.read_text())==doc
print(json.dumps(dict(rows=len(rows),native_instructions_per_row=1,zero_ebx=hex(zero.address))))
