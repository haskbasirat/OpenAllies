"""Static source proof of Data+464 Property registry and reserved mobile0.
No complete data-loader execution or wildcard file ordering is claimed.
"""
from pathlib import Path
import csv,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'));from original_body import IMAGE,ACCEPTED,pe
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')}
ranges=[]
for address in [0x48a8e0,0x47efd0,0x47d880,0x48db60,0x47e9b0,0x47d8d0,0x4b4e60,0x766de0,0x4915f0,0x491740,0x48b580,0x48b640,0x4d7230]:
 raw=IMAGE.at(address,sizes[address]);assert raw==ACCEPTED.at(address,len(raw));ranges.append({'address':hex(address),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
 (HERE/f'property-domain-{address:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(raw,address))+'\n')
strings=[]
for address,text in [(0xa054e4,'Property'),(0xa0cdbc,'mobile')]:
 raw=(text+'\0').encode('utf-16le');assert IMAGE.at(address,len(raw))==ACCEPTED.at(address,len(raw))==raw;strings.append({'address':hex(address),'text':text,'sha256':hashlib.sha256(raw).hexdigest()})
vtable=IMAGE.at(0xa0cf88,32);assert vtable==ACCEPTED.at(0xa0cf88,32)
assert struct.unpack('<8I',vtable)==(0x483a40,0x93c990,0x6a71d0,0x408b50,0x491520,0x487760,0x48d490,0x48b580)
assets=ROOT.parent/'reconstruction/assets/original/Data';source=assets/'Properties/properties.tgi';manifest=assets/'index_aa.lst'
assert 'ids = mobile' in source.read_text() and 'Properties/*.tgi' in manifest.read_text()
report={'schema':1,'passed':True,'scope':'Static original/v14 code and original authored source pins; reserved first entry is proved from explicit bootstrap, not guessed file ordering.','original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':sha(Path(__file__)),'registry':{'global':'B36530','field':'0x464','kind':'Property','constructor':'47EFD0','arguments':[64,1],'initial_count':0,'initial_capacity':64,'reuse_policy':1,'vector_offset':'0x24','count_offset':'0x28','default_ordinal':0,'default_id':'mobile'},'evidence':['48B1B4..48B1BA constructs47EFD0(64,1);48B1E4 stores the registry atData+464.','47EFD0 initializes registry+24 pointer,count+28=0,capacity+2C=64 and base registry policy+8=1. Constructor alone reserves no entry.','48DCB7 calls47E9B0 CreatePredefinedProperties during native data bootstrap, before ordinary source loads.','47E9E3 supplies UTF16mobile to47D8D0.47E9F3 selectsData+464 then47EA02 calls4B4E60.47EA07..0D explicitly checks assigneddefinition+C==0.','4B4E60 validates no duplicate/out-of-order ID, assigns currentcount todefinition+C, appends766DE0 and registers its name491740.','48EEBC..48EEC2 pairsData+464 with native Property type nameA054E4.','48B580 with nonzero registry policy looks up existing IDS4915F0 and loads into the same definition;48B640 only creates absent IDs. Thus authoredmobile reuses reservedordinal0.','4D734A..4D737B ActorInitExisting readsData+464 vector+24[0]; requiresnoninvisible actor, nonnullProperty, Propertiescomponent and actual5035E0 applicability before5817A0.'],'ranges':ranges,'strings':strings,'vtable':{'address':'0xA0CF88','sha256':hashlib.sha256(vtable).hexdigest()},'original_source':{'path':'Properties/properties.tgi','sha256':sha(source),'ids':['mobile']},'manifest':{'sha256':sha(manifest),'pattern':'Properties/*.tgi'},'limits':['Only reservedmobile ordinal0 is established; remaining cross-file Property order is not inferred.','Source definition loading and dependency/effect application still require actual typed loader and runtime.','Property+24 enabled_target_flags is separate from is_technology+5C; no technology metadata bit is invented.']}
p=HERE/'property-domain-evidence.json';p.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'sha256':sha(p)}))
