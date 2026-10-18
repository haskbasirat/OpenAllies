from pathlib import Path
import hashlib,json,re
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def pin(path):
 p=ROOT/path
 return {'path':path,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
sources=['research/phase4-orders/crt_order_initializers.py','research/phase4-orders/recover_orders.py','research/phase4-orders/gen_registry.py',
 'research/phase8-orders/native_order_traces.py','research/phase8-orders/native_order_flow.py','research/phase8-orders/gen_schemas.py',
 'research/phase8-orders/gen_flow_table.py','research/phase8-orders/gen_inventory.py','crates/aa-orders/src/native_registry.rs',
 'crates/aa-orders/src/native.rs','crates/aa-orders/src/flow.rs','crates/aa-orders/src/flow_table.rs',
 'crates/aa-orders/src/commands.rs','crates/aa-orders/src/payload/schemas.rs','crates/aa-orders/src/payload/mod.rs',
 'crates/aa-orders/src/payload/complex.rs','crates/aa-orders/src/payload/game.rs','crates/aa-orders/src/payload/lobby.rs',
 'crates/aa-orders/src/payload/editor.rs','crates/aa-orders/tests/registry_correction.rs','crates/aa-netcode/src/local_version_ack.rs']
fixtures=['research/phase4-orders/orders-native-oracle.json','research/phase4-orders/registry-exploration.json',
 'research/phase8-orders/order-traces.json','research/phase8-orders/order-flow.json','research/phase8-orders/static-handler-calls.json',
 'research/phase8-orders/descriptor-write-sites.json','research/phase8-orders/inventory.json']
for name in ['native_registry_audit.py','native-registry-audit.json','native_version_ack_header.py','native-version-ack-header.json',
 'regenerate_traces.py','synthetic-definition-managers.json','rekey_emitter_inventory.py','historical-357/manifest.json']:
 fixtures.append(str((HERE/name).relative_to(ROOT)).replace('\\','/'))
log=(HERE/'orders-tests.log').read_text()
counts=[tuple(map(int,m))for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',log)]
assert sum(c[0]for c in counts)==42 and not any(c[1]or c[2]for c in counts)
assert 'Finished' in (HERE/'netcode-source-check-final.log').read_text()
old=HERE/'historical-357/crates/aa-orders/src/payload'
def bodies(path):
 t=path.read_text();a=t.split('Some(match descriptor {',1)[1].split('_ => return None',1)[0]
 maps={int(i):name for ids,name in re.findall(r'([\d |]+)=> (\w+),',a)for i in ids.split('|')}
 defs={n:re.sub(r'\s+','',b)for n,b in re.findall(r'pub\(super\) fn (\w+)\([^\n]*\) -> Result<\(\)> \{(.*?)\n\}',t,re.S)}
 return {i:defs[n]for i,n in maps.items()}
comparisons=0
for name in ['game','lobby','editor']:
 before=bodies(old/f'{name}.rs');after=bodies(ROOT/f'crates/aa-orders/src/payload/{name}.rs')
 for i,body in before.items():
  assert after[i+(i>=323)]==body,(name,i)
  comparisons+=1
report={'schema':1,'status':'author-frozen correction; historical reports are not broadened',
 'defect':'Incomplete Ghidra calls.tsv membership caused actual CRT9E65F0 to be incorrectly skipped as a non-order callback.',
 'native':{'crt_unique_initializers_audited_per_image':3634,'separately_executed_images':['original','v14'],'registrations':358,
           'missing_export_initializer':'0x9e65f0','new_descriptor':{'id':323,'name':'OrderVersionAckRequest','handler':'0x766cf0','allocation':'0xb40af0'},
           'unchanged_old_ids':[0,322],'shifted_old_ids':[323,356],'shift':1,'width_bits':9,
           'ack_header_machines':6,'ack_header_cases':324,'ack_header_executed_pc_pins_including_crt':32891,
           'fresh_payload_runs':2864,'decoded_payload_runs':2851,'first_native_null_cutoffs':13,
           'template_flow_returned_cases':1242,'template_flow_native_null_cases':6,'template_shapes':357},
 'validation':{'aa_orders_tests':42,'failed':0,'ignored':0,'build_seconds':17.39,
               'normalized_existing_generated_schema_bodies_unchanged':comparisons,
               'current_netcode_lib_check':'passed; no runtime/client unit link',
               'new_netcode_owner_tests':'saved2; awaiting root warm cohort, not counted as passes',
               'immutable_registry_and_ack_replays':'passed'},
 'scope':['Real CRT registration supplies every corrected descriptor ordinal/count; no generated Rust ID inventory is used to discover membership.',
          'Direct766D40 emitter uses same retained NetNode mutex and Admin request stream; no active gate, SetPlayer prefix, payload or sync.',
          'ID323 has no generic request/decree Shape. Its actual receipt751FC0 is the next separate scope.',
          'Full old mixed corpus preserved. New bounded8-small-seed corpus stops at first real native null access; no old expected outputs weakened.',
          'Existing static emitter addresses rekeyed only by native descriptor allocation identity; new766D40 has its own native execution proof.',
          'No whole CRT, arbitrary indirect registration, full network transport, game startup, native save/profile/film modification or activation claim.'],
 'source_pins':[pin(p)for p in sources],'fixture_and_producer_pins':[pin(p)for p in fixtures],
 'logs':[pin(str((HERE/n).relative_to(ROOT)).replace('\\','/'))for n in ['orders-tests.log','netcode-source-check.log','netcode-source-check-final.log','registry-replay.log','ack-replay.log','native-flow.log','native-traces-bounded.log']]}
(HERE/'findings.json').write_text(json.dumps(report,indent=2)+'\n')
print('findings',hashlib.sha256((HERE/'findings.json').read_bytes()).hexdigest())
