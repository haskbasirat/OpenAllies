"""Record the final owned source/fixture pins and bounded test evidence.
This is an evidence manifest writer, not a native oracle or runtime capability.
"""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
def h(p):return hashlib.sha256(p.read_bytes()).hexdigest()
owned=['crates/aa-runtime/src/map_resource_mode.rs','crates/aa-runtime/tests/map_resource_mode.rs',
       'crates/aa-runtime/src/load_tail_source.rs','crates/aa-runtime/src/load_tail_source_tests.rs',
       'crates/aa-runtime/src/load_tail_world.rs','crates/aa-runtime/src/load_tail_binding.rs']
files=owned+[str(p.relative_to(ROOT)).replace('\\','/') for p in sorted(HERE.glob('native*')) if p.is_file()]
files+=['research/development10/map-resource-mode/load_tail_world-before-borrowed-session.rs',
        'research/development10/map-resource-mode/actual-source-priorities-final.json',
        'research/development10/map-resource-mode/priority-tests-final-v7.log',
        'research/development10/map-resource-mode/combined-tail-tests-v1.log',
        'research/development10/map-resource-mode/gmap-extent-replay-final.log']
pins={p:h(ROOT/p) for p in files}
reports=json.loads((HERE/'actual-source-priorities-final.json').read_text())
summary=[]
for row in reports:
    values=row['definitions'];summary.append(dict(map=row['map'],count=len(values),
        nonzero=[dict(id=v['canonical'],value=v['value']) for v in values if v['value']]))
findings=dict(schema=1,status='author scoped validation passed; independent review separate',
    changes=['native WorldRuleMode/task dispatch helpers',
             'retained World constructor TailState',
             'required source integer starting-location priority through same moved Bank',
             'borrowed SessionRead interface with bounded old-fixture compatibility',
             'actual Staged post-record load-tail adapter and GMap float extent binding'],
    tests=[dict(log='priority-tests-final-v7.log',passed=4,failed=0,
                invocation='direct final subject binary aa_runtime-2264c0723cc57ac2.exe load_tail_source::tests --include-ignored',
                subject='final frozen source/test hashes; binary produced by tests-final-v1 FirstTick compile'),
           dict(log='combined-tail-tests-v1.log',passed=10,failed=0,
                subject='existing7 load_tail_world plus3 mode tests; these subject source hashes remain unchanged')],
    native=[dict(fixture='native-map-resource-mode.json',rows=144,pins=232),
            dict(fixture='native-starting-priority.json',rows=42,pins=44),
            dict(fixture='native-tail-constructor.json',rows=18,pins=166),
            dict(fixture='native-gmap-extent.json',rows=30,pins=16)],
    actual_map_priority_summary=summary,
    qualifications=['No complete restoration/PostLoad/BeginPlaying/battle claim from these source tests.',
        'Actual concrete tail requires complete PostLoaded records, same retained World/Bank, actual SessionRead and Kingdom membership owners.',
        'Reentrant mutations during read-only binding assembly and native invalid-pointer/exception unwind are outside this successful adapter domain.',
        'Saved/metagame world-start service remains an explicit required binding at its native branch.',
        'Superseded failed compile snapshots retained; none counted as passing validation.'],
    pins=pins)
(HERE/'findings.json').write_text(json.dumps(findings,indent=2)+'\n')
(HERE/'final-freeze.json').write_text(json.dumps(dict(schema=1,pins=pins,
    findings_sha256=h(HERE/'findings.json')),indent=2)+'\n')
print(json.dumps(dict(findings_sha256=h(HERE/'findings.json'),freeze_sha256=h(HERE/'final-freeze.json'),maps=summary)))
