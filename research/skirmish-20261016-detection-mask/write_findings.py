"""Pin this bounded fix and completed tests; original assets are read only."""
from pathlib import Path
import hashlib
import json
import re

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    capture_path = ROOT/'research/development10/native-source-capture-03/capture.json'
    capture = json.loads(capture_path.read_bytes())
    assert capture['completed_source_return'] and capture['original_error'] is None
    snapshots = [v for v in capture.values() if isinstance(v, list)]
    returns = [v for array in snapshots for v in array
               if isinstance(v, dict) and v.get('label')=='source_return']
    assert len(returns)==1
    observed = returns[0]
    assert observed['x87_control']==0x27f
    mines = [v for v in observed['actors'] if v['ids'] in ('minefield','beach_mine_pole')]
    assert len(mines)==2 and all(v['flags_174']==1090145 for v in mines)
    proof = json.loads((HERE/'native-detection-mask.json').read_bytes())
    assert len(proof['cases'])==2331
    before = json.loads((HERE/'final-source-tree-before.json').read_text(encoding='utf-8-sig'))
    after = json.loads((HERE/'final-source-tree-after.json').read_text(encoding='utf-8-sig'))
    assert before==after, 'Rules/metagame source changed during final checks'
    paths = [
        ROOT/'crates/aa-rules/src/native.rs',
        ROOT/'crates/aa-rules/src/native_object_definition.rs',
        ROOT/'crates/aa-rules/tests/native_detection_mask.rs',
        HERE/'native_detection_mask.py', HERE/'native-detection-mask.json', HERE/'write_findings.py',
        HERE/'constructor-218.asm.txt', HERE/'detection-block.asm.txt',
        HERE/'native-proof.log', HERE/'README.txt', capture_path,
        HERE/'backups/native.rs.before', HERE/'backups/native_object_definition.rs.before',
        HERE/'final-source-tree-before.json', HERE/'final-source-tree-after.json',
        HERE/'all-rules-tests.incomplete-before-source-freeze.log',
        HERE/'aborted-before-source-freeze.json',
        HERE/'all-rules-tests.earlier-source-root-guard.log',
        HERE/'source-tree.before-latest-root-guard.json',
    ]
    tests = {}
    for name in ['focused-tests.log','all-rules-tests.log','all-metagame-tests.log']:
        path = HERE/name
        text = path.read_text(encoding='utf-8-sig')
        counts = [tuple(map(int, m)) for m in re.findall(
            r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)]
        assert counts and not re.search(r'^error:|test result: FAILED', text, re.M), name
        assert all(failed==0 and ignored==0 for _,failed,ignored in counts), (name,counts)
        tests[name] = {'passed':sum(passed for passed,_,_ in counts),
                       'test_binaries':len(counts),'failed':0,'ignored':0,'sha256':sha(path)}
        paths.append(path)
    findings = {
        'schema':1, 'status':'native_fragment_and_required_real_source_regressions_passed',
        'original_sha256':proof['original_sha256'], 'v14_sha256':proof['v14_sha256'],
        'addresses':{'fresh_constructor':'4624B0','zero_ebx':'4624E1','store_control_zero':'462677',
                     'loader':'464BD0','detection_fragment':['465011','465089'],
                     'control_compare':'465061','remote_compare':'46506E',
                     'clear_bit22':'46507B','later_defender_control_store':'4657CA'},
        'condition':'Clear bit22 iff current+218 <=0 AND retained remote <=0, both ordered. Fresh+218 is constructor0; Defender is loaded later.',
        'observed_windows':{'capture_sha256':sha(capture_path),'precision':'Double53',
                            'control_word':observed['x87_control'],'mines':mines},
        'native_rows':2331,'present_rows':2304,'absent_rows':27,
        'precision':['Single24','Double53','Extended64'], 'required_checks':tests,
        'profiles':{'focused_detection':'default release: thinLTO, codegen-units1',
                    'all_rules_and_metagame':'release --config profile.release.lto=false --config profile.release.codegen-units=16'},
        'rust_source_tree_unchanged':True, 'rust_source_tree_files':len(before),
        'independent_review':'No independent reviewer acceptance claimed.',
        'seams':['5B4410 component lookup returns explicit presence',
                 '5B5130 returns explicit binary32 Fixed values via FLD32/RET4 stub'],
        'qualifications':[
            'Not full464BD0; mask/source projection only, not full loader exception or partial-store parity.',
            'Fresh mask API only; general nonzero control cases are isolated existing-object branch probes.',
            'NaN probes validate unordered mask decisions, not sNaN quieting, payload or x87 status.',
            'Observed Windows masks are PC53; allPC extension is proven by the isolated native fragment.',
            'No full-battle, multiplayer or arbitrary derived-definition/reload claim.',
            'Full CV v3 source/regeneration is owned by settlement_completion, not silently substituted here.'
        ],
        'files':[{ 'path':str(p.relative_to(ROOT)).replace('\\','/'), 'sha256':sha(p)} for p in paths],
    }
    output = HERE/'findings.json'
    output.write_text(json.dumps(findings, indent=2)+'\n')
    print(json.dumps({'findings_sha256':sha(output),'required_checks':tests}))

if __name__=='__main__':
    main()
