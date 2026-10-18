from pathlib import Path
import sys,json,hashlib
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import original,modified
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
TARGETS={'enable_wrapper':(0x542080,23),'kingdom_disable':(0x79e2e0,11),'enable_flag':(0x79e2f0,11),'actor_disable':(0x79e980,137),'actor_sai_handle':(0x4d03c0,7),'disable_kingdom_wrapper':(0x5432c0,188),'disable_actor_wrapper':(0x543380,131),'command_new':(0x4d9e20,0x85),'command_process':(0x4d9b60,0x76),'command_cleanup':(0x4ca2d0,81)}
def main():
 sizes={}
 for row in (ROOT.parent/'reconstruction/research/native-aa/functions.tsv').read_text().splitlines():
  p=row.split('\t')
  try:sizes[int(p[0],16)]=int(p[3])
  except(ValueError,IndexError):pass
 md=Cs(CS_ARCH_X86,CS_MODE_32);pins={}
 for name,(a,n)in TARGETS.items():
  n=max(n,sizes.get(a,0));raw=original.range(a,n);assert raw==modified.range(a,n)
  (OUT/(name+'-asm.txt')).write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}'for i in md.disasm(raw,a))+'\n');pins[name]={'address':hex(a),'length':n,'sha256':hashlib.sha256(raw).hexdigest()}
 (OUT/'provenance.json').write_text(json.dumps({'original_sha256':hashlib.sha256(original.b).hexdigest(),'v14_sha256':hashlib.sha256(modified.b).hexdigest(),'functions':pins},indent=2)+'\n')
if __name__=='__main__':main()
