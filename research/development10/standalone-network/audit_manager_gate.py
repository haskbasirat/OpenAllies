"""Read-only byte/decode audit of the manager-field proof, not full startup."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parents[1]/'phase8-ai'));import native_machine as nm
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
cs=Cs(CS_ARCH_X86,CS_MODE_32)
path=HERE/'native-manager-gate.json';fixture=json.loads(path.read_text());out=[]
seams={0x786c30,0x6f7810,0x7700b00,0x7700b10,0x7700b20,0x7700b30}
for row in fixture['instructions']:
 va=int(row['va'],16);raw=bytes.fromhex(row['bytes']);assert va not in seams
 assert raw==nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))
 decoded=list(cs.disasm(raw,va));assert len(decoded)==1 and decoded[0].size==len(raw)
 out.append({'va':row['va'],'bytes':row['bytes'],'instruction':decoded[0].mnemonic+' '+decoded[0].op_str})
assert any(r['va']=='0x7880ea'and r['bytes']=='889ebc000000'for r in out)
assert any(r['va']=='0x7876f3'and r['bytes']=='c681bc00000000'for r in out)
doc={'schema':1,'fixture_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'decoded_instructions':out,
 'seams_excluded':[hex(v)for v in sorted(seams)],'scope':fixture['scope']}
dest=HERE/'author-manager-gate-audit.json';dest.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps({'rows':len(fixture['rows']),'pins':len(out),'sha256':hashlib.sha256(dest.read_bytes()).hexdigest()}))
