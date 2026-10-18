"""Pin original/v14 native startup caller flags; static evidence, not execution."""
from pathlib import Path
import sys,json
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import original,modified,sha
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
rows=[]
for label,a,b in [('root_UVars_flags0',0x419c6a,0x419c90),('user_UVars_flags3',0x419f53,0x419f87),('load_flags_restricted_mode',0x4b43b0,0x4b43e1),('existing_variable_restricted_guard',0x4b3fd4,0x4b4004)]:
 raw=original.range(a,b-a);assert raw==modified.range(a,b-a)
 rows.append({'label':label,'start':hex(a),'end':hex(b),'sha256':sha(raw),'bytes':raw.hex(),'instructions':[f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(raw,a)]})
out={'schema':1,'passed':True,'scope':'Read-only original/v14 instruction windows, not native whole startup/profile discovery. RootUVars passesflags0, UserUVarsFilename passesflags3; loader lowbit selectsSettings+24 restricted mode; existing native variable withoutflag16 skips4B3C10 assignment.','original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'windows':rows}
(HERE/'static-context.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8')
print('pinned',len(rows),'original/v14 context windows')
