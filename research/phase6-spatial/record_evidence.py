"""Reproduce Phase6 spatial/height/movement checks and bind current sources."""
from pathlib import Path
import hashlib,json,re,subprocess,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
    commands=[]
    for name,command in [
        ('binding-oracle.log',[sys.executable,str(OUT/'original_height_binding.py')]),
        ('maps-oracle.log',[sys.executable,str(OUT/'original_map_heights.py')]),
        ('walk-oracle.log',[sys.executable,str(OUT/'original_tile_walk.py')]),
        ('rust-tests.log',['cargo','test','-p','aa-heightfield','-p','aa-spatial','-p','aa-movement','--','--include-ignored','--nocapture']),
        ('clippy.log',['cargo','clippy','-p','aa-heightfield','-p','aa-spatial','-p','aa-movement','--all-targets','--','-D','warnings']),
    ]:
        run=subprocess.run(command,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        (OUT/name).write_bytes(run.stdout)
        commands.append({'command':command,'exit_code':run.returncode,'log':name,'sha256':sha(OUT/name)})
        assert run.returncode==0,run.stdout.decode(errors='replace')
    tests=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',(OUT/'rust-tests.log').read_text())))
    assert tests==18,tests
    binding=json.loads((OUT/'native-height-binding.json').read_text())
    maps=json.loads((OUT/'native-real-map-heights.json').read_text())
    walks=json.loads((OUT/'native-tile-walk.json').read_text())
    files=[]
    for crate in ['aa-heightfield','aa-spatial','aa-movement']:
        directory=ROOT/'crates'/crate
        files.extend([directory/'Cargo.toml',directory/'README.md'])
        files.extend(sorted((directory/'src').glob('*.rs')))
        files.extend(sorted((directory/'tests').glob('*.rs')))
    files.append(ROOT/'crates/aa-client/src/mechanics_check.rs')
    files += [OUT/name for name in ['README.txt','original_height_binding.py','original_map_heights.py','original_tile_walk.py','native-height-binding.json','native-real-map-heights.json','native-tile-walk.json','record_evidence.py']]
    dependencies=[ROOT/name for name in ['crates/aa-numeric/src/lib.rs','crates/aa-trig/src/lib.rs','crates/aa-trig/src/angle.rs','crates/aa-mapworld/src/archive.rs','crates/aa-mapworld/src/header.rs','research/phase6-mapheader/inspect_header.py','research/phase4-rng/original_rng.py']]
    report={'schema':1,'passed':True,'component':'Native height construction/input, exact cell walk and fallible ground-world boundary',
        'native_cases':{'constructors':len(binding['constructor']),'v1_loads':len(binding['loads']),'real_map_samples':len(maps['samples']),'tile_walks':len(walks['fixtures'])},
        'actual_maps_bound':len(maps['maps']),'actual_height_layers_bound':len(maps['maps'])*2,'rust_tests':tests,
        'source_sha256':{p.relative_to(ROOT).as_posix():sha(p)for p in files},'dependency_sha256':{p.relative_to(ROOT).as_posix():sha(p)for p in dependencies},
        'commands':commands,'source_images':{'original':binding['original_sha256'],'v14':binding['v14_sha256']},
        'limits':['Actual height reader551E60 only supportsversion1 here; version0 world-dependent flags rebuild is rejected. Writer5500F0 is never used as input execution.',
            'Original-map fixture supplies the source-pinned resolution1 value and verifies no matching decoded rule override; runtime precision/resolution remain explicit caller inputs.',
            'Height samplers preserve extended returns; raw sampleflags and unread chunk tails are retained without guessed semantics.',
            'Tile traversal supplies exact candidate order, not passability or collision answers.',
            'Movement callback failures stop later work but retain prior native field writes and host effects; no automatic retry/rollback.',
            'Full shape collision, tile ownership/membership, live actor registry integration and the complete world adapter remain unimplemented.',
            'No original game process, map, asset, prior release or user configuration modified.'],
        'peer_review':'Recorded separately; do not treat this producer ledger as independent review.'}
    (OUT/'findings.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'passed':True,'tests':tests,'native_cases':report['native_cases'],'sha256':sha(OUT/'findings.json')}))
if __name__=='__main__':main()
