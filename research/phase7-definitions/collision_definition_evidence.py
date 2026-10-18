from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parents[1]/'research/phase5-actors'))
from original_creation import P,V,H
log=(HERE/'collision-definition-verification.txt').read_text(encoding='utf-8-sig')
rows=next(json.loads(line) for line in log.splitlines() if line.startswith('[{'))
assert len(rows)==6 and all(not r['gate'] and not r['settlement_wall'] and r['required_object'] is None and r['source_count']==1847 and r['wall_reference_count']==0 for r in rows)
(HERE/'collision-definition-inputs.json').write_text(json.dumps(rows,indent=2)+'\n')
windows=[(0x462570,0x46258c,'ActorData constructor initializes settlement-wall byte1A8 to0'),(0x462b0a,0x462b25,'ActorData constructor initializes Wall gate42C to0'),(0x462c19,0x462c60,'ActorData constructor initializes required object/predecessor pointers'),(0x4678db,0x4678fa,'WallComponent gate GetBool assignment42C'),(0x4640f0,0x4649e0,'Settlement loader typed placeholder callbacks and positive1A8 writes'),(0x4606d0,0x460940,'Construction deferred required_object loader and validation'),(0x45fae0,0x45fc00,'Required object predecessor inheritance')]
pins=[]
for a,b,meaning in windows:
 raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw),'meaning':meaning})
out={'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'source_report_sha256':H((HERE/'collision-definition-inputs.json').read_bytes()),'source_counts':{'cases':len(rows),'files':1847,'assigned_wall_reference_count':0},'ranges':pins,'scope':['Static original/v14 loader and constructor assignment proof plus typed original-source projection tests; not a full Settlement loader execution.','Complete conservative syntax census includes uninstantiated/ambiguous templates; any matching positive placeholder reference rejects the negative certification.','Nonempty or unprovable Upgrade graph rejects required_object predecessor assumptions.','Two original source parse omissions are recorded; their hash-checked single-byte text contains none of the three relevant placeholder field names, and no Upgrade byte sequence.','Gate can be true from authored WallComponent; the original constructor provides false only when absent. Required object remains canonical MARKER identity with native radius/tile-lock checks.']}
(HERE/'collision-definition-evidence.json').write_text(json.dumps(out,indent=2)+'\n');print(H((HERE/'collision-definition-evidence.json').read_bytes()))

