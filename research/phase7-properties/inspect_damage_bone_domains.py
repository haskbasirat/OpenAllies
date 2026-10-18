"""Static native DamageType/Bone registry identity, count and ID assignment."""
from pathlib import Path
import csv,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED,pe
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')};ranges=[]
for a in [0x48a8e0,0x48db60,0x47d880,0x481410,0x481570,0x481630,0x486650,0x4867b0,0x486870,0x491520,0x93d4e0,0x483a20]:
 raw=IMAGE.at(a,sizes[a]);assert raw==ACCEPTED.at(a,len(raw));ranges.append({'address':hex(a),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
 (HERE/f'damage-bone-domain-{a:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(raw,a))+'\n')
rows=[]
for name,field,ctor,cap,vt,loader,create,text in [('DamageType',0x3a4,0x481410,11,0xa0cfe4,0x481570,0x481630,0xa0e9c0),('Bone',0x580,0x486650,17,0xa0d3b4,0x4867b0,0x486870,0xa0e668)]:
 raw=(name+'\0').encode('utf-16le');assert IMAGE.at(text,len(raw))==ACCEPTED.at(text,len(raw))==raw
 data=IMAGE.at(vt,32);assert data==ACCEPTED.at(vt,32) and struct.unpack_from('<I',data,28)[0]==loader
 rows.append({'name':name,'data_field':hex(field),'constructor':hex(ctor),'initial_count':0,'initial_capacity':cap,'initial_policy':0,'vtable':hex(vt),'loader':hex(loader),'create':hex(create),'string_address':hex(text),'string_sha256':hashlib.sha256(raw).hexdigest(),'vtable_sha256':hashlib.sha256(data).hexdigest()})
doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'registries':rows,'native_ranges':ranges,'semantics':['Data constructor passes policy0 to both typed registry constructors, initializes count+28=0, and registers each field under the named UTF16 type. Capacity is separate from count.', 'Loaders branch on registry+8: nonzero policy reuses an existing IDS if found; initial policy0 always invokes the creation path, which diagnoses duplicate IDS.', 'Creation stores old registry+28 into definition+C, appends via93D4E0/483A20, and inserts IDS lookup metadata. Thus ordinals are zero-based loaded insertion order, not process pointers.', 'No reserved definitions are created by these constructors. Definition source order and any subsequent policy mutation must be supplied by the caller; this report does not assert whole data-loader execution.'], 'boundary':'Bone record identity is sufficient for a non-null data reference; this is not skeletal/NIF bone validation. Source-bound IDs can represent identities without fabricating native addresses.'}
path=HERE/'damage-bone-domain-evidence.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(hashlib.sha256(path.read_bytes()).hexdigest())
