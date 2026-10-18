"""Pin full original preparation/startup bodies and source service boundaries."""
from pathlib import Path
import hashlib,json,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import original,modified
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
def main():
 sizes={}
 for line in (ROOT.parent/'reconstruction/research/native-aa/functions.tsv').read_text().splitlines():
  parts=line.split('\t')
  try:sizes[int(parts[0],16)]=int(parts[3])
  except(ValueError,IndexError):pass
 targets={'prepare':0x7abc30,'bottleneck':0x7bc8b0,'bounded_traversal':0x7bb1c0,'cache_init':0x53f0a0,'startup':0x7a0d40,'kingdom_first':0x4f0680,'kingdom_next':0x4f06b0,'registry_next':0x4efbe0}
 md=Cs(CS_ARCH_X86,CS_MODE_32);pins={}
 for name,address in targets.items():
  size=sizes[address];raw=original.range(address,size);assert raw==modified.range(address,size)
  (OUT/(name+'-assembly.txt')).write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}'for i in md.disasm(raw,address))+'\n')
  pins[name]={'address':hex(address),'length':size,'sha256':hashlib.sha256(raw).hexdigest(),'v14_identical':True}
 (OUT/'native-provenance.json').write_text(json.dumps({'original_sha256':hashlib.sha256(original.b).hexdigest(),'v14_sha256':hashlib.sha256(modified.b).hexdigest(),'functions':pins},indent=2)+'\n');print({'passed':True,'functions':len(pins)})
if __name__=='__main__':main()
