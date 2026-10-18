"""Reproduce navigation disassembly and original/v14 code/data identity pins."""
from pathlib import Path
import hashlib,json,struct,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase3-terrain'))
from terrain_uv_oracle import Pe,ORIGINAL_SHA,ACCEPTED_SHA
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
ADDRESSES={'result_plan':0x5336b0,'solve':0x53e1f0,'adjust_endpoint':0x53d340,'near_goal':0x52e1d0,'normalize':0x409230,'clamp':0x552640,'materialize':0x53d500,'materialize_same_cell':0x53cf40,'node_value':0x52ca90,'prepend':0x52d180,'reset_path':0x52cd90,'failed_strategy':0x53c450}
def main():
 pe=Pe(ROOT.parent/'AA.exe',ORIGINAL_SHA);v14=Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',ACCEPTED_SHA)
 sizes={}
 for line in (ROOT.parent/'reconstruction/research/native-aa/functions.tsv').read_text().splitlines():
  parts=line.split('\t')
  try:sizes[int(parts[0],16)]=int(parts[3])
  except(ValueError,IndexError):pass
 md=Cs(CS_ARCH_X86,CS_MODE_32);functions={}
 for name,address in ADDRESSES.items():
  size=sizes[address];raw=pe.at(address,size);assert raw==v14.at(address,size)
  functions[name]={'start':hex(address),'size':size,'sha256':hashlib.sha256(raw).hexdigest(),'v14_identical':True}
  (OUT/(name+'-assembly.txt')).write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}' for i in md.disasm(raw,address))+'\n')
 constants={}
 for address in [0xa0a9f0,0xa32b58,0xa3aa10,0x9f3228,0x9fdc48,0x9f32dc,0x9f32e0,0x9fbdb8,0xa365c4]:
  raw=pe.at(address,4);assert raw==v14.at(address,4);constants[hex(address)]={'bits':struct.unpack('<I',raw)[0],'v14_identical':True}
 tables={}
 for name,address in [('deltas',0xb322e0),('directions',0xb32328)]:
  raw=pe.at(address,72);assert raw==v14.at(address,72);tables[name]={'address':hex(address),'words':list(struct.unpack('<18I',raw)),'sha256':hashlib.sha256(raw).hexdigest(),'v14_identical':True}
 (OUT/'native-provenance.json').write_text(json.dumps({'original_sha256':ORIGINAL_SHA,'v14_sha256':ACCEPTED_SHA,'functions':functions,'constants':constants,'tables':tables},indent=2)+'\n')
 print(json.dumps({'passed':True,'functions':len(functions),'constants':len(constants),'tables':len(tables)}))
if __name__=='__main__':main()
