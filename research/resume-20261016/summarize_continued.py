"""Seal scoped results without treating tests/maps as game completion."""
from pathlib import Path
import hashlib,json,re

ROOT=Path(__file__).resolve().parents[2]
QA=ROOT/'qa-output/resume-20261016'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def main():
    expected={'continued-actors.log':12,'continued-mind.log':10,
        'continued-runtime-lib.log':26,'continued-runtime-final-integration.log':20}
    logs={};passed=0
    for name,count in expected.items():
        p=QA/name;t=p.read_text(encoding='utf-8-sig')
        actual=sum(map(int,re.findall(r'test result: ok\. (\d+) passed; 0 failed',t)))
        assert actual==count and 'test result: FAILED' not in t,(name,actual)
        passed+=actual;logs[name]={'sha256':sha(p),'passed':actual}
    initial=QA/'continued-runtime-integration.log';t=initial.read_text(encoding='utf-8-sig')
    initial_passed=sum(map(int,re.findall(r'test result: ok\. (\d+) passed; 0 failed',t)))
    assert initial_passed==8 and 'c.boolean == 2 && c.trailing == 255' in t
    passed+=initial_passed
    logs[initial.name]={'sha256':sha(initial),'passed_before_superseded_failure':initial_passed,
        'superseded_failure':'Test expected command boolean/trailing bytes reversed; final native-field comparison target passed.'}
    check=QA/'continued-workspace-check.log';t=check.read_text(encoding='utf-8-sig')
    assert 'Finished' in t and 'error[' not in t and '\nerror:' not in t
    logs[check.name]={'sha256':sha(check),'result':'passed','client_existing_warnings':13}
    census_log=QA/'continued-mode1-inventory.log';t=census_log.read_text(encoding='utf-8-sig')
    assert 'test result: ok. 1 passed; 0 failed' in t
    logs[census_log.name]={'sha256':sha(census_log),'passed':1}
    comparison=read(QA/'continued-mode1-comparison.json')
    assert comparison['before']['native_load_mode']==comparison['after']['native_load_mode']==1
    baseline=read(QA/'skirmish-checkpoint-validation.json')['source_hashes']
    changed=[name for name,value in baseline.items() if (ROOT/name).is_file() and sha(ROOT/name)!=value]
    sensitive=[name for name in changed if name.startswith(('crates/aa-rules/','crates/aa-metagame/'))
        or name in ['Cargo.toml','Cargo.lock']]
    assert not sensitive,sensitive
    source={str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in (ROOT/'crates').rglob('*.rs')}
    result={'schema':1,'scope':'Post-Development08 faithful source continuation; skirmish first, multiplayer second',
        'selected_tests_passed':passed,'diagnostic_census_test_passed':1,
        'workspace_all_targets_compile':'passed','logs':logs,'map_comparison':comparison,
        'changed_baseline_source_paths':changed,'new_rust_paths':sorted(set(source)-set(baseline)),
        'source_sha256':source,'rules_metagame_manifests_unchanged':True,
        'no_new_full_rules_metagame_run_reason':'No aa-rules/aa-metagame source or workspace manifest/lock changed.',
        'strict_clippy':'Not globally green; preexisting style lints documented by authors.',
        'not_claimed':['Playable skirmish or complete world publication','Live Regiment state execution',
            'Full HQ recruitment/non-null-owner Economy refresh','New EXE/release','Real LAN/internet battle',
            'Full original save writing/compatibility','Completion percentage']}
    p=QA/'continued-acceptance.json'
    with p.open('x',encoding='utf-8') as f:json.dump(result,f,indent=2);f.write('\n')
    print(json.dumps({'selected_tests_passed':passed,'diagnostic_census_test_passed':1,
        'baseline_changed_paths':len(changed),'new_rust_paths':len(result['new_rust_paths']),
        'after':comparison['after'],'report_sha256':sha(p)}))
if __name__=='__main__':main()
