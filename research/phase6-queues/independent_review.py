"""Replay immutable queue evidence; review owns only this script/report."""
from pathlib import Path
import hashlib
import json
import native_queues as native

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
fixture_path=HERE/'native-queues.json'
before=fixture_path.read_bytes()
fixture=json.loads(before)
for case in fixture['cases']:
    machine=native.Machine(case)
    for operation in case['operations']:
        machine.action(operation)
    assert machine.snapshot()==case['expected'],case['name']
    assert machine.events==case['events'],case['name']
    assert machine.read(machine.world+0xd4)==case['rng_state'],case['name']
base=fixture['cases'][0]
for period in fixture['periods']:
    machine=native.Machine({**base,'precision':period['precision'],
                           'clock':{**base['clock'],'step_bits':period['step_bits']}})
    result=machine.call(0x4f0720,machine.world,[period['period_bits']])
    assert result==(period['ticks']&0xffffffff),period
assert fixture_path.read_bytes()==before
report={'schema':1,'reviewer':'visual_audit','passed':True,
        'native_replayed_sequences':len(fixture['cases']),'native_replayed_periods':len(fixture['periods']),
        'source_review':[
            'Periodic dispatch0x50b330 stores bucket_count*world_step to f32 once and reuses that elapsed value for every callback in the bucket.',
            'Cancel-current advances shared world cursor before dispatcher advances it again, matching native skipped successor behavior.',
            'Deferred dispatch unlinks both lists before callback and reloads current head; callback additions are visited in the same bucket.',
            'Every1000th deferred event is removed but not delivered, matching unchanged0x50b020.',
            'Deferred growth preserves current slot and relative offsets while relocating wrapped heads beyond the old end, matching0x50b710.',
            'period_ticks explicitly rounds the divided value to binary64 before ceiling, matching the native spill boundary.',
        ],
        'nonblocking_note':'Resolved: cancel removes an owner map entry only after its retained event vector becomes empty. Source review confirms no changes to list unlinking, cursor advancement, callbacks, has() results or cancel_owner() termination; this releases empty identities and their vector allocations.',
        'limits':['Original actor callbacks, allocations and logger remain explicit oracle seams; no actor behavior or full simulation tick claim.',
                  'Nonfinite arithmetic and nested queue dispatch are explicitly outside the supported Rust contract.'],
        'pins':{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()for p in [fixture_path,HERE/'native_queues.py',ROOT/'crates/aa-world/src/queues.rs',ROOT/'crates/aa-world/tests/queues.rs']}}
(HERE/'independent-review.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(json.dumps({'passed':True,'sequences':len(fixture['cases']),'periods':len(fixture['periods'])}))
