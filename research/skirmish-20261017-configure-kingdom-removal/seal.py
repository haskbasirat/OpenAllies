from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def pin(path):
 p=ROOT/path
 return dict(path=path,sha256=hashlib.sha256(p.read_bytes()).hexdigest())
files=['crates/aa-runtime/src/configure_kingdom_removal.rs','crates/aa-runtime/src/configure_kingdom_removal_tests.rs',
 'crates/aa-runtime/src/actor_uninit.rs','crates/aa-runtime/src/map_replacement_drain.rs',
 'research/skirmish-20261017-configure-kingdom-removal/native_kingdom_f0.py',
 'research/skirmish-20261017-configure-kingdom-removal/native-kingdom-f0.json',
 'research/skirmish-20261017-configure-kingdom-removal/native_session_scan.py',
 'research/skirmish-20261017-configure-kingdom-removal/native-session-scan.json',
 'research/skirmish-20261017-configure-kingdom-removal/native_kingdom_e0.py',
 'research/skirmish-20261017-configure-kingdom-removal/native-kingdom-e0.json',
 'research/skirmish-20261017-configure-kingdom-removal/tests-01.log',
 'research/skirmish-20261017-configure-kingdom-removal/check-e0-01.log',
 'research/skirmish-20261017-configure-kingdom-removal/independent-builder-review.json',
 'research/skirmish-20261017-configure-kingdom-removal/README.md',
 'research/skirmish-20261017-configure-kingdom-removal/shared-drain-replay.log',
 'research/skirmish-20261017-configure-kingdom-removal/shared-ownership-replay.log',
 'research/skirmish-20261017-configure-kingdom-removal/shared-lifetime-replay.log',
 'research/development10/playable-startup/attempt10.worker.json']
doc=dict(scope='Pre-static queued class4 Kingdom destruction on same restore owners; actual selected deletion/configuration prefix completed, no whole startup/playability claim',
 native=dict(f0_rows=66,f0_executed_pins=165,session_rows=54,session_executed_pins=26,e0_rows=42,e0_executed_pins=728,
  qualification='Counts include both images/all3PC. Laboratory prefix/destructor executions; heap/CString transport and named child cutoffs remain scoped.'),
 rust=dict(focused_passed=2,shared_regressions_passed=7,failed=0,log='tests-01.log',duration_seconds=0.01,
  qualification='Binary captured before later shared ThingCore handle-width widening; source body hashes independently reviewed and unchanged.'),
 actual=dict(input='research/development10/playable-startup/attempt07.removal-owned.json',observed_kingdom=1677,
  status='Actual selected attempt10 configuration_completed=true after Kingdom1677 deletion; DynamicThing1682 completes,1683 stops on now-removed owner1677.',
  completed_prefix='research/development10/playable-startup/attempt10.worker.json'),
 files=[pin(p)for p in files])
(HERE/'findings.json').write_text(json.dumps(doc,indent=2)+'\n')
print(hashlib.sha256((HERE/'findings.json').read_bytes()).hexdigest())
