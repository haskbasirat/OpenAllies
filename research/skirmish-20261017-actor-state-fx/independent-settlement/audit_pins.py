"""Independent PE/decode audit; distinguish observer stop bytes from execution."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'phase8-ai'));import native_machine as nm
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
cs=Cs(CS_ARCH_X86,CS_MODE_32);base=HERE.parent
f=json.loads((base/'findings.json').read_text());allpins={};details={}
observed={'native-actor-source-boundaries.json':{0x465007,0x464fe7},'native-vfx-active-source.json':{0x494fa0}}
seams={0x6f7810,0x50c800,0x4bb310,0x5b5380,0x5b4ae0,0x4976b0,0x42b8e0,0x410c40,
 0x5b54d0,0x4012c0,0x405b80,0x5b4ed0,0x5b4c30,0x497220,0x5b4220,0x42be10,
 0x85c680,0x4bdb70,0x42c070,0x4c5bf0,0x404f60,0x5043f0,0x96cad6}
for name in f['fixtures']:
 data=json.loads((base/name).read_text());rows=[]
 for pin in data['instructions']:
  va=int(pin['va'],16);raw=bytes.fromhex(pin['bytes']);assert va not in seams
  assert nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
  instructions=list(cs.disasm(raw,va));assert len(instructions)==1 and instructions[0].size==len(raw)
  status='observer cutoff entry, not executed'if va in observed.get(name,set())else'executed instruction'
  rows.append(dict(va=pin['va'],bytes=pin['bytes'],classification=status,decode=instructions[0].mnemonic+' '+instructions[0].op_str))
  allpins.setdefault(va,set()).add(status)
 for window in data.get('static_slices',[]):
  va=int(window['va'],16);raw=bytes.fromhex(window['bytes']);assert raw==nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))
 details[name]={'pins':rows,'recorded':len(rows),'observed_cutoff_entries':sum('observer'in r['classification']for r in rows)}
out={'schema':1,'scope':__doc__,'fixtures':details,'unique_recorded_instruction_addresses':len(allpins),
 'unique_executed_addresses':sum('executed instruction'in labels for labels in allpins.values()),
 'observer_only_addresses':[hex(va)for va,labels in sorted(allpins.items())if'executed instruction'not in labels],
 'declared_transport_entries_excluded':[hex(v)for v in sorted(seams)]}
p=HERE/'pin-audit.json';p.write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({'recorded_unique':len(allpins),'observed_only':out['observer_only_addresses'],'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
