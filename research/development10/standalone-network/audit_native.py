"""Read-only executed-instruction and real functor selection audit."""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parents[1]/'phase8-ai'))
import native_machine as nm
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
cs=Cs(CS_ARCH_X86,CS_MODE_32)
fixture=json.loads((HERE/'native-standalone.json').read_text())
seams={0x6f7810,0x405370,0x7700b00,0x7700b10}
decoded=[]
for row in fixture['instructions']:
 va=int(row['va'],16);raw=bytes.fromhex(row['bytes']);assert va not in seams
 assert raw==nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))
 dis=list(cs.disasm(raw,va));assert len(dis)==1 and dis[0].size==len(raw)
 decoded.append({'va':row['va'],'bytes':row['bytes'],'instruction':dis[0].mnemonic+' '+dis[0].op_str})
# Actual7876F0 publishes a functor with this vtable; its +4 dispatch is785560.
# Queue/thread dispatch/locks are a separate boundary, not silently simulated.
refs=[]
for va,size in [(0x787706,6),(0xa97fec,4)]:
 raw=nm.original.range(va,size);assert raw==nm.modified.range(va,size)
 refs.append({'va':hex(va),'bytes':raw.hex()})
assert refs[0]['bytes']=='c700e87fa900'
assert struct.unpack('<I',bytes.fromhex(refs[1]['bytes']))[0]==0x785560
doc={'schema':1,'fixture_sha256':hashlib.sha256((HERE/'native-standalone.json').read_bytes()).hexdigest(),
 'images':[{'name':name,'sha256':hashlib.sha256(pe.b).hexdigest()}for name,pe in [('original',nm.original),('v14',nm.modified)]],
 'seam_entries_excluded':[hex(v)for v in sorted(seams)],'instructions':decoded,'functor_references':refs,
 'scope':'Actual standalone functor constructor and native subconstructor proof; thread queue/locks/old online service destruction are outside this fresh scope.'}
path=HERE/'author-native-audit.json';path.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps({'pins':len(decoded),'rows':len(fixture['rows']),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
