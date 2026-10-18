"""Seal a bounded source-only forecast beside the actual39 retained graph."""
from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).resolve().parent
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
actual_path=ROOT/'research/development10/playable-startup/attempt39.fresh-actor-prefix.json'
actual=json.loads(actual_path.read_text())
source=json.loads((HERE/'actual39-suffix-source.json').read_text(encoding='utf-8-sig'))
properties=json.loads((HERE/'actual39-initial-properties.json').read_text(encoding='utf-8-sig'))
record=actual['records'][0]
runtime=record['actual_runtime_debug']
assert record['gtid']==1688 and record['object_key']==2362
assert 'id: "ger_corps"' in runtime and 'word_c: 0,' in runtime
assert '344: 0,' in runtime and 'effects: [],' in runtime
assert source['initial_property_count']==11 and not source['mover_component_present']
assert [r['count'] for r in source['fx']]==[0,17,2]
deps=ROOT/'target/s-local-bootstrap/release/deps'
rlibs=[max(deps.glob(pattern),key=lambda p:p.stat().st_mtime_ns)
       for pattern in ['libaa_rules-*.rlib','libaa_numeric-*.rlib','libserde_json-*.rlib']]
pins=[actual_path,HERE/'audit_actual39_suffix_source.rs',HERE/'audit-actual39-suffix-source.exe',
 HERE/'actual39-suffix-source.json',HERE/'audit_actual39_properties.rs',HERE/'audit-actual39-properties.exe',
 HERE/'actual39-initial-properties.json',ROOT/'crates/aa-runtime/src/map_actor_init_existing.rs',
 ROOT/'crates/aa-runtime/src/fresh_post_create_world.rs',ROOT/'crates/aa-runtime/src/stored_creation_fx.rs',
 ROOT/'crates/aa-runtime/src/staged_property_instances.rs',ROOT/'crates/aa-runtime/src/stored_world_registration.rs',
 ROOT/'crates/aa-runtime/src/stored_scene_publication.rs',ROOT/'crates/aa-client/src/view_world_owner.rs',
 ROOT/'crates/aa-client/src/battle_notifications.rs',ROOT/'crates/aa-runtime/tests/playable_battle_startup.rs',
 Path(__file__),*rlibs]
report={
 'scope':'Read-only actual39 retained ger_corps plus independently resolved same V14 source inventory. No synthetic Staged, callback execution or future success.',
 'actual':{'gtid':1688,'key':2362,'world_revision':actual['actual_world_revision'],
  'source_pc':actual['source_precision'],'worker_pc':actual['requested_worker_precision'],
  'inactive_observer_active':actual['actual_inactive_observer_active'],
  'required_object_158':0,'thing_0c':0,'properties_effects_at_refusal':[],
  'original_mutable_params':'not retained; requested_id/word3c and captured request properties count are not asserted'},
 'source':{'definition':'ger_corps','initial_properties_head_first':[
  {'reference':r['reference'],'ids':r['source']['ids']} for r in properties['initial_properties']],
  'global_property_registration_slot0':source['global_registration_slot0']['ids'],
  'mover_component_present':False,'track_infos':0,
  'track_poll_settings':'not read on this definition: native kernel only reads settings for nonempty tracks',
  'needs_settlement_zone':False,
  'constructed_fx':{'count':17,'kind':'CREATE_VFX','chains':0,
   'vfx_ids':[r['vfx_ids']['String']['value'] for r in source['fx'][1]['roots']]},
  'created_fx_count':0,'selected_fx_count':2},
 'next_two_positive_domains_in_native_order':[
  {'domain':'4D755F existing/Constructed42A590',
   'guard':'requires current state low bit21 clear AND original mutable Params.requested_id==word3c; only state is present in snapshot',
   'binding':'stored_creation_fx::publish uses actual sourceBank, current position, creator, lazy worker/application OS identity, World begin time and same presentation queue',
   'possible_named_refusals':['fresh FX on the Application OS thread requires actual VWorld8C clock loan','fresh FX positive Thing0C requires actual current View54/vfBC/45C740 subscription owner','fresh FX requires actual attached View43B870 delivery'],
   'qualification':'Current retained Thing0C=0; no natural graph compile or future queue delivery is certified'},
  {'domain':'4CF300 -> Properties581880 -> initial5817A0/581630',
   'positive_source_entries':11,
   'binding':'fresh_post_create_world::component_post_create forwards same registered property provenance into staged_property_instances::initial; same instance arena and sourceBank retained',
   'possible_named_refusals':['stored initial Property source identity differs','stored Performer requires actual current-thread CRT sampler'],
   'qualification':'Sampler is now bound lazily by real worker. Current ger_corps mask has no Organization or Settlement, so their positive property notifications cannot be reached on this allocation. Other property numeric/source children may still refuse.'}],
 'later':{'registration':'stored_world_registration composes actual4F9510 registration/state/zones over same ActorStore and World; genuine current dispatcher/presentation gates control50C220',
  'named_refusal':'fresh50C220 positive actual client View factory loan is required only if actual phase/attached-view gates demand it',
  'captured_request_property_application':'4FBB32 still requires actual4CEF10(newActor,Property,2,newActor) for each retained request property; original request list is not present, so nonempty count is unknown',
  'created_diagnostic':'current modern transport implemented separately; not naturally reached'},
 'ui_pre_world_ad':{'runner':'actual39 retains AppState once and captures real FIFO; it does not construct ApplicationViews or consume notifications, so it cannot certify AD or Loading transitions',
  'client':'ApplicationViews::from_application_constructor retains the native prefix once before World; current_initialized_for_application reads its actual live AD reader. battle_notifications::State drains SAME queue after ActualActivatedSession and uses that getter for both EndSession notifications.',
  'integration_constraint':'Use this same client owner/bridge on retained SimulationThread. Do not infer false from absence of runner UI or consume queue in a separate diagnostic owner.'},
 'validation':{'source_probe':'Two direct rustc source inventories PASS against pinned existing aa_rules/aa_numeric/serde_json rlibs. No Cargo link or game callback executed.',
  'native':'No new native oracle run: source-only forecast refers to existing native4D7230/4CF300/42A590 kernels and current adapters, not independent parity certification.'},
 'pins':[{'path':p.relative_to(ROOT).as_posix(),'sha256':H(p)} for p in pins]}
path=HERE/'actual39-suffix-audit.json'
with path.open('x') as f:json.dump(report,f,indent=2);f.write('\n')
print(json.dumps({'path':str(path),'sha256':H(path),'initial_properties':11,'constructed_fx':17,'track_infos':0}))
