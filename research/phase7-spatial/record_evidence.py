"""Pin the implemented spatial scope and attributed validation, not self-review."""
from pathlib import Path
import json,hashlib
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).parent
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    fixtures={name:json.loads((HERE/name).read_text())for name in ['native-geometry.json','native-index.json','native-transforms.json','native-position.json','native-query.json']}
    for name,producer in [('native-geometry.json','original_geometry.py'),('native-index.json','original_index.py'),('native-transforms.json','original_transforms.py'),('native-position.json','original_position.py'),('native-query.json','original_query.py')]:
        assert fixtures[name]['producer_sha256']==H(HERE/producer)
    assert len(fixtures['native-geometry.json']['cases'])==792
    assert sum(len(c['steps'])for c in fixtures['native-index.json']['cases'])==432
    assert sum(len(c['steps'])for c in fixtures['native-transforms.json']['cases'])==288
    assert sum(len(c['steps'])for c in fixtures['native-position.json']['cases'])==576
    assert sum(len(c['steps'])for c in fixtures['native-query.json']['cases'])==2304
    source=['crates/aa-spatial/src/geometry.rs','crates/aa-spatial/src/index.rs','crates/aa-spatial/src/index_world.rs',
        'crates/aa-spatial/src/position.rs','crates/aa-spatial/tests/position_native.rs',
        'crates/aa-spatial/src/query.rs','crates/aa-spatial/src/query_tests.rs',
        'crates/aa-spatial/tests/geometry_native.rs','crates/aa-spatial/tests/live_maps.rs','crates/aa-spatial/Cargo.toml']
    files=['README.txt','record_evidence.py','original_geometry.py','original_index.py','original_transforms.py','original_position.py','original_query.py','query-validation.json','query-integration-tests.log','query-integration-clippy.log','terrain-mask-rename.json',*fixtures.keys()]
    deps=['crates/aa-spatial/src/lib.rs','crates/aa-spatial/src/collision.rs','crates/aa-spatial/src/height_layers.rs','crates/aa-numeric/src/lib.rs',
        'crates/aa-heightfield/src/lib.rs','crates/aa-heightfield/src/binding.rs','crates/aa-rules/src/objects.rs','crates/aa-rules/src/native.rs',
        'crates/aa-mapworld/src/terrain.rs','research/phase6-spatial/native-real-map-terrain.json','research/phase4-rng/original_rng.py',
        'crates/aa-spatial/src/shape_overlap.rs','research/phase7-shape-overlap/independent-review.json']
    validation=json.loads((HERE/'query-validation.json').read_text());assert validation['all_passed']
    report={'schema':1,'passed':True,'scope':'Owned native tile membership, ordinary transforms and prepared live first-hit queries with actual terrain/height bindings; mandatory component/policy boundaries',
        'native_geometry_cases':792,'native_membership_sequences':48,'native_membership_steps':432,'native_transform_sequences':96,'native_transform_steps':288,
        'source_maps':80,'source_ground_definitions':7,'prepared_map_moves':560,
        'native_position_callback_sequences':192,'native_position_callback_steps':576,
        'native_query_sequences':384,'native_queries':2304,
        'validation':validation['runs'],'crate_tests_passed':26,
        'source_sha256':{p:H(ROOT/p)for p in source}|{str((HERE/p).relative_to(ROOT)).replace('\\','/'):H(HERE/p)for p in files},
        'dependency_sha256':{p:H(ROOT/p)for p in deps},
        'limits':['ActorPositionChanged outer orchestration and native small state leaves are concrete; cell/fog/terrain effects, linked actor movement and larger component services remain required fallible implementations, not claimed by the intercepted fixtures.',
          'Collision geometry is concrete; descriptor policy requires live definitions, allegiance, organization and virtual semantics.',
          'New query fixtures execute original preparation, scan, shape and marks, but explicitly intercept allocation/free, source/custom policy and definition ObjectData-view callbacks. A complete real-map query backend is not claimed.',
          'Ordinary immutable native shape getters only; visual scale is intentionally not applied.',
          'Finite arithmetic domain, callback lifetime constraints and source-compatible identity representations are explicit.',
          'Real-map test is composition of already source-proved pieces with prepared actors/callbacks, not a native full match or save restoration.'],
        'independent_review':'Not self-approved; separate reviewer report required.'}
    path=HERE/'findings.json';path.write_text(json.dumps(report,indent=2)+'\n');print({'sha256':H(path)})
if __name__=='__main__':main()
