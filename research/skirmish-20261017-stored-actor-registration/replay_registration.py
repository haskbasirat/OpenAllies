"""Replay the immutable original4D62F0/477FC0 fixture without its writer.
Preserves the old declared constructor/effect seams; does not add a child
success or treat intercepted entry-address pins as executed body evidence.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-load-actor-registration'))
import native_registration as base
fixture=ROOT/'research/skirmish-20261016-load-actor-registration/native-registration.json'
doc=json.loads(fixture.read_text());H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
for row in doc['table_runs']:
 base.h.P=base.P if row['image']=='original' else base.V
 actual=base.Machine(row['precision']).table();assert actual==row['records']
for row in doc['cases']:
 base.h.P=base.P if row['image']=='original' else base.V
 actual=base.Machine(row['precision']).run(row['input']);assert actual==row['output']
base.h.P=base.P
for pin in doc['instructions']:
 a=int(pin['va'],16);raw=bytes.fromhex(pin['bytes'])
 assert base.P.range(a,len(raw))==base.V.range(a,len(raw))==raw
print(json.dumps(dict(scope=__doc__,fixture_sha256=H(fixture),producer_sha256=H(Path(base.__file__)),replay_sha256=H(Path(__file__)),tables=len(doc['table_runs']),rows=len(doc['cases']),native_executions=len(doc['table_runs'])+len(doc['cases']),result='PASS')))
