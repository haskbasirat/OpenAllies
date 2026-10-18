"""Read-only two-image static ownership receipt. No new native execution claim."""
from pathlib import Path
import sys,json,hashlib,csv
folder=Path(__file__).resolve().parent;root=folder.parents[1]
sys.path.insert(0,str(root/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED
size={int(r['address'],16):int(r['bodyBytes'])for r in csv.DictReader((root.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')}
blocks=[]
for addr,length in [(0x48af85,0x13),(0x480ed0,size[0x480ed0]),(0x47cd40,size[0x47cd40]),(0x47ce00,size[0x47ce00]),(0x93d4e0,size[0x93d4e0]),(0x483a20,size[0x483a20]),(0x43fca4,0x4d),(0x555f20,size[0x555f20])]:
 a=IMAGE.at(addr,length);b=ACCEPTED.at(addr,length);assert a==b
 blocks.append({'address':hex(addr),'length':length,'bytes':a.hex(),'sha256':hashlib.sha256(a).hexdigest(),'original_v14_equal':True})
files=['crates/aa-rules/src/native_load.rs','crates/aa-runtime/src/battle_terrain.rs','research/phase8-definitions/tag-managers.json','research/skirmish-20261017-view-grid/gdata630-constructor-disassembly.log']
out={'scope':'Static ownership and existing canonical registry binding, not a fresh native execution or captured count',
 'images':[hashlib.sha256(p.data).hexdigest()for p in [IMAGE,ACCEPTED]],'blocks':blocks,
 'chain':['48AF88 LEA ECX,[GameData+60C];48AF93 CALL480ED0 constructs Terrain DataManager mode0','480ED0 sets manager+24 array allocation and +28 count0; capacity17 is allocation capacity, never the initialized count','47CD40 vf1C mode0 reaches47CE00 Create; duplicates fail before creation','47CE00 assigns TerrainData+C from manager+28 then invokes real93D4E0 on manager+24','93D4E0->483A20 reserves one growable slot, stores incremented count then writes current object into returned tail slot','Native Terrain generic root appends own entry; Variation children append to separate manager atGameData+658','43FCA4 reads GameData+634 to size/fill VWorld130/134 slots','555F20 uses GameData630 entry+88 water_doodad_IDS ActorData reference; this is a Terrain registry, not Actor or selected palette'],
 'canonical_binding':'Catalog.actors.native_data_registry("Terrain").ok_or(required)?.entries.len(); retain source owner and native registration order; do not use source tag census, palette or captured/default count',
 'source_pins':{p:hashlib.sha256((root/p).read_bytes()).hexdigest()for p in files},
 'limitations':['Does not port TerrainData positive water-doodad reference loader or555F20 deletion','No new all-PC execution is claimed for this lookup; existing native generic-manager/source-loader proof remains its own scope','Factory uses current canonical registry, no literal count is supplied'],}
(folder/'gdata630-static-receipt.json').write_text(json.dumps(out,indent=2)+'\n')
print(hashlib.sha256((folder/'gdata630-static-receipt.json').read_bytes()).hexdigest())
