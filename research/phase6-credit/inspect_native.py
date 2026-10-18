"""Original/v14 kill-credit and classification/experience instruction binding."""
from pathlib import Path
import csv,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED,pe
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
FUNCTIONS=[0x4d57c0,0x58e870,0x58d950,0x58ea30,0x58ea50,0x58ea80,0x58eab0,0x58eae0,0x58edd0,0x58ee00,0x45eca0,0x571820,0x571960,0x4ce6c0,0x7ba3c0,0x432e40]
def main():
 sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')}
 rows=[];cs=Cs(CS_ARCH_X86,CS_MODE_32)
 for a in FUNCTIONS:
  raw=IMAGE.at(a,sizes[a]);assert raw==ACCEPTED.at(a,len(raw))
  rows.append({'address':hex(a),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
  (HERE/f'{a:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in cs.disasm(raw,a))+'\n')
 (HERE/'native-ranges.json').write_text(json.dumps({'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'ranges':rows},indent=2)+'\n')
if __name__=='__main__':main()
