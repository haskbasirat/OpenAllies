"""Reproduce original terrain vectors and bind the Rust input adapter evidence."""
from pathlib import Path
import hashlib,json,subprocess,sys
OUT=Path(__file__).resolve().parent;ROOT=OUT.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
commands=[
    [sys.executable,'research/phase6-spatial/native_terrain.py'],
    [sys.executable,'research/phase6-spatial/original_map_terrain.py'],
    ['cargo','test','-p','aa-mapworld','--test','terrain','--test','terrain_corpus','--','--include-ignored','--nocapture'],
    ['cargo','clippy','-p','aa-mapworld','--all-targets','--','-D','warnings'],
]
logs=[]
for i,cmd in enumerate(commands):
    run=subprocess.run(cmd,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    log=OUT/f'terrain-evidence-{i}.log';log.write_bytes(run.stdout)
    assert run.returncode==0,run.stdout.decode(errors='replace')
    logs.append({'command':cmd,'exit_code':run.returncode,'log':log.name,'sha256':sha(log)})
files=[
    'crates/aa-mapworld/src/terrain.rs','crates/aa-mapworld/tests/terrain.rs','crates/aa-mapworld/tests/terrain_corpus.rs',
    'crates/aa-mapworld/src/archive.rs','crates/aa-mapworld/src/inventory.rs','crates/aa-mapworld/src/header.rs',
    'crates/aa-numeric/src/lib.rs','research/phase6-spatial/README-terrain.txt',
    'research/phase6-spatial/native_terrain.py','research/phase6-spatial/original_map_terrain.py',
    'research/phase6-spatial/record_terrain_evidence.py','research/phase6-spatial/native-terrain.json',
    'research/phase6-spatial/native-real-map-terrain.json','research/phase6-spatial/original_height_binding.py',
    'research/phase4-rng/original_rng.py','research/phase6-mapheader/inspect_header.py',
    'research/phase6-terrain/terrain-render-metadata.json','research/phase6-terrain/native-defaults.json',
    'research/engine-baseline/terrain-variation-metadata.json',
]
v=json.loads((OUT/'native-terrain.json').read_text());maps=json.loads((OUT/'native-real-map-terrain.json').read_text())['maps']
report={'schema':1,'passed':True,'scope':'Native terrainv6/v7 source input, exact static tile flags and explicit missing-definition carry fallback. Dynamic membership, region/auxiliary restoration and live world publication remain separate.',
    'native_layout_cases':len(v['layouts']),'native_tile_load_cases':len(v['loads']),
    'native_passability_cases':len(v['flags']),'native_fallback_sequences':len(v['fallback']),
    'original_maps':len(maps),'original_tiles':sum(r['tile_count']for r in maps.values()),
    'original_native_layouts':len(maps),'native_source_sample_cells':sum(len(r['native_sampled_cells'])for r in maps.values()),
    'source_fallback_cells':sum(r['fallback_count']for r in maps.values()),'rust_tests':5,'strict_clippy_passed':True,
    'source_sha256':{name:sha(ROOT/name)for name in files},'commands':logs,
    'limits':['Typed registry inputs and evaluated MapGTileResolutionBits are required; no implicit precision/definition/world is invented.',
        'The original80 complete grids are checked against source-bound native-derived formulas; only720 source cells run through native tile load, as documented.',
        'Stream game mode isfalse; saved region references are preserved but no live registry handles are installed.',
        'Native first-two missing-variation localized diagnostics are an intercepted presentation boundary. Missing counters and effective carried definitions execute.',
        'Original allocation/index representability is checked. No global hardware memory or bandwidth cap is imposed by this adapter.']}
(OUT/'terrain-findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'passed':True,'tests':5,'maps':len(maps),'tiles':report['original_tiles'],'sha256':sha(OUT/'terrain-findings.json')}))
