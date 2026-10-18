from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
paths=[ROOT/'crates/aa-runtime/src/view_texture_effect.rs',ROOT/'crates/aa-runtime/src/view_texture_effect_tests.rs',
 ROOT/'crates/aa-runtime/src/view_environment_projection.rs',ROOT/'crates/aa-runtime/src/view_environment_projection_tests.rs']
paths += [HERE/n for n in ['native_effect.py','native-effect.json','audit_effect.py','effect-pin-audit.json','effect_isolated.rs','isolated-effect-final.exe','effect-build-final.log','effect-tests-final.log','effect-native-final-replay.log','effect-production-check-03.log','EFFECT.md']]
out=dict(schema=1,status='accepted bounded fresh environment114 numeric/context core',
 scope='constructor86B940 projected scalar/context fields then fresh86E8D0→86EED0→866BE0→86BC30→8A2C70→86BA10→870720→4B2830',
 native=dict(paired_rows=234,executions=468,precision=['Single24','Double53','Extended64'],decoded_equal_executed_pins=343,seams=[],constructor_required_seed_prefixes=9,update_required_read_prefixes=52),
 rust=dict(exact_production_module_combined_tests=4,passed=4,production_check='PASS20.98s, effect-production-check-03.log'),
 retained_failures=['effect-tests-01.log: native class/vtable read positions omitted from initial read trace; corrected by typed dispatch reads without changing fixture outputs',
 'effect-production-check-01.log: external Directory target argument order during source edit (author subsequently fixed)',
 'effect-production-check-02.log: external resource_texture module declaration during source edit (author subsequently saved)'],
 ownership='ONE Core owns local/world/projection/plane/D0 plus constructor-null link context. Outer State owns actual Resource texture154/Texture110; Scene118 remains separate.',
 limits=['No positive controller/property/parent/bound fallback; future actual writers require real typed owners/services',
 'No raw pointer/vtable/refcount/C8/B32450 identity or B45F14 native ABI accounting claim',
 'No AD/view/application activation or full environment source-load completion claim',
 'Source matrix/unit-X table constants pinned in both images; current point globals supplied through actual retained Globals loan; no perpetual mutable-global immutability claim',
 'Finite successful arithmetic, round-nearest masked x87 PC explicit; native traps/invalid pointers/Inf/NaN and Rust panic/allocation-failure parity excluded'],
 pins={str(p.relative_to(ROOT)).replace('\\','/'):H(p)for p in paths})
p=HERE/'effect-findings.json';p.write_text(json.dumps(out,indent=1)+'\n');print(H(p))
