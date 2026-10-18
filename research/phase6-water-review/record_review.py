"""Bind the completed independent native/source review and attributed integration tests."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
geometry=json.loads((HERE/'geometry-replay.json').read_text());shading=json.loads((HERE/'shading-replay.json').read_text())
assert geometry['passed'] and geometry['maps']==80 and shading['passed']
validation=json.loads((ROOT/'research/phase6-water/validation-record.json').read_text())
for name,expected in validation['source_sha256'].items():assert sha(ROOT/name)==expected,name
captures=[]
for name,frames in [('crete',5),('stalingrad',3)]:
    path=ROOT/f'qa-output/phase6/water/{name}-final-native-source.json';row=json.loads(path.read_text())
    assert row['render_frames']==frames and row['draw_order_rebuilds']==1
    warning=next(v for v in row['warnings'] if v.startswith('Static native water:'))
    assert '0 unresolved' in warning
    captures.append({'name':name,'json_path':path.relative_to(ROOT).as_posix(),'json_sha256':sha(path),
        'png_sha256':sha(path.with_suffix('.png')),'adapter':row['adapter'],'water_status':warning,
        'visually_inspected_by_reviewer':True,'original_pixel_parity_claimed':False})
files=['crates/aa-client/src/water_render.rs','crates/aa-client/src/water_surface.rs','crates/aa-client/src/water_geometry.rs',
    'crates/aa-client/src/loading.rs','crates/aa-mapworld/src/terrain.rs','crates/aa-mapworld/src/header.rs',
    'crates/aa-heightfield/src/lib.rs','crates/aa-heightfield/src/binding.rs','crates/aa-spatial/src/lib.rs',
    'crates/aa-spatial/src/height_layers.rs','research/phase6-water/recover_water.py',
    'research/phase6-water/native-water-vectors.json','research/phase6-water/water-metadata.json',
    'research/phase6-water/FINDINGS.txt','research/phase6-water/validation-record.json',
    'research/phase6-water-geometry/native_water_geometry.py','research/phase6-water-geometry/native-water-geometry.json',
    'research/phase6-water-geometry/findings.json','research/phase6-water-geometry/README.txt',
    'research/phase6-terrain/terrain-render-metadata.json','research/phase6-terrain/native-defaults.json',
    'research/phase6-spatial/terrain-findings.json','research/phase6-spatial/independent-review-terrain.json',
    'research/engine-baseline/terrain-variation-metadata.json']
files += [p.relative_to(ROOT).as_posix()for p in HERE.iterdir()if p.suffix in('.py','.json')and p.name!='independent-review.json']
report={'schema':1,'passed':True,'reviewer':'orders_spatial','production_edits_by_reviewer':False,'findings':[],
    'scope':'Static original-source water preview: effective terrain selection, raw height/depth queries, authored color curve, saved cache footprint and GPU integration inputs. Full water lifecycle and native final-pixel parity remain unclaimed.',
    'independent_native_validation':{'saved_map_cache_readers_and_lookups':80,'curve_cases':888,'quad_cases':16,'cell_selector_cases':4608,'pixel_identical_first_frames':8},
    'rust_validation':{'passed':7,'attribution':'Four curve/topology/selector/Crete production tests executed by material_render; three geometry tests (including all80 maps) executed by terrain_roads with AA_GAME_ROOT set. Sources remained frozen. Initial missing-environment failure did not require source changes.',
        'strict_clippy':'Passed in both owners final executions, recorded with attribution in their evidence; root owns the final workspace gate.'},
    'source_sha256':{name:sha(ROOT/name)for name in files},'captures':captures,
    'checks':[
        'Independent native replays regenerated all committed geometry and shading fixtures in memory, compared complete outputs (excluding geometry wall-clock duration only), and wrote separate reviewer copies without overwriting source evidence.',
        'Original suffix reader byte offsets, complete vertex/index hashes and real cell hash lookup outcomes match on all80 maps; missing cache entries, existing empty ranges and nonempty triangle ranges remain distinct.',
        'Raw TerrainChunk binding preserves saved source order and original missing-ID carry/default behavior. Effective definitions drive both selected water material and height-query type/noise; no unknown kind0 substitute remains.',
        'Original736B17 subtracts0.5 from the extended base-height return and stores surfaceZ.736B4E reloads that stored word before4F0E50;736B77 subtracts the complete ground result before depth storage. The Rust composition preserves these boundaries.',
        'The4F0E50 overlay condition is any kind1/2 neighbor; mountain noise still depends on the current cell. The preview uses explicit Single24 and exposes required non-height callbacks as errors.',
        'Native current-or-first-ring water selector, regular quad order and authored depth interpolation match all original vectors. All authored metadata curves have distinct depth breakpoints.',
        'Eight original DDS first-frame images independently decode pixel-identically to recovered PNG inputs; original texture data is unchanged.',
        'Loading appends the water draws and warnings without deleting model placement records or bypassing visibility/material handling. Binding failure preserves the map and reports omitted water explicitly.',
        'Final Crete and Stalingrad captures were visually inspected. Both report zero unresolved water cells; Stalingrad exercises saved cache triangles, Crete exercises the regular-quad fallback. These are functional captures, not original screenshot equivalence tests.',
    ],
    'limits':[
        'Static frame0 at zero scroll origin; flip controller, wind/scroll lifecycle, shoreline cache regeneration, waves/doodads and reflections remain incomplete.',
        'Native whole-engine rendering FPU state is unproven; preview arithmetic explicitly uses Single24. Native generated-water sphere propagation and world traversal remain incomplete.',
        'Modern sRGB/linear lighting and blending are not the original full fixed-function pipeline. Underlying terrain geometry still derives from rounded viewer inputs; small ground/water seams may remain.',
        'Runtime verifies referenced terrain TGI hashes. Template/translation/biome metadata provenance is pinned and tested against the known corpus, not fully re-evaluated from arbitrary edited source at runtime.',
        'Native fallback when WaterInfo is absent is not implemented; such unsupported cells are explicitly counted/skipped rather than assigned a guessed color.',
        'GPU cache functional/timing validation is tracked separately under research/phase6-render-cache; no FPS or controlled performance improvement is asserted here.',
    ]}
(HERE/'independent-review.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'passed':True,'native_cache_maps':80,'rust_tests_attributed':7,'sha256':sha(HERE/'independent-review.json')}))
