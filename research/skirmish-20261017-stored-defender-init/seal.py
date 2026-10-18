from pathlib import Path
import hashlib,json,shutil
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def pin(path):
 p=ROOT/path
 return dict(path=path,sha256=hashlib.sha256(p.read_bytes()).hexdigest(),bytes=p.stat().st_size)
sources=[
 'crates/aa-actors/src/defender_init.rs','crates/aa-actors/src/defender_init_tests.rs',
 'crates/aa-actors/src/component_virtuals.rs','crates/aa-runtime/src/stored_defender_init_world.rs',
 'crates/aa-runtime/src/map_actor_init_existing.rs','crates/aa-runtime/src/stored_component_init_source.rs',
 'crates/aa-runtime/src/stored_zone_activation.rs','crates/aa-runtime/src/stored_performer_refresh.rs',
 'crates/aa-runtime/src/load_detection_activation.rs',
 'research/skirmish-20261017-stored-defender-init/native_defender.py',
 'research/skirmish-20261017-stored-defender-init/native-defender.json',
 'research/skirmish-20261017-stored-defender-init/replay_defender.rs',
 'research/skirmish-20261017-stored-defender-init/replay-defender.ps1',
 'research/skirmish-20261017-stored-defender-init/README.txt',
]
snap=HERE/'source-frozen';snap.mkdir(exist_ok=True)
for name in sources:
 p=snap/name;p.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(ROOT/name,p)
artifact='target/s-borrowed-leader/defender-tests.exe'
shutil.copyfile(ROOT/artifact,HERE/'frozen-defender-tests.exe')
deps=['target/s-borrowed-leader/release/deps/libaa_actors-da01587d98fcfe3e.rlib',
 'target/s-borrowed-leader/release/deps/libserde_json-3f0b3bb60c709793.rlib']
out=dict(schema=1,status='bounded native+exact coordinator replay; runtime adapter compile only',
 native_images=[dict(role='Loader ORIGINAL',sha256='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'),dict(role='Loader ACCEPTED v14',sha256='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1')],
 addresses=['566DB0','529C30','45D6C0','45D4C0','4D69A0','516640','515940'],
 explicit_native_children=['515F40 zone solver','57E430 Performer'],
 native=dict(rows=1050,executions=2100,pcs=['Single24','Double53','Extended64'],executed_block_pins=48,write='native-write.log',immutable='native-immutable.log'),
 rust=dict(tests_passed=2,log='rust-core-final.log',failure_retained='rust-core01.log: fullEAX instead of Boolean AL expectation'),
 runtime=dict(check='production-check-final.log',actual_adapter_executed=False,actual_mission_ready=False),
 sources=[pin(p)for p in sources],artifact=pin(artifact),dependencies=[pin(p)for p in deps],
 exclusions=['SignallingNaN Unicorn parity','malloc failure','dead-actor diagnostic completion','actual zone solver callback parity','actual57E430 computations','actualger_corps1688/full mission startup'])
(HERE/'findings.json').write_text(json.dumps(out,indent=2)+'\n')
print(hashlib.sha256((HERE/'findings.json').read_bytes()).hexdigest())
