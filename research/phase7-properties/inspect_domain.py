"""Source pins for the native AbilityType count used by Modifier502750.

Static evidence only: this does not execute the full data loader or assert that
an arbitrary ActorCatalog matches native manifest order.
"""
from pathlib import Path
import csv,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED,pe
from capstone import Cs,CS_ARCH_X86,CS_MODE_32

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')}
ranges=[]
for address in [0x48a8e0,0x48db60,0x486b30,0x47d880,0x483a40,0x486c90,0x486d50,0x491520,0x502750]:
    raw=IMAGE.at(address,sizes[address]);assert raw==ACCEPTED.at(address,len(raw))
    ranges.append({'address':hex(address),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
    (HERE/f'domain-{address:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(raw,address))+'\n')
literal='AbilityType\0'.encode('utf-16le');assert IMAGE.at(0xa0e9f8,len(literal))==ACCEPTED.at(0xa0e9f8,len(literal))==literal
vt=IMAGE.at(0xa0d3ec,32);assert vt==ACCEPTED.at(0xa0d3ec,32)
assert struct.unpack('<8I',vt)==(0x483a40,0x93c990,0x6a71d0,0x408b50,0x491520,0x487760,0x48d4a0,0x486c90)
assets=ROOT.parent/'reconstruction/assets/original/Data';manifest=assets/'index_aa.lst';source=assets/'Game/ability_types_aa.tgi'
assert 'Game/ability_types*.tgi' in manifest.read_text()
matches=sorted(p.name for p in (assets/'Game').glob('ability_types*.tgi'));assert matches==['ability_types_aa.tgi']
catalog_path=ROOT/'research/phase7-definitions/ability-type-definitions.json'
catalog=json.loads(catalog_path.read_text())
assert [entry['id']['String']['value'] for entry in catalog]==['repair','attack']
assert all(entry['id']['String']['source']['sha256']==sha(source) for entry in catalog)
doc={'schema':1,'scope':'Static original/v14 instruction and original asset evidence; no full loader execution or arbitrary catalog parity claimed.',
 'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':sha(Path(__file__)),
 'registry':{'global':'0xB36530','field':'0x3A8','kind':'AbilityType','count_field':'0x28','vector_field':'0x24','constructor':'0x486B30','count_getter':'0x491520','initial_count':0,'initial_capacity':7},
 'evidence':[
  '48B37D..48B3AC constructs486B30(7,0) and stores the resulting registry at Data+3A8.',
  '486BAE..486BB4 initializes pointer+24=0, count+28=0 and growth+2C=the supplied7. No reserved definition is constructed.',
  '48E4B4 loads Data+3A8 and48E4BA pairs it with UTF16 AbilityType atA0E9F8 for type registration.',
  'AbilityType vtable+10 invokes491520, which returns registry+28.',
  '483A40 invokes registry vtable+1C486C90; mode0 creates via486D50, which checks duplicate IDS, assigns old count as definition+C, and appends to the registry vector.',
  '50279A..5027AF reads B36530→+3A8→+28 into Modifier+94 growth hint.'
 ],'native_ranges':ranges,'data_pins':[{'address':'0xA0E9F8','size':len(literal),'sha256':hashlib.sha256(literal).hexdigest()},{'address':'0xA0D3EC','size':32,'sha256':hashlib.sha256(vt).hexdigest()}],
 'original_manifest_sha256':sha(manifest),'matching_files':matches,'authored_source_sha256':sha(source),'authored_ids':['repair','attack'],
 'resolved_catalog_evidence':{'path':catalog_path.relative_to(ROOT).as_posix(),'sha256':sha(catalog_path),'count':len(catalog),'scope':'Full V14Sources ActorCatalog enumeration supplied by the aa-rules owner; each actual authored source hash matches the original asset.'},
 'count_contract':'Use distinct loaded non-template AbilityType definitions after validating catalog/source scope; duplicates or unresolved definitions must not silently alter the count. The constructor7 is capacity, not count. Current catalog count is derived by aa-rules, not hardcoded here.'}
(HERE/'domain-evidence.json').write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'passed':True,'sha256':sha(HERE/'domain-evidence.json')}))
