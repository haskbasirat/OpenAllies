"""Seal the supplemental Team/real-header-field scope, not the base report."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
def pin(name):
    p=ROOT/name
    return {'file':name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
names=[
 'crates/aa-runtime/src/film_parameters.rs',
 'crates/aa-runtime/src/film_header_fields.rs',
 'crates/aa-runtime/tests/film_teams_native.rs',
 'crates/aa-runtime/tests/film_fields_native.rs',
 'research/skirmish-20261017-film/native_film_teams.py',
 'research/skirmish-20261017-film/native-film-teams.json',
 'research/skirmish-20261017-film/native-film-capture.json',
 'research/skirmish-20261017-film/teams-fields-tests-authority.log',
 'research/skirmish-20261017-film/PARAMETERS.txt',
]
report={
 'schema':1,'date':'2026-10-17',
 'status':'Team phase and generic fresh-header coordinator passed bounded native fixture replay',
 'base_report':'findings.json',
 'base_report_sha256':hashlib.sha256((HERE/'findings.json').read_bytes()).hexdigest(),
 'pins':[pin(n) for n in names],
 'native_team_phase':{'paired_rows':36,'separate_original_v14_image_sessions':72,'pc':[24,53,64],'executed_block_pins':193,'stop_before':'511020 Kingdom count reset'},
 'reused_header_capture':{'paired_rows':51,'new_native_executions_counted':0},
 'rust':{'passed':2,'failed':0,'log':'teams-fields-tests-authority.log','profile':'release --locked --offline -j4'},
 'authority':'Private State can only be populated through concrete same-ActorStore/canonical-source Team reader; explicit-input Table cannot be installed into State.',
 'limitations':[
  'No actual positive managed Team capture executed; concrete reader compiled.',
  'Full Kingdom and trigger-extra 510EB0 phases are still required.',
  'Fresh Header factory/current complete capture remains unbound.',
  'No complete film startup, filesystem/depot lock, save or battle claim.'
 ]
}
(HERE/'parameters-findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'report':'parameters-findings.json','sha256':hashlib.sha256((HERE/'parameters-findings.json').read_bytes()).hexdigest(),'production':report['pins'][:2]},indent=2))
