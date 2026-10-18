from pathlib import Path
import hashlib,json
BASE=Path(__file__).resolve().parent;ROOT=BASE.parents[1]
def rel(p):return p.relative_to(ROOT).as_posix()
def receipt(p):return dict(file=rel(p),sha256=hashlib.sha256(p.read_bytes()).hexdigest())
modules=['film_session','film_header','film_resource','film_text','film_variables','film_paths','film_header_capture','film_creator','film_header_entries']
proofs=[('session',123,140,'instructions'),('header',9,1099,'instructions'),('text',36,1327,'blocks'),('rotation',27,1371,'blocks'),('paths',120,98,'blocks'),('capture',51,34,'blocks'),('creator',24,1147,'instructions'),('entries',84,98,'blocks')]
fixtures=[]
for name,count,pins,basis in proofs:
 p=BASE/f'native-film-{name}.json';data=json.loads(p.read_text())
 assert len(data['cases'])+len(data.get('actual_sources',[]))==count
 assert len(data[basis])==pins
 fixtures.append(dict(**receipt(p),paired_rows=count,separate_image_runs=2*count,pinned_units=pins,pin_basis=basis,all_pc=True))
checks=[]
for name,count in [('session-tests-fixed.log',1),('header-tests.log',1),('resource-tests.log',1),('text-tests-fixed.log',1),('variables-tests-final.log',2),('paths-tests.log',1),('capture-tests.log',1),('entries-creator-tests.log',2)]:
 p=BASE/name;assert 'test result: ok.'in p.read_text(errors='replace')
 checks.append(dict(**receipt(p),passed=count))
tests=sorted((ROOT/'crates/aa-runtime/tests').glob('film*.rs'))+[ROOT/'crates/aa-runtime/src/film_variables_tests.rs']
doc=dict(schema=1,date='2026-10-17',status='bounded components and required-prefix tests passed; complete current film startup remains unimplemented',
 production=[receipt(ROOT/f'crates/aa-runtime/src/{name}.rs')for name in modules],tests=[receipt(p)for p in tests],
 generators=[receipt(p)for p in sorted(BASE.glob('native_film_*.py'))],native_fixtures=fixtures,
 paired_rows_total=sum(x['paired_rows']for x in fixtures),separate_image_runs_total=sum(x['separate_image_runs']for x in fixtures),
 rust_checks=checks,rust_tests_passed_total=sum(x['passed']for x in checks),
 scoped_current_check='latest-final-test-check.log passed --test film_entries_native --test film_creator_native and shared production/aa-netcode',
 integration_dependencies=dict(actual_roster='FilmRoster/PlayersBorrow +same Sim and Node8 opaque allocation',
  actual_recorder='Session1C Resource and Node268 weak RecorderHandle +same mirror stream',
  actual_creator='resource_creator Creator Kind and base_payload +same sourceResource50',actual_ai='load_sai_players Arena.profile_for_kingdom native ascending first-match slots'),
 unimplemented=['fresh5A23F0/5A14C0 header processmetadata/current clock owner','510EB0 complete current parameters, localized pairs and majorKingdom controlglyph4E93C0','4F1310/411790 current CampaignState services','5A3250 work-depot publication and5A00E0 actual filename/open102','5A1120 positive missing map-text-entry creation/current mutable TextData list integration','59FDA0 optional separate MAPSTRINGS resource/file branch','complete7757F0 actual launch/header/factory/output invocation and RFM playback'],
 not_claimed=['full film startup or playable battle','current save/network completion','Windows depot/file-lock lifecycle','general Unicode/font escapes/language merge','identical native TGI formatter or zlib deflater bytes','native allocator failure or C++ SEH equivalence','fresh native run from existing RFM observation'],
 existing_native_rfm_observation=receipt(BASE/'existing-native-film-observation.json'),readme=receipt(BASE/'README.txt'))
p=BASE/'findings.json';p.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(dict(**receipt(p),native_rows=doc['paired_rows_total'],separate_image_runs=doc['separate_image_runs_total'],rust_tests=doc['rust_tests_passed_total'])))
