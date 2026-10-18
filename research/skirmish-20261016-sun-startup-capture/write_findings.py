"""Pin the completed Sun startup input component; no native re-execution."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def main():
    assert '2 passed; 0 failed; 0 ignored' in (HERE/'runtime-tests.log').read_text()
    assert json.loads((HERE/'post-process-inventory.json').read_bytes()) == []
    files = [ROOT/'crates/aa-runtime/src/load_sun_reference.rs',
             ROOT/'crates/aa-runtime/tests/load_sun_reference.rs']
    files += [HERE/name for name in ['prepare_tracer.py', 'capture_sun_startup.py',
        'capture-01/capture.json', 'capture-01/runtime-files.json',
        'seal_sun_reference.py', 'sun-startup-reference.json',
        'runtime-tests.log', 'post-process-inventory.json', 'README.txt']]
    result = {
        'schema': 1,
        'status': 'frozen startup input verifier; no Sun constructor observation/map/battle claim',
        'native_address': 'B2814C/B28150/B28154',
        'native_image_storage': 'initialized writable .data raw bytes, not PE zero-fill tail',
        'observed_words': [0,0,0], 'native_writes_before_cutoff': 0,
        'threads_armed': 62, 'post_cutoff_watch_calibration_probes': 3,
        'source_return_and_session_ctor_observed': True,
        'sun_ctor_itself_observed': False,
        'support_files_independently_copied_and_checked': 54,
        'all1028_rows_and_source_FP_numeric_match_frozen_capture03': True,
        'original_profiles_and_archives_unchanged': True,
        'cleanup': 'normal wait0/exit0/EXIT_PROCESS/errors[]; PID50308 gone',
        'production': 'private ConstructorInput binds sealed Catalog/source/FP target; create_sun consumes input after current target revalidation and calls separately proved sun_state::State::new',
        'tests': {'passed':2, 'failed':0, 'ignored':0,
                  'profile':'release --locked --offline -j4 --config profile.release.lto=false --config profile.release.codegen-units=16'},
        'independent_review': 'world_startup_gap narrow tracer diff accepted; raw hardware receipt review pending separately at freeze',
        'limits': ['not a Sun constructor dynamic observation', 'no map/battle/perpetual immutability claim',
            'Sun native constructor and exact read-only address-reference audit are separate sun_state evidence',
            'inherited pre-try setup limitation; accepted success requires recorded final cleanup',
            'two original CreateProcess handles close with bounded Python process exit; duplicates close explicitly',
            'focused tests, not full workspace or final EXE verification'],
        'all_owned_commands_reaped': True,
        'pins': {str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in files},
    }
    out = HERE/'findings.json'
    with out.open('x') as stream:
        json.dump(result,stream,indent=2)
        stream.write('\n')
    print({'findings_sha256':sha(out),'pins':len(files)})

if __name__ == '__main__':
    main()
