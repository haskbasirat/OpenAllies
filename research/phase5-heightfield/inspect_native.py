"""Reproduce complete sampler/fmod disassembly with original/v14 byte checks."""
from pathlib import Path
import hashlib,json,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase3-terrain'))
from terrain_uv_oracle import Pe,ORIGINAL_SHA,ACCEPTED_SHA
from capstone import Cs,CS_ARCH_X86,CS_MODE_32

def main():
 a=Pe(ROOT.parent/'AA.exe',ORIGINAL_SHA)
 b=Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',ACCEPTED_SHA)
 md=Cs(CS_ARCH_X86,CS_MODE_32);rows={}
 for name,start,end in [('nearest',0x550190,0x550205),('interpolated',0x550210,0x55042f),('fmod',0x96db7a,0x96dba2),('fmod-classifier',0x9791c0,0x97939a)]:
  raw=a.at(start,end-start);assert raw==b.at(start,end-start)
  (OUT/(name+'-assembly.txt')).write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}' for i in md.disasm(raw,start))+'\n')
  rows[name]={'start':hex(start),'end_exclusive':hex(end),'sha256':hashlib.sha256(raw).hexdigest(),'v14_identical':True}
 (OUT/'native-ranges.json').write_text(json.dumps(rows,indent=2)+'\n')
 print(json.dumps({'passed':True,'ranges':len(rows)}))
if __name__=='__main__':main()
