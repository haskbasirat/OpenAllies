from pathlib import Path
import hashlib,json,re
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
fixture=json.loads((HERE/'native-entry.json').read_text())
log=(HERE/'tests.log').read_text()
assert '4 passed; 0 failed' in log
census=re.search(r'actual saved entry census: absent=(\d+), null owner=(\d+), nonnull owner=(\d+)',log)
assert census
source='crates/aa-runtime/src/saved_economy_refresh.rs'
prior=ROOT/'research/phase7-economy-refresh/native-refresh-fixtures.json'
assert sha(prior)==fixture['prior_refresh_oracle_sha256']
result={
 'schema':1,'date':'2026-10-16','scope':'Only saved4CF0F0 absent-Economy and570020 null-owner return; nonnull remains unsupported',
 'api':'aa_runtime::saved_economy_refresh::refresh_member(&mut SavedActions,ActorId)->aa_combat::Result<()> [crate-private]',
 'native_addresses':['4CF0F0..4CF100','570020..570051','5701F6 return','4D1E80..4D1E87'],
 'original_sha256':fixture['original_sha256'],'v14_sha256':fixture['v14_sha256'],
 'native_entry_cases':len(fixture['cases']),'prior_refresh_rows_replayed':3,
 'source_sha256':{source:sha(ROOT/source)},
 'evidence_sha256':{p:sha(HERE/p) for p in ['native_entry.py','native-entry.json','oracle.log','tests.log']},
 'prior_refresh_fixture_sha256':sha(prior),
 'backups_sha256':{p.name:sha(p) for p in (HERE/'backups').iterdir()},
 'tests':{'passed':4,'failed':0,'includes_ignored_real_assets':True},
 'actual_saved_census':dict(zip(['absent_economy','null_owner','nonnull_owner'],map(int,census.groups()))),
 'mutation':'None; success and explicit unsupported leave all component and actor data unchanged',
 'identity_guard':'Header owner mismatch is explicit runtime consistency refusal; not native corrupt-world parity',
 'native_boundary':'Nonnull owner stopped before570051 in new oracle; no resource arithmetic claimed',
 'limits_changed':[],
 'remaining':['Concrete nonnull-owner570020 service','Canonical initialized Kingdom resource ownership','Complete saved-world publication','Live economy/skirmish/multiplayer'],
}
(HERE/'findings.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'source_sha256':result['source_sha256'],'fixture_sha256':result['evidence_sha256']['native-entry.json'],'tests':result['tests'],'census':result['actual_saved_census']},indent=2))
