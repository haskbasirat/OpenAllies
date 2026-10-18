"""Pin concrete region vtables and full native routine bytes in both binaries."""
from pathlib import Path
import hashlib,json,struct,sys
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
 tables={};addresses={0x53f320,0x53ede0,0x53eb50,0x53ebb0,0x53ebd0,0x53ed30,0x53f350,0x53f410,0x53e840,0x7a9d50,0x7b8e10,0x7b8fb0,0x7b90a0,0x7b90c0,0x7bb420,0x7b9a60,0x5c2cd0}
 for label,address,length in [('solver',0xa3680c,16),('strategy',0xa367b0,64),('queue',0xa367f0,28)]:
  raw=original.range(address,length);assert raw==modified.range(address,length)
  words=list(struct.unpack('<'+'I'*(length//4),raw));addresses.update(words)
  tables[label]={'address':hex(address),'length':length,'words':[hex(x) for x in words],'sha256':hashlib.sha256(raw).hexdigest(),'v14_identical':True}
 # Catalogue omits many concrete virtual methods. Pin their entire contiguous
 # code area (including padding), and complete lengths for external thunks.
 sizes.update({0x4c9fa0:3,0x4db6b0:7,0x7c7840:6,0x93e9b0:8,0x9613b0:9})
 addresses={a for a in addresses if not 0x53e530<=a<0x53f580}
 addresses.add(0x53e530);sizes[0x53e530]=0x53f580-0x53e530
 md=Cs(CS_ARCH_X86,CS_MODE_32);functions={};assembly=[]
 for address in sorted(addresses):
  size=sizes[address];raw=original.range(address,size);assert raw==modified.range(address,size)
  functions[hex(address)]={'size':size,'sha256':hashlib.sha256(raw).hexdigest(),'v14_identical':True}
  assembly.append('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}' for i in md.disasm(raw,address)))
 (OUT/'graph-full-assembly.txt').write_text('\n\n'.join(assembly)+'\n')
 report={'original_sha256':hashlib.sha256(original.b).hexdigest(),'v14_sha256':hashlib.sha256(modified.b).hexdigest(),'functions':functions,'vtables':tables,'scope':'Concrete flag0 path; full finalizer bytes include unimplemented flag1 branch.'}
 (OUT/'graph-provenance.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({'passed':True,'functions':len(functions),'tables':len(tables)}))
if __name__=='__main__':main()
