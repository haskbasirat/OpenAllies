from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-denizen-init-create.json').read_text());assert doc['producer_sha256']==H(HERE/'native_denizen.py')
md=Cs(CS_ARCH_X86,CS_MODE_32);pins=[]
def decode(address,bits):
 a=int(address,16);raw=bytes.fromhex(bits);assert h.P.range(a,len(raw))==h.V.range(a,len(raw))==raw
 seq=list(md.disasm(raw,a));assert sum(i.size for i in seq)==len(raw)
 return [dict(address=hex(i.address),bytes=i.bytes.hex(),instruction=i.mnemonic+' '+i.op_str)for i in seq]
for address,bits in doc['pins'].items():
 seq=decode(address,bits);assert len(seq)==1;pins.extend(seq)
assert len(pins)==53 and len(doc['rows'])==195
source=[]
for address,bits in doc['source_windows'].items():source.extend(decode(address,bits))
source.extend(decode('0x4624e1','33db'))
names={}
for address,text in[(0xa077d8,'Runway'),(0x9fc730,'delay')]:
 raw=(text+'\0').encode('utf-16-le');assert h.P.range(address,len(raw))==h.V.range(address,len(raw))==raw;names[hex(address)]=dict(value=text,bytes=raw.hex())
assert h.P.range(0xa3c12c,4)==h.V.range(0xa3c12c,4)==(0x6a71d0).to_bytes(4,'little')
assert h.P.range(0x6a71d0,3)==h.V.range(0x6a71d0,3)==bytes.fromhex('b001c3')
out=dict(schema=1,paired_rows=195,executions=390,decoded_equal_executed_pins=pins,static_source_producer_windows=source,static_names=names,
 static_queue_acceptance=dict(secondary_vtable_slot='0xa3c12c',target='0x6a71d0',bytes='b001c3'),
 qualifier='Runtime fields come from actual canonical bindings/sourcePC. Source3A4 is ctor-zero (EBX clear) and optional Runway.delay load. Static source windows are not additional executed pins or full constructor/source completion evidence. Positive50AC60 uses existing concrete queues/RNG; native queue seam is explicit.')
data=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'pin-audit.json'
if '--write'in sys.argv:p.write_bytes(data)
else:assert p.read_bytes()==data
print(json.dumps(dict(pins=len(pins),static_source_instructions=len(source),sha256=hashlib.sha256(data).hexdigest())))
