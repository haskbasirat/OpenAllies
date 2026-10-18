"""Seal the bounded author evidence after successful immutable replay logs exist."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 source=json.loads((HERE/'source-metadata.json').read_text())
 maps=json.loads((HERE/'actual-map-inputs.json').read_text())['maps']
 fixtures={name:json.loads((HERE/name).read_text()) for name in ('native-source-find.json','native-source-fields.json','native-battle-terrain.json')}
 assert len(maps)==240 and len({row['source'] for row in maps})==80
 changes=[dict(source=row['source'],precision=row['precision'],old_fallback=row['previous_fallback_count'],native_fallback=row['fallback_count'],flags_changed=row['flags_sha256']!=row['previous_flags_sha256'],aliases=row['changed_previous_aliases']) for row in maps if row['changed_previous_aliases']]
 assert len(changes)==12
 tests={
  'author-final-rust-replay.log':{'passed':1,'scope':'all80 actualmaps x3PC, source facts, both native PE ground results'},
  'existing-height-proofs.log':{'passed':6,'scope':'native_binding3/native_fixtures2/original_maps1'},
  'author-mapworld-proof.log':{'passed':5,'scope':'terrain4 plus independent lifecycle1'},
  'author-source-pc-guard-replay.log':{'passed':1,'scope':'final source-PC receipts plus all80 actualmaps x3PC replay, no value code change'},
 }
 for name,row in tests.items():
  log=(HERE/name).read_text(encoding='utf-8-sig')
  assert 'failed;' in log and '0 failed;' in log and 'error:' not in log.lower(),name
  row['sha256']=sha(HERE/name)
 files=['crates/aa-runtime/src/battle_terrain.rs','crates/aa-runtime/src/battle_terrain_tests.rs']+[str((HERE/name).relative_to(ROOT)).replace('\\','/') for name in ('source-metadata.json','actual-map-inputs.json','native-source-find.json','native-source-fields.json','native-battle-terrain.json','native_source_find.py','native_source_fields.py','native_battle_terrain.py','README.md')]
 report=dict(schema=1,status='bounded_owned_terrain_accepted_by_author',original_sha256=fixtures['native-battle-terrain.json']['original_sha256'],v14_sha256=fixtures['native-battle-terrain.json']['v14_sha256'],
  map_loads=240,original_maps=80,precision_controls=['Single24','Double53','Extended64'],
  source_counts=[dict(precision=row['precision'],terrains=len(row['terrains']),variations=len(row['variations']),biomes=len(row['biomes']),files=len(row['files']),translations=len(row['variation_translations'])) for row in source['precisions']],
  native_proofs=[dict(fixture=name,rows=len(doc[key]),pins=len(doc['instructions']),sha256=sha(HERE/name)) for name,key in [('native-source-find.json','lookups'),('native-source-fields.json','cases'),('native-battle-terrain.json','samples')] for doc in [fixtures[name]]],
  separate_image_witnesses=fixtures['native-source-find.json']['loaded_image_witnesses'],
  tests=tests,old_recursive_alias_reference_corrections=changes,files={name:sha(ROOT/name) for name in files},
  boundaries=['Typed source getter facts are explicit seams in scalar native snippets; full native TGI/texture loading is not executed.','Actual mapheight arrays are raw hash-checked source; ground native proof has no height/noise answer seam.','Resolution SVar override rules explicitly refused; none in80originalmaps.','No Worldstartup/ObjectInit/battle/pathfinding/dynamicterrain/multiplayer completion claim.','Original assets and release packages unchanged by this work.'])
 out=HERE/'findings.json';out.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(status=report['status'],sha256=sha(out))))
if __name__=='__main__':main()
