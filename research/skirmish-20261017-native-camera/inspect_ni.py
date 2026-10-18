from pathlib import Path
import sys,csv,capstone
folder=Path(__file__).resolve().parent;root=folder.parents[1]
sys.path.insert(0,str(root/'research/phase4-body'));from original_body import IMAGE
sizes={int(r['address'],16):int(r['bodyBytes'])for r in csv.DictReader((root.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')}
md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_32)
addresses=[0x6fe050,0x8a2c70,0x8742d0,0x874240,0x8737b0,0x873790,0x873300,0x873380,0x873430,0x873500,0x8735e0,0x8736b0,0x872f00,0x873000,0x873100,0x873200,0x42d750,0x869c90]
out=[]
for a in addresses:
 out.append(f'\nFUNCTION {a:08X} size{sizes[a]}')
 out.extend(f'{i.address:08X} {i.bytes.hex():<20} {i.mnemonic} {i.op_str}'for i in md.disasm(IMAGE.at(a,sizes[a]),a))
(folder/'ni-disassembly.log').write_text('\n'.join(out)+'\n')
print('written',len(out),'lines')
