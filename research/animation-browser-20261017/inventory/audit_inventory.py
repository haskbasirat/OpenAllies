"""Read-only original animation inventory/cache audit; writes this research folder only.

No exporter is imported (some exporters create directories at import time).
Existing parsed NIF/KF records are accepted only after their exact source bytes
match both installed Data.rwd members and extracted original files.
"""
from pathlib import Path
import collections, datetime, hashlib, json, struct

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
GAME=ROOT.parent
ORIGINAL=GAME/'reconstruction/assets/original/Data'
CACHE=ROOT/'assets-cache/animation'
sha=lambda b:hashlib.sha256(b).hexdigest()
def read(p):return json.loads(p.read_bytes())
def pin(p):return dict(path=p.relative_to(ROOT).as_posix(),bytes=p.stat().st_size,sha256=sha(p.read_bytes()))
def write(name,obj):
 with (HERE/name).open('x',encoding='utf-8',newline='\n')as f:json.dump(obj,f,indent=2);f.write('\n')
def counts(items):return dict(sorted(collections.Counter(items).items()))
def key(s):return s.replace('\\','/').casefold()
def features(asset):
 reasons=set()
 for c in asset['controllers']:
  if c['class']!='NiKeyframeController':reasons.add('controller:'+c['class'])
  if c['frequency']!=1 or c['phase']!=0:reasons.add('nondefault_frequency_or_phase')
  for field in ('translation','scale','quaternion'):
   q=c.get(field)
   if q and q['keys']and q['kind']!=1 and not(q['kind']==2 and q['constant_with_zero_tangents']):reasons.add(field+'_key_type:'+str(q['kind']))
  if any(q['keys']and not q['constant_with_zero_tangents']for q in c.get('euler_xyz',[])):reasons.add('varying_euler_rotation')
 return sorted(reasons)
def controller_counts(asset):return counts(c['class']for c in asset['controllers'])

inventory_path=ROOT/'research/phase3-animation/source-inventory.json'
inventory=read(inventory_path)
assert not inventory['errors']and len(inventory['assets'])==inventory['source_file_count']==2481
assets={key(a['source']):a for a in inventory['assets']}
assert len(assets)==2481
# Parse only the archive's original uncompressed directory, matching aa-formats.
with (GAME/'Data.rwd').open('rb')as f:
 magic,version,n=struct.unpack('<4sII',f.read(12));assert magic==b'TGCK'and version==2
 f.seek(0,2);archive_size=f.tell();f.seek(archive_size-n*96);chunks={}
 for _ in range(n):
  b=f.read(96);name=b[:64].decode('utf-16-le').rstrip('\0');chunks[name]=struct.unpack_from('<QQQQ',b,64)
 assert all(v[1]==v[3]for v in chunks.values())
 f.seek(chunks['Footer'][0]);footer=f.read(chunks['Footer'][1]);at=4;members={};member_count=struct.unpack_from('<I',footer)[0]
 for _ in range(member_count):
  size=struct.unpack_from('<H',footer,at)[0];at+=2;name=footer[at:at+size*2].decode('utf-16-le').replace('\\','/');at+=size*2
  offset,length,flags,index=struct.unpack_from('<QQII',footer,at);at+=24
  if Path(name).suffix.lower()in('.nif','.kf'):
   assert flags==0 and offset+length<=chunks['Files'][1]
   assert key(name)not in members
   members[key(name)]=dict(source=name,offset=chunks['Files'][0]+offset,bytes=length,flags=flags,source_index=index)
 assert at==len(footer)and members.keys()==assets.keys()
 verified=[]
 for k,m in sorted(members.items()):
  a=assets[k];assert m['bytes']==a['size']
  f.seek(m['offset']);raw=f.read(m['bytes']);assert len(raw)==m['bytes']and sha(raw)==a['sha256']
  p=ORIGINAL/a['source'];assert p.stat().st_size==a['size']and sha(p.read_bytes())==a['sha256']
  verified.append(dict(**m,sha256=a['sha256']))
original_files={key(p.relative_to(ORIGINAL).as_posix())for p in ORIGINAL.rglob('*')if p.is_file()and p.suffix.lower()in('.nif','.kf')}
assert original_files==assets.keys()

rigs=[];by_model=collections.defaultdict(list);clip_sources=set();all_pairs=set();top_pairs=set();top_models=set();top_clips=set()
for p in sorted(CACHE.rglob('*.rig.json')):
 d=read(p);assert d['format']=='aa-original-rig-v1';model=assets[key(d['source'])];assert model['sha256']==d['source_sha256']
 top=p.parent==CACHE;clips=[]
 for c in d['clips']:
  original=assets[key(c['source'])];assert original['sha256']==c['source_sha256']
  clips.append(dict(name=c['name'],source=original['source'],sha256=original['sha256'],duration=c['duration'],tracks=len(c['tracks'])))
  clip_sources.add(key(c['source']));all_pairs.add((key(d['source']),key(c['source'])))
  if top:top_pairs.add((key(d['source']),key(c['source'])));top_clips.add(key(c['source']))
 row=dict(**pin(p),model_source=model['source'],model_sha256=model['sha256'],browser_currently_enumerates=top,clips=clips)
 rigs.append(row);by_model[key(d['source'])].append(row['path'])
 if top:top_models.add(key(d['source']))
catalog=[]
for k,a in sorted(assets.items()):
 embedded=a['source'].lower().endswith('.nif')and bool(a['controllers'])
 catalog.append(dict(source=a['source'],sha256=a['sha256'],bytes=a['size'],archive_member=members[k],nif_version=a['version'],kind=Path(a['source']).suffix[1:].lower(),embedded_controllers=embedded,controllers=a['controllers'],sequence_roots=a['sequences'],node_count=a['node_count'],duplicate_node_names=a['duplicate_node_names'],current_exporter_gaps=features(a),cached_model_rigs=by_model.get(k,[]),is_used_as_cached_clip=k in clip_sources,browser_lists_model=k in top_models))
deaths=[x for x in catalog if x['source'].startswith('Buildings/')and'death'in Path(x['source']).stem.lower()]
hqs=[x for x in deaths if'hq'in Path(x['source']).stem.lower()]
corps=[x for x in deaths if'corps'in Path(x['source']).stem.lower()]
animated=[a for a in inventory['assets']if a['source'].lower().endswith('.nif')and a['controllers']]
kfs=[a for a in inventory['assets']if a['source'].lower().endswith('.kf')]
families=read(CACHE/'families-manifest.json');game=read(CACHE/'game-rigs.json');legacy=read(CACHE/'manifest.json')
known_inputs=[inventory_path,ROOT/'research/phase3-animation/animation-catalog.json',ROOT/'docs/NATIVE-PORT-RULES.md',ROOT/'crates/aa-client/src/animation_preview.rs',ROOT/'crates/aa-animation/src/lib.rs']+[ROOT/'crates/aa-animation/tools'/x for x in('inventory_phase3.py','build_phase3_catalog.py','export_original.py','export_families.py','export_game_rigs.py')]+[CACHE/x for x in('manifest.json','families-manifest.json','game-rigs.json','promoted-families.json')]
summary=dict(source_files=len(assets),original_nif=len(assets)-len(kfs),original_kf=len(kfs),embedded_animated_nif=len(animated),nif_with_embedded_animation_not_browser_listed=sum(key(a['source'])not in top_models for a in animated),browser_root_rigs=sum(x['browser_currently_enumerates']for x in rigs),browser_model_clip_bindings=sum(len(x['clips'])for x in rigs if x['browser_currently_enumerates']),browser_unique_kf=len(top_clips),all_recursive_rig_files=len(rigs),unique_cached_model_sources=len(by_model),unique_cached_model_kf_pairs=len(all_pairs),unique_cached_kf=len(clip_sources),original_kf_without_any_cached_clip=sum(key(a['source'])not in clip_sources for a in kfs),game_rig_files=len(game['rigs']),game_role_clip_bindings=sum(x['clips']if isinstance(x['clips'],int)else len(x['clips'])for x in game['rigs']),building_death_nif=len(deaths),hq_death_nif=len(hqs),corps_death_nif=len(corps),building_death_nif_listed_in_browser=sum(x['browser_lists_model']for x in deaths),embedded_by_category=counts(a['source'].split('/')[0]for a in animated),kf_by_category=counts(a['source'].split('/')[0]for a in kfs),unsupported_controller_classes=counts(c['class']for a in inventory['assets']for c in a['controllers']if c['class']!='NiKeyframeController'),assets_by_exporter_gap=counts(g for a in inventory['assets']for g in features(a)))
write('original-source-catalog.json',dict(schema=1,scope='Complete installed original Data.rwd NIF/KF source catalog, byte-verified against prior parsed inventory and extracted originals. Inventory is not playback support or runtime selection proof.',archive=dict(path=str(GAME/'Data.rwd'),bytes=archive_size,member_count=member_count,footer_sha256=sha(footer),selected_members=verified),assets=catalog))
write('cache-coverage.json',dict(schema=1,rigs=rigs,selected_family_rejections=families['skipped'],game_skipped_models=game['skipped_models'],game_skipped_clips=game['skipped_clips'],game_unresolved_units=game['unresolved_units']))
write('hq-building-deaths.json',dict(schema=1,scope='Original embedded NIF controller timelines, not fabricated KF clips. Full source paths/hashes and controller/key-type records are authoritative inventory; scheduling/playback remains unproved.',hq_death=hqs,corps_death=corps,all_building_death=deaths))
write('findings.json',dict(schema=1,utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),passed=True,scope='Read-only original archive/extracted-source/cache inventory audit. No crate/assets/source/release edits, exporter invocation, Rust build, native execution or new playback acceptance.',counts=summary,causes=[dict(consumer='crates/aa-client/src/animation_preview.rs:44',finding='available() lists root-level .rig.json only; nested game/ and phase3-staging are absent from the list. manifest.json is used only as a cache-directory sentinel.'),dict(exporter='export_original.py',finding='Explicitly exports five infantry models and Idle/Jog/Attack1/Die1 only.'),dict(exporter='export_families.py',finding='Default model-name regex selects Tank|Battleship|Destroyer|Carrier|Artillery|CarArmored; known selected failures reject entire model family. It only exports external View-associated KF sequences, not embedded NIF timelines.'),dict(exporter='export_game_rigs.py',finding='Selects infantry-marked recovered units and at most six gameplay roles; richer KF families and building NIF timelines are intentionally absent.'),dict(consumer='crates/aa-animation/src/lib.rs',finding='Rig schema only contains NiKeyframeController node transform tracks; validator admits linear channels plus proven constant Euler bind orientation, not arbitrary embedded property/particle/path/visibility schedules.')],recommended_catalog=['Use authoritative Data.rwd member path + source SHA + source-local block/sequence identity as catalog keys, not basename or invented clips. Preserve all581 KF sources and743 embedded-controller NIF sources, including currently unsupported entries with explicit reasons.','Separate asset inventory from model/clip associations: KF target-name lists alone do not prove a compatible model; use source TGI declarations/precise native binding, qualified recovered candidate pairs and exact model/source hashes. Do not silently select a model for default Character animations.','Expose cached game/ rigs through an explicit authoritative cache index, deduplicating staged copies by exact model/clip source identity while retaining distinct original sequence/role names. This alone improves discovery but does not add HQ/death playback.','Add embedded NIF controller timeline support with exact source target/property/geometry identity, authored ranges/flags/frequency/phase, visibility/alpha/texture/particle behavior and curve implementations before marking full animations playable. Unsupported channels must not be omitted while displaying a falsely complete death animation.','Prioritize all20 HQ death and5 Corps death NIFs plus77 building death NIFs and119 Pack/Unpack sources; then generals, ambient animals, scenery/weather, effects/projectiles, cutscenes and other authored controller assets. Preserve complete catalog entries even before all playback is implemented.'],limits=['Existing parsed controller facts are reused only after exact archive and extracted-file byte hashes match. No new NIF parser or native interpolator was executed.','Prior recovered TGI associations are candidate evidence; archived aa-game/aa-net behavior is not a source of truth and must not be revived.','Inventory completeness does not imply animation playback, event/controller scheduling, blending, root motion, battlefield integration or native numeric parity.','No default duration, replacement keyframe, fabricated wheel/motion clip, basename retargeting, or silently dropped unsupported controller is authorized.'],inputs=[pin(p)for p in known_inputs],outputs=[pin(HERE/x)for x in('original-source-catalog.json','cache-coverage.json','hq-building-deaths.json')]))
print(json.dumps(summary,indent=2))
print('findings_sha256',sha((HERE/'findings.json').read_bytes()))
