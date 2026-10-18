from pathlib import Path
import csv,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED,pe
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
FUNCTIONS=[0x571690,0x5716b0,0x571a60,0x571c50,0x45db60,0x529c30,0x4cebb0,0x4ceb70,0x58e890,0x57dcf0,0x4cef10,0x4cef70]
def main():
 sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')};rows=[]
 for a in FUNCTIONS:
  raw=IMAGE.at(a,sizes[a]);assert raw==ACCEPTED.at(a,len(raw));rows.append({'address':hex(a),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
  (HERE/f'{a:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(raw,a))+'\n')
 (HERE/'native-ranges.json').write_text(json.dumps({'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'ranges':rows},indent=2)+'\n')
if __name__=='__main__':main()
