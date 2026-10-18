"""Seal the bounded constituent checkpoint; refuses to overwrite a seal."""
from pathlib import Path
import json,hashlib,shutil
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
snapshot=HERE/'source-v1';snapshot.mkdir()
sources=[
 'crates/aa-runtime/src/map_actor_creation.rs',
 'crates/aa-runtime/src/map_component_init_create.rs',
 'crates/aa-runtime/src/actor_world.rs',
 'crates/aa-runtime/src/configure_world_nations.rs',
 'crates/aa-actors/src/leaf_init.rs',
 'crates/aa-runtime/tests/map_actor_creation.rs',
]
pins=[]
for name in sources:
 p=ROOT/name;copy=snapshot/(name.replace('/','__'))
 shutil.copyfile(p,copy);pins.append(dict(path=name,sha256=sha(p),snapshot=str(copy.relative_to(ROOT)).replace('\\','/')))
dependencies=[
 'research/phase8-ai/native_machine.py',
 'research/phase8-actor-creation/actor_machine.py',
 'research/phase8-actor-creation/native_actor_creation.py',
 'research/phase8-conversion/native_machine.py',
 'research/phase7-actors/native-actor-states.json',
 'research/phase5-actors/direction-tables.bin',
]
for p in sorted(HERE.glob('native*.py'))+sorted(HERE.glob('native*.json'))+[HERE/'actual-constructor-inputs.json',HERE/'README.txt',Path(__file__)]+[ROOT/p for p in dependencies]:
 pins.append(dict(path=str(p.relative_to(ROOT)).replace('\\','/'),sha256=sha(p)))
binary=ROOT/'target/s-owner-recruitment/release/deps/map_actor_creation-b275c3e451b3d8f8.exe'
pins.append(dict(path=str(binary.relative_to(ROOT)).replace('\\','/'),sha256=sha(binary)))
report=dict(schema=1,scope='bounded constructors/source contributor/requirements/first-layout-slot only; full live creation bindings compiled but not accepted',
 tests=dict(command='map_actor_creation-b275c3e451b3d8f8.exe --include-ignored --nocapture',passed=3,failed=0,log='research/skirmish-20261017-map-actor-creation/author-final-replay-02.log'),
 native=dict(rows=261,executions=522,original_v14_separately=True,precisions=['Single24','Double53','Extended64']),pins=pins)
with (HERE/'freeze.json').open('x')as f:json.dump(report,f,indent=2);f.write('\n')
print(json.dumps(dict(freeze_sha256=sha(HERE/'freeze.json'),binary_sha256=sha(binary),pins=len(pins))))
