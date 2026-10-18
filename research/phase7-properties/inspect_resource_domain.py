"""Static native binding for the Kingdom Resource-vector initialization count."""
from pathlib import Path
import csv,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED,pe
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')}
ranges=[]
for address in [0x48a8e0,0x48db60,0x47f670,0x47d880,0x491520,0x483a40,0x479ea0,0x479f60,0x93d4e0,0x483a20,0x4e2ea0,0x45f180]:
    raw=IMAGE.at(address,sizes[address]);assert raw==ACCEPTED.at(address,len(raw))
    ranges.append({'address':hex(address),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
    (HERE/f'resource-domain-{address:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(raw,address))+'\n')
literal='Resource\0'.encode('utf-16le');assert IMAGE.at(0xa0e958,len(literal))==ACCEPTED.at(0xa0e958,len(literal))==literal
vt=IMAGE.at(0xa0c534,32);assert vt==ACCEPTED.at(0xa0c534,32)
assert struct.unpack_from('<I',vt,0x10)[0]==0x491520
assets=ROOT.parent/'reconstruction/assets/original/Data';manifest=assets/'index_aa.lst';source=assets/'Game/resources.tgi'
assert 'Game/resources*.tgi' in manifest.read_text()
matches=sorted(p.name for p in (assets/'Game').glob('resources*.tgi'));assert matches==['resources.tgi']
catalog_path=ROOT/'research/phase7-definitions/resource-definitions.json';catalog=json.loads(catalog_path.read_text())
ids=[entry['id']['String']['value'] for entry in catalog]
assert ids==['money','ammo','oil','unit_limit_provided','unit_limit_consumed','building_limit_provided','building_limit_consumed']
assert all(entry['id']['String']['source']['sha256']==sha(source) for entry in catalog)
doc={'schema':1,'scope':'Static original/v14 instruction and original asset evidence; actual loaded Resource count is supplied by the source-resolved catalog, not guessed here.',
 'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':sha(Path(__file__)),
 'registry':{'global':'0xB36530','embedded_field':'0x10','kind':'Resource','registry_count_field':'0x28','data_count_field':'0x38','constructor':'0x47F670','count_getter':'0x491520','initial_count':0,'initial_capacity':8},
 'evidence':['48A96C..48A97A constructs47F670(8,0) on Data+10. The registry constructor initializes count+28=0 and capacity+2C=8, with no reserved entry.',
 '48EB32 takes Data+10 and48EB35 pairs it with UTF16 Resource atA0E958 for type registration.',
 'Resource vtableA0C534+10 points491520, returning registry+28.',
 '483A40 dispatches vtable+1C479EA0; initial policy0 creates through479F60, which diagnoses duplicate IDS, assigns old count+28 as definition+C, then appends via93D4E0/483A20 on registry+24.',
 '4E2F3B..4E2F57 reads B36530+38 once, passes the same value to45F180 for Kingdom+1C4 and+258.'],
 'native_ranges':ranges,'data_pins':[{'address':'0xA0E958','size':len(literal),'sha256':hashlib.sha256(literal).hexdigest()},{'address':'0xA0C534','size':len(vt),'sha256':hashlib.sha256(vt).hexdigest()}],
 'original_manifest_sha256':sha(manifest),'matching_files':matches,'authored_source_sha256':sha(source),
 'resolved_catalog_evidence':{'path':catalog_path.relative_to(ROOT).as_posix(),'sha256':sha(catalog_path),'count':len(catalog),'ids':ids,'scope':'Full V14Sources ActorCatalog enumeration supplied by aa-rules owner; all authored source hashes equal the original asset.'},
 'count_contract':'Use validated loaded Resource definitions from aa-rules. Capacity8 is not the entry count. The source file includes commented examples, so raw bracket/IDS text counting is not valid.'}
path=HERE/'resource-domain-evidence.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'passed':True,'sha256':sha(path)}))
