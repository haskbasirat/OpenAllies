from pathlib import Path
import hashlib,json,sys,struct
ROOT=Path(__file__).resolve().parents[3]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
pin=lambda rel:{'path':rel,'sha256':sha(ROOT/rel)}
audit=json.loads((Path(__file__).with_name('pin-audit.json')).read_text())
for report in audit:
 for item in report['pins']:
  current=sha(ROOT/item['path'])
  assert current==item['actual'],(item['path'],'changed during independent review')
common={'schema':1,'reviewer':'leader_binding_review','accepted_bounded_scope':True,'blocking_findings':[],
 'method':'Read-only source review, independently re-executed frozen exact-source Rust executable and immutable original/v14 Unicorn producers; independent byte/image audit. No Cargo build, game process, source/fixture mutation or whole-client execution.',
 'instruction_pin_qualification':'Observed instruction bytes match both PEs. native_machine records before observer cutoffs; no blanket per-case instruction-completion claim.',
 'pin_audit':'research/skirmish-20261017-texture-load-kernel/independent-leader/pin-audit.json',
 'native_pin_audit':'research/skirmish-20261017-texture-load-kernel/independent-leader/native-pin-verification.json'}
texture=dict(common,scope='5A8CD0/5A8B40/5A6A10 borrowed Texture field/refcount/child order plus native default Source/Pixel metadata recipe',
 rust={'tests_passed':1,'rows':351,'log':'independent-leader/exact-kernel-tests.log'},
 native={'texture_rows':351,'default_rows':36,'fresh_original_v14_executions':774,'precisions':['Single24','Double53','Extended64'],'fixture_equality':'passed','instruction_byte_pins':1245},
 checks=['Opaque Source/Pixel identities stay separate; mandatory child callbacks cannot silently succeed.',
 'Early writable-child, filename, exact no-Ni byte1 and force-low short circuit match native guards.',
 'Width54 store before height read; zero-size normalization, unsigned power-of-two/wrapping accounting, ratio stores and signed warning remain in native order.',
 'Source discard byte precedes normalize; old Source release precedes Resource3C store/new retain; actual partial refcount/stores persist on child refusal.',
 'All351 Rust outputs include events/fields/refs/result/error; every attempted state refuses retry.',
 'Default native recipe executes PixelFormat/Pixel layout/Source list and static-once colors; successful allocator/OS/renderer/destructor seams remain explicit.',
 'B323B0 starts1 in the PE; mode0 fixture does not authorize a live default-policy assumption.'],
 exclusions=['No concrete Source/Pixel arena provenance acceptance, actual decoder/resize pixel equivalence, renderer virtual child success, GPU/Task/readiness, natural VWorld or playable-battle claim.',
 'Later texture_source_renderer successor is outside this review.'],
 reviewed_pins=[pin('research/skirmish-20261017-texture-load-kernel/findings.json'),pin('research/skirmish-20261017-texture-load-kernel/default-source-findings.json'),pin('crates/aa-runtime/src/texture_load_kernel.rs'),pin('crates/aa-runtime/src/texture_load_kernel_tests.rs'),pin('research/skirmish-20261017-texture-load-kernel/exact-kernel-tests.exe')])
factory=dict(common,scope='4F0750/50CAB0 empty-pending/50C220/43D160/43AC20/43C420/4EFBE0 factory traversal and publication order',
 rust={'tests_passed':1,'rows':195,'log':'independent-leader/exact-order-tests.log'},
 native={'rows':195,'fresh_original_v14_executions':390,'precisions':['Single24','Double53','Extended64'],'fixture_equality':'passed','instruction_byte_pins':554},
 checks=['Actual Kingdom slots precede occupied-GTID traversal; count reread after each child is retained.',
 'IdentityRegistry::iter walks ascending occupied slots; adapter retains generation-bearing IDs and validates live Object identity at reached access.',
 'World phase6/null/attached/source gates remain lazy; factory repeats current source getter.',
 'Init success update-list insertion precedes Thing+C publication; false Init invokes C4 destruction before publishing null.',
 'Per-view State retains failure prefix and refuses retry, with no construction/Init/GPU receipt fabricated.'],
 shared_successor={'path':'crates/aa-client/src/view_world_owner.rs','current_sha256':sha(ROOT/'crates/aa-client/src/view_world_owner.rs'),'old_author_pin_matches':False,'qualification':'Author explicitly released shared file for later camera/policy/concrete InitContext changes. Frozen core, test, runtime adapter, producer, fixture and executable pins all match; shared successor not accepted by this kernel review.'},
 exclusions=['Positive derived constructor/Init/destructor/allocator failure, nonempty FX command bodies, concrete client Directory/GPU acceptance, AD/readiness and full render/battle remain outside scope.'],
 reviewed_pins=[pin('research/skirmish-20261017-view-factory/findings.json'),pin('crates/aa-runtime/src/view_factory_order.rs'),pin('crates/aa-runtime/src/view_factory_order_tests.rs'),pin('crates/aa-runtime/src/view_factory.rs'),pin('research/skirmish-20261017-view-factory/exact-order-tests.exe')])
for rel,d in [('research/skirmish-20261017-texture-load-kernel/independent-review.json',texture),('research/skirmish-20261017-view-factory/independent-review.json',factory)]:
 p=ROOT/rel;p.write_text(json.dumps(d,indent=2)+'\n');print(rel,sha(p))
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as n
assert n.original.range(0xb36518,4)==n.modified.range(0xb36518,4)==bytes(4)
visual=dict(common,scope='Visual process-global B36518 logical owner and exact7241CE..724205 dimensions; publication adaptation source review only',
 rust={'independently_rerun':False,'qualification':'No cold Cargo link requested; source checks and native immutable replay only.'},
 native={'rows':63,'fresh_original_v14_executions':126,'pins':105,'immutable_fixture_equality':'passed','raw_PE_B36518_zero':'independently verified in both images'},
 checks=['Fresh null Global is independently justified by actual raw PE DWORD0; simulation World existence does not imply positive visual map.',
 'Dimensions exactly promote stored binary32 extents, apply ceil and signed32 conversion in the bounded finite domain. Nonfinite/out-of-range refuses.',
 'Positive publication requires same actual World, completed sampled height allocation, installed terrain phase and validated concrete TerrainBinding.',
 'Positive lookup checks live World witness and same height allocation; foreign/dead/replaced owners fail.',
 'Existing positive global cannot be overwritten; unimplemented43E400/backend teardown remains explicit.'],
 qualification='GPU terrain-stage publication is a documented modern adaptation, not native721330/723E00 ABI/timing parity. Pure dimension success grants no map publication, AD, readiness or battle capability.',
 exclusions=['No actual GPU terrain publication run, native teardown, full visual map constructor or end-to-end render parity accepted.'],
 reviewed_pins=[pin('crates/aa-runtime/src/view_visual_map.rs'),pin('crates/aa-runtime/src/view_visual_map_tests.rs'),pin('crates/aa-runtime/src/view_heightfield.rs'),pin('crates/aa-runtime/src/view_init_world.rs'),pin('research/development10/visual-map/native_dimensions.py'),pin('research/development10/visual-map/native-dimensions.json'),pin('research/development10/native-render-defaults/visual-process-initial.json')])
p=ROOT/'research/development10/visual-map/independent-leader/independent-review.json';p.write_text(json.dumps(visual,indent=2)+'\n');print(p.relative_to(ROOT),sha(p))
