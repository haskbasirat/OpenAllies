"""Author evidence manifest; refuses to attest a missing/failed test log.
This file grants no runtime capability and is not an independent review.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
def h(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    name=sys.argv[1];log=HERE/name
    assert log.parent==HERE and log.is_file()
    assert 'test result: ok. 3 passed; 0 failed' in log.read_text(),name
    paths=['crates/aa-runtime/src/live_first_tick.rs',
           'crates/aa-runtime/src/live_first_tick_tests.rs']
    paths += [str(p.relative_to(ROOT)).replace('\\','/') for p in sorted(HERE.glob('native*')) if p.is_file()]
    paths += ['research/development10/live-first-tick/'+p for p in
              ['actual-source-inputs.json',name,'README.txt','nation-source-replay-final.log']]
    pins={p:h(ROOT/p) for p in paths}
    fixture=json.loads((HERE/'native-first-tick.json').read_text())
    nation=json.loads((HERE/'native-nation-music.json').read_text())
    actual=json.loads((HERE/'actual-source-inputs.json').read_text())
    inputs=[dict(id=v['nation'],audio=v['audio']['id'],clips=len(v['audio']['files']['paths'])) for v in actual['nation_inputs']]
    assert len(inputs)==5
    findings=dict(schema=1,status='author scoped validation passed; independent review separate',
        source_sha256=pins[paths[0]],test_sha256=pins[paths[1]],
        tests=dict(log=name,passed=3,failed=0,
            cases=['72 actual native selection/error rows',
                   'required output callback/storage partial-effects',
                   'actual source input binding with laboratory bound-null SessionRead/output']),
        native=[dict(fixture='native-first-tick.json',rows=len(fixture['rows']),pins=len(fixture['pins'])),
                dict(fixture='native-nation-music.json',rows=len(nation['rows']),pins=len(nation['pins']))],
        actual_nation_inputs=inputs,observer=actual['observer_request'],
        qualifications=['No full747630/Miles backend, voice/channel/playlist parity claim.',
            'Actual callback wiring/device playback and live BeginPlaying remain separate integration tests.',
            'The actual source test uses an explicitly laboratory bound-null SessionRead and laboratory output; it does not certify a live local Player+28 or audible playback.',
            'Controlled fresh source AVar/Nation values only; arbitrary later profile/setter/current-definition mutations are not inferred.',
            'Selected clip paths are source metadata; this module does not choose/shuffle a playlist.',
            'Native source/name/registry/fatal/backend service seams and no unwind parity are declared in the fixture/readme.',
            'Superseded failed test/compile logs remain retained and excluded.'],pins=pins)
    (HERE/'findings.json').write_text(json.dumps(findings,indent=2)+'\n')
    (HERE/'final-freeze.json').write_text(json.dumps(dict(schema=1,pins=pins,findings_sha256=h(HERE/'findings.json')),indent=2)+'\n')
    print(json.dumps(dict(findings_sha256=h(HERE/'findings.json'),freeze_sha256=h(HERE/'final-freeze.json'),nation_inputs=inputs)))
if __name__=='__main__':main()
